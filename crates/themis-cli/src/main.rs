use clap::{Parser, Subcommand};
use themis_core::{find_dotenv_directory, ThemisConfig};
use themis_ipc::client::connect;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "themis-cli", about = "Themis command-line tool")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Query service status via gRPC
    Status,
    /// Capture system audio for N seconds and print diagnostics (no Azure)
    #[cfg(any(windows, target_os = "macos"))]
    AudioProbe {
        /// Seconds to listen (default 5)
        #[arg(short, long, default_value = "5")]
        seconds: u64,
    },
    /// Record system audio and run one Azure REST recognition (validates STT)
    #[cfg(windows)]
    SttProbe {
        #[arg(short, long, default_value = "6")]
        seconds: u64,
    },
    /// Health checks (Azure, gRPC)
    Doctor,
    /// Manage background service (Windows)
    #[cfg(windows)]
    Service {
        #[command(subcommand)]
        action: ServiceAction,
    },
    /// Manage LaunchAgent (macOS)
    #[cfg(target_os = "macos")]
    Agent {
        #[command(subcommand)]
        action: AgentAction,
    },
}

#[derive(Subcommand)]
enum ServiceAction {
    Install,
    Start,
    Stop,
    Uninstall,
}

#[derive(Subcommand)]
enum AgentAction {
    Install,
    Start,
    Stop,
    Uninstall,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("info".parse()?))
        .init();

    let cli = Cli::parse();
    let config = ThemisConfig::from_env();

    match cli.command {
        Commands::Status => cmd_status(&config).await?,
        #[cfg(any(windows, target_os = "macos"))]
        Commands::AudioProbe { seconds } => cmd_audio_probe(seconds).await?,
        #[cfg(windows)]
        Commands::SttProbe { seconds } => cmd_stt_probe(&config, seconds).await?,
        Commands::Doctor => cmd_doctor(&config).await?,
        #[cfg(windows)]
        Commands::Service { action } => cmd_service(action)?,
        #[cfg(target_os = "macos")]
        Commands::Agent { action } => cmd_agent(action)?,
    }

    Ok(())
}

async fn cmd_status(config: &ThemisConfig) -> anyhow::Result<()> {
    let mut client = connect(config.grpc_port).await?;
    let resp = client
        .get_status(themis_ipc::GetStatusRequest {})
        .await?
        .into_inner();
    println!("state: {}", resp.state);
    println!("message: {}", resp.message);
    println!("transcripts: {}", resp.transcripts_received);
    println!("capture_mode: {}", resp.capture_mode);
    println!("audio_sessions: {}", resp.audio_sessions);
    println!("audio_peak: {}", resp.audio_peak);
    println!("audio_frames: {}", resp.audio_frames);
    if !resp.capture_detail.is_empty() {
        println!("capture_detail: {}", resp.capture_detail);
    }
    Ok(())
}

#[cfg(any(windows, target_os = "macos"))]
async fn cmd_audio_probe(seconds: u64) -> anyhow::Result<()> {
    use std::sync::Arc;
    use std::time::{Duration, Instant};
    use themis_audio::{create_loopback, SystemAudioOptions};
    use themis_core::CaptureDiagnostics;
    use tokio::sync::mpsc;

    println!("Themis audio probe ({seconds}s)");
    #[cfg(windows)]
    println!("Play sound on your PC (e.g. YouTube). Mute speakers to test process loopback.\n");
    #[cfg(target_os = "macos")]
    println!(
        "Play sound (e.g. YouTube). Using Core Audio process tap (no BlackHole). \
         In call apps (Zoom/Teams), auto mode captures output + mic. \
         Allow System Audio Recording and Microphone if macOS prompts.\n"
    );

    let config = ThemisConfig::from_env();
    let diag = Arc::new(CaptureDiagnostics::new());
    let (tx, mut rx) = mpsc::channel(512);
    let mut source = create_loopback(
        16_000,
        1,
        SystemAudioOptions {
            capture_mode: config.audio_capture_mode.clone(),
            input_device: config.audio_input_device.clone(),
            gain_max: config.audio_gain_max,
            diagnostics: Some(Arc::clone(&diag)),
            ..SystemAudioOptions::default()
        },
    )?;
    source.start(tx)?;

    let deadline = Instant::now() + Duration::from_secs(seconds);
    while Instant::now() < deadline {
        let _ = tokio::time::timeout(Duration::from_millis(100), rx.recv()).await;
    }
    source.stop()?;

    let snap = diag.snapshot();
    println!("--- results ---");
    println!("mode:     {}", snap.mode);
    println!("detail:   {}", snap.detail);
    println!("sessions: {}", snap.sessions);
    println!("frames:   {}", snap.frames);
    println!("peak:     {} (0 = silent, >200 ok, >2000 strong)", snap.peak);

    if snap.frames == 0 {
        println!("\nFAIL: no audio frames captured.");
        println!("  - Is any app playing sound?");
        #[cfg(windows)]
        println!("  - Try THEMIS_AUDIO_CAPTURE_MODE=process or endpoint");
        #[cfg(target_os = "macos")]
        {
            if snap.mode == "process_tap" {
                println!("  - Allow System Audio Recording (see error above if probe failed at start)");
                println!("  - Rebuild: cargo build -p themis-cli && ./scripts/themis.sh probe");
            } else {
                println!("  - System Settings → Sound → Input: pick the correct device");
                println!("  - Or use THEMIS_AUDIO_CAPTURE_MODE=auto for process tap");
            }
        }
        std::process::exit(1);
    } else if snap.peak < 200 {
        println!("\nWARN: signal very weak. Speech recognition may fail.");
        #[cfg(windows)]
        println!("  - Process loopback usually works when endpoint loopback is silent.");
        #[cfg(target_os = "macos")]
        println!("  - Play louder audio; confirm System Audio Recording is allowed for themis-cli");
        std::process::exit(2);
    } else {
        println!("\nOK: capture pipeline is receiving audio.");
    }
    Ok(())
}

#[cfg(windows)]
async fn cmd_stt_probe(config: &ThemisConfig, seconds: u64) -> anyhow::Result<()> {
    use std::sync::Arc;
    use std::time::{Duration, Instant};
    use themis_audio::{create_loopback, SystemAudioOptions};
    use themis_core::{normalize_pcm16, CaptureDiagnostics};
    use tokio::sync::mpsc;

    let (key, region) = match (&config.azure_speech_key, &config.azure_speech_region) {
        (Some(k), Some(r)) => (k.clone(), r.clone()),
        _ => anyhow::bail!("Set AZURE_SPEECH_KEY and AZURE_SPEECH_REGION in .env"),
    };

    println!("STT probe ({seconds}s) — play speech (e.g. YouTube) now.\n");

    let diag = Arc::new(CaptureDiagnostics::new());
    let (tx, mut rx) = mpsc::channel(512);
    let mut source = create_loopback(
        16_000,
        1,
        SystemAudioOptions {
            capture_mode: config.audio_capture_mode.clone(),
            input_device: config.audio_input_device.clone(),
            gain_max: config.audio_gain_max,
            diagnostics: Some(Arc::clone(&diag)),
            ..SystemAudioOptions::default()
        },
    )?;
    source.start(tx)?;

    let mut pcm: Vec<i16> = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(seconds);
    while Instant::now() < deadline {
        if let Ok(Some(frame)) =
            tokio::time::timeout(Duration::from_millis(200), rx.recv()).await
        {
            let mut chunk = frame.to_mono_pcm16(16_000);
            normalize_pcm16(&mut chunk, 12_000, config.audio_gain_max);
            pcm.extend(chunk);
        }
    }
    source.stop()?;

    let snap = diag.snapshot();
    println!("capture: mode={} peak={} samples={}", snap.mode, snap.peak, pcm.len());
    if snap.peak < 200 {
        anyhow::bail!("capture too quiet — fix audio before testing STT");
    }

    use themis_azure::{resolve_speech_languages, AzureMultiLangRestRecognizer, AzureRestRecognizer};
    let langs = resolve_speech_languages(config);
    println!("languages: {}", langs.join(", "));
    println!("calling Azure dictation...");
    let text = if langs.len() > 1 {
        AzureMultiLangRestRecognizer::new(key, region, langs)
            .recognize_pcm(pcm)
            .await?
    } else {
        AzureRestRecognizer::new(
            key,
            region,
            langs.into_iter().next().unwrap_or_else(|| "en-US".into()),
        )
        .recognize_pcm(pcm)
        .await?
    };
    match text {
        Some(t) => {
            println!("\nOK — Azure heard:\n  {t}");
        }
        None => {
            println!("\nWARN — Azure returned no speech in this chunk (try longer / louder audio)");
        }
    }
    Ok(())
}

async fn cmd_doctor(config: &ThemisConfig) -> anyhow::Result<()> {
    println!("Themis doctor");
    println!("  data dir: {}", ThemisConfig::data_dir().display());
    println!("  log dir:  {}", ThemisConfig::log_dir().display());

    if config.use_mock_speech {
        println!("  speech:   mock (no Azure keys)");
    } else if let (Some(key), Some(region)) =
        (&config.azure_speech_key, &config.azure_speech_region)
    {
        print!("  azure:    checking... ");
        match themis_azure::check_connectivity(key, region).await {
            Ok(()) => println!("OK"),
            Err(e) => println!("FAIL ({e})"),
        }
    } else {
        println!("  azure:    not configured");
    }

    if config.llm_configured() {
        let deployment = config
            .foundry_deployment
            .as_deref()
            .unwrap_or("gpt-4o-mini");
        println!("  foundry:  configured (deployment: {deployment})");
        println!("            restart themis-service after editing FOUNDRY_* in .env");
    } else {
        let env_body = find_dotenv_directory()
            .and_then(|dir| std::fs::read_to_string(dir.join(".env")).ok());
        let has_foundry_line = env_body
            .as_ref()
            .is_some_and(|body| body.contains("FOUNDRY_ENDPOINT"));
        let has_placeholder = env_body.as_ref().is_some_and(|body| {
            body.contains("your-resource.openai.azure.com")
                || body.contains("your_openai_key")
        });
        if has_placeholder {
            println!("  foundry:  not configured (.env still has template placeholders — replace FOUNDRY_*)");
        } else if has_foundry_line {
            println!("  foundry:  in .env but not loaded — quote values that contain spaces");
            println!("            (e.g. AZURE_SPEECH_CORRECTIONS=\"Reg:RAG,L L M:LLM\"), then restart");
        } else {
            println!("  foundry:  not configured (set FOUNDRY_ENDPOINT + FOUNDRY_API_KEY)");
        }
    }

    print!("  grpc:     ");
    match connect(config.grpc_port).await {
        Ok(_) => println!("reachable on port {}", config.grpc_port),
        Err(e) => println!("unreachable ({e})"),
    }

    Ok(())
}

#[cfg(windows)]
fn cmd_service(action: ServiceAction) -> anyhow::Result<()> {
    use std::ffi::OsString;
    use std::process::Command;
    use windows_service::service::{
        ServiceAccess, ServiceErrorControl, ServiceInfo, ServiceStartType, ServiceType,
    };
    use windows_service::service_manager::{ServiceManager, ServiceManagerAccess};

    const SERVICE_NAME: &str = "ThemisService";
    const DISPLAY_NAME: &str = "Themis Audio Capture Service";

    let exe = std::env::current_exe()?
        .parent()
        .map(|p| p.join("themis-service.exe"))
        .filter(|p| p.exists())
        .unwrap_or_else(|| std::path::PathBuf::from("themis-service.exe"));

    match action {
        ServiceAction::Install => {
            let manager =
                ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CREATE_SERVICE)?;
            let service_info = ServiceInfo {
                name: OsString::from(SERVICE_NAME),
                display_name: OsString::from(DISPLAY_NAME),
                service_type: ServiceType::OWN_PROCESS,
                start_type: ServiceStartType::AutoStart,
                error_control: ServiceErrorControl::Normal,
                executable_path: exe,
                launch_arguments: vec![],
                dependencies: vec![],
                account_name: None,
                account_password: None,
            };
            manager.create_service(&service_info, ServiceAccess::empty())?;
            println!("Service '{SERVICE_NAME}' installed.");
        }
        ServiceAction::Start => {
            Command::new("sc").args(["start", SERVICE_NAME]).status()?;
        }
        ServiceAction::Stop => {
            Command::new("sc").args(["stop", SERVICE_NAME]).status()?;
        }
        ServiceAction::Uninstall => {
            let _ = Command::new("sc").args(["stop", SERVICE_NAME]).status();
            Command::new("sc").args(["delete", SERVICE_NAME]).status()?;
            println!("Service '{SERVICE_NAME}' removed.");
        }
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn cmd_agent(action: AgentAction) -> anyhow::Result<()> {
    use std::fs;
    use std::path::PathBuf;
    use std::process::Command;

    let home = std::env::var("HOME")?;
    let plist_dir = PathBuf::from(&home).join("Library/LaunchAgents");
    let plist_path = plist_dir.join("com.themis.agent.plist");

    let exe = std::env::current_exe()?
        .parent()
        .map(|p| p.join("themis-service"))
        .filter(|p| p.exists())
        .unwrap_or_else(|| std::path::PathBuf::from("themis-service"));

    match action {
        AgentAction::Install => {
            fs::create_dir_all(&plist_dir)?;
            let plist = include_str!("../../../packaging/macos/com.themis.agent.plist");
            let plist = plist.replace("THEMIS_SERVICE_PATH", &exe.display().to_string());
            fs::write(&plist_path, plist)?;
            println!("LaunchAgent installed at {}", plist_path.display());
        }
        AgentAction::Start => {
            Command::new("launchctl")
                .args(["load", "-w", &plist_path.display().to_string()])
                .status()?;
        }
        AgentAction::Stop => {
            Command::new("launchctl")
                .args(["unload", "-w", &plist_path.display().to_string()])
                .status()?;
        }
        AgentAction::Uninstall => {
            let _ = Command::new("launchctl")
                .args(["unload", "-w", &plist_path.display().to_string()])
                .status();
            let _ = fs::remove_file(&plist_path);
            println!("LaunchAgent removed.");
        }
    }
    Ok(())
}
