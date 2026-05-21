use clap::{Parser, Subcommand};
use themis_core::ThemisConfig;
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
