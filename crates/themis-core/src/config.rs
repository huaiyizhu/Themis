use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Non-secret view of STT / LLM configuration (for tray ↔ service cross-check).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConfigStatusSnapshot {
    pub stt_configured: bool,
    /// `azure` or `mock`
    pub stt_mode: String,
    pub llm_configured: bool,
    pub speech_region: String,
    pub foundry_deployment: String,
    pub analysis_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemisConfig {
    pub azure_speech_key: Option<String>,
    pub azure_speech_region: Option<String>,
    pub foundry_endpoint: Option<String>,
    pub foundry_api_key: Option<String>,
    pub foundry_deployment: Option<String>,
    pub grpc_port: u16,
    pub log_level: String,
    pub use_mock_speech: bool,
    /// Azure Speech: `auto` (en+zh), `en-US`, `zh-CN`, or comma-separated list
    pub speech_language: String,
    pub sample_rate: u32,
    pub channels: u16,
    /// Optional Windows playback endpoint (friendly name substring or device ID).
    /// Default: Windows default **audio output** (render endpoint), not the microphone.
    pub audio_output_device: Option<String>,
    /// macOS: optional input device name substring (e.g. `BlackHole`). Default: system default input.
    pub audio_input_device: Option<String>,
    /// Max gain applied when loopback signal is quiet (default 16).
    pub audio_gain_max: f32,
    /// Windows capture strategy: `auto` | `process` | `endpoint` | `call` | `dual`
    /// macOS: `auto` | `process_tap` | `input` | `call` | `dual` — `auto` uses dual capture when a call app is detected.
    pub audio_capture_mode: String,
    /// Enable transcript insight extraction (keywords, terms, Q&A).
    pub analysis_enabled: bool,
    /// How long Questions/Terms cards stay visible before expiring (seconds).
    pub insight_dwell_secs: u32,
    /// Full-session summary refresh interval (seconds).
    pub session_summary_interval_secs: u32,
}

impl Default for ThemisConfig {
    fn default() -> Self {
        Self {
            azure_speech_key: None,
            azure_speech_region: None,
            foundry_endpoint: None,
            foundry_api_key: None,
            foundry_deployment: None,
            grpc_port: 50051,
            log_level: "info".into(),
            use_mock_speech: false,
            speech_language: "auto".into(),
            sample_rate: 16_000,
            channels: 1,
            audio_output_device: None,
            audio_input_device: None,
            audio_gain_max: 16.0,
            audio_capture_mode: "auto".into(),
            analysis_enabled: true,
            insight_dwell_secs: 600,
            session_summary_interval_secs: 20,
        }
    }
}

fn parse_session_summary_interval_secs(raw: Option<String>) -> u32 {
    const DEFAULT: u32 = 20;
    const MIN: u32 = 10;
    const MAX: u32 = 120;
    let Some(s) = raw.filter(|v| !v.is_empty()) else {
        return DEFAULT;
    };
    match s.parse::<u32>() {
        Ok(v) if v >= MIN => v.min(MAX),
        _ => DEFAULT,
    }
}

fn parse_insight_dwell_secs(raw: Option<String>) -> u32 {
    const DEFAULT: u32 = 600;
    const MIN: u32 = 5;
    const MAX: u32 = 3600;
    let Some(s) = raw.filter(|v| !v.is_empty()) else {
        return DEFAULT;
    };
    match s.parse::<u32>() {
        Ok(v) if v >= MIN => v.min(MAX),
        _ => DEFAULT,
    }
}

fn non_empty_env(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .filter(|s| !s.trim().is_empty() && !is_env_placeholder(s))
}

/// True when a value is empty or still the `.env.example` template (not a real credential).
pub fn is_env_placeholder(value: &str) -> bool {
    let s = value.trim();
    if s.is_empty() {
        return true;
    }
    let lower = s.to_ascii_lowercase();
    const MARKERS: &[&str] = &[
        "your_speech_key",
        "your_speech_key_here",
        "your_openai_key",
        "your-openai-key",
        "your_api_key",
        "your_key_here",
        "changeme",
        "replace_me",
        "xxx.openai.azure.com",
        "your-resource.openai.azure.com",
        "your_resource.openai.azure.com",
    ];
    if MARKERS.iter().any(|m| lower.contains(m)) {
        return true;
    }
    if lower.starts_with("your_") || lower.starts_with("your-") {
        return true;
    }
    false
}

/// Directory containing `.env`, searched from cwd then the running executable path.
pub fn find_dotenv_directory() -> Option<PathBuf> {
    let mut search_roots = Vec::new();
    if let Ok(cwd) = std::env::current_dir() {
        search_roots.push(cwd);
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            search_roots.push(parent.to_path_buf());
        }
    }
    for mut dir in search_roots {
        for _ in 0..8 {
            if dir.join(".env").is_file() {
                return Some(dir);
            }
            if !dir.pop() {
                break;
            }
        }
    }
    None
}

fn load_dotenv() {
    if dotenvy::dotenv().is_ok() {
        return;
    }
    if let Some(dir) = find_dotenv_directory() {
        let _ = dotenvy::from_path(dir.join(".env"));
    }
}

/// Reload `.env` into the process environment, overriding existing keys.
pub fn reload_dotenv_override() {
    if dotenvy::dotenv_override().is_ok() {
        return;
    }
    if let Some(dir) = find_dotenv_directory() {
        let _ = dotenvy::from_path_override(dir.join(".env"));
    } else {
        let path = crate::env_file::env_file_path_or_default();
        if path.is_file() {
            let _ = dotenvy::from_path_override(&path);
        }
    }
}

impl ThemisConfig {
    pub fn from_env() -> Self {
        load_dotenv();
        Self::from_process_env()
    }

    /// Re-read `.env` from disk and rebuild config (overrides in-process env vars).
    pub fn reload_from_disk() -> Self {
        reload_dotenv_override();
        Self::from_process_env()
    }

    fn from_process_env() -> Self {
        let key = std::env::var("AZURE_SPEECH_KEY")
            .ok()
            .filter(|s| !is_env_placeholder(s));
        let region = std::env::var("AZURE_SPEECH_REGION")
            .ok()
            .filter(|s| !s.trim().is_empty());

        let use_mock = std::env::var("THEMIS_USE_MOCK_SPEECH")
            .ok()
            .map(|v| v == "true" || v == "1")
            .unwrap_or_else(|| key.is_none() || region.is_none());

        Self {
            azure_speech_key: key,
            azure_speech_region: region,
            foundry_endpoint: non_empty_env("FOUNDRY_ENDPOINT"),
            foundry_api_key: non_empty_env("FOUNDRY_API_KEY"),
            foundry_deployment: non_empty_env("FOUNDRY_DEPLOYMENT"),
            grpc_port: std::env::var("THEMIS_GRPC_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(50051),
            log_level: std::env::var("THEMIS_LOG_LEVEL").unwrap_or_else(|_| "info".into()),
            use_mock_speech: use_mock,
            speech_language: std::env::var("AZURE_SPEECH_LANGUAGE")
                .unwrap_or_else(|_| "auto".into()),
            sample_rate: 16_000,
            channels: 1,
            audio_output_device: std::env::var("THEMIS_AUDIO_OUTPUT_DEVICE")
                .ok()
                .filter(|s| !s.is_empty()),
            audio_input_device: std::env::var("THEMIS_AUDIO_INPUT_DEVICE")
                .ok()
                .filter(|s| !s.is_empty()),
            audio_gain_max: std::env::var("THEMIS_AUDIO_GAIN_MAX")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(16.0),
            audio_capture_mode: std::env::var("THEMIS_AUDIO_CAPTURE_MODE")
                .unwrap_or_else(|_| "auto".into())
                .to_lowercase(),
            analysis_enabled: std::env::var("THEMIS_ANALYSIS_ENABLED")
                .ok()
                .map(|v| v != "0" && !v.eq_ignore_ascii_case("false"))
                .unwrap_or(true),
            insight_dwell_secs: parse_insight_dwell_secs(
                std::env::var("THEMIS_INSIGHT_DWELL_SECS").ok(),
            ),
            session_summary_interval_secs: parse_session_summary_interval_secs(
                std::env::var("THEMIS_SESSION_SUMMARY_INTERVAL_SECS").ok(),
            ),
        }
    }

    /// True when Azure OpenAI (`FOUNDRY_ENDPOINT` + `FOUNDRY_API_KEY`) are real values, not `.env.example` templates.
    pub fn llm_configured(&self) -> bool {
        self.foundry_endpoint
            .as_ref()
            .is_some_and(|s| !is_env_placeholder(s))
            && self
                .foundry_api_key
                .as_ref()
                .is_some_and(|s| !is_env_placeholder(s))
    }

    /// Non-secret snapshot for UI / gRPC (STT + LLM config cross-check).
    pub fn config_snapshot(&self) -> ConfigStatusSnapshot {
        ConfigStatusSnapshot {
            stt_configured: !self.use_mock_speech,
            stt_mode: if self.use_mock_speech {
                "mock".into()
            } else {
                "azure".into()
            },
            llm_configured: self.llm_configured(),
            speech_region: self.azure_speech_region.clone().unwrap_or_default(),
            foundry_deployment: if self.llm_configured() {
                self.foundry_deployment
                    .clone()
                    .unwrap_or_else(|| "gpt-4o-mini".into())
            } else {
                String::new()
            },
            analysis_enabled: self.analysis_enabled,
        }
    }

    pub fn log_dir() -> PathBuf {
        if cfg!(target_os = "windows") {
            std::env::var("LOCALAPPDATA")
                .map(|p| PathBuf::from(p).join("Themis").join("logs"))
                .unwrap_or_else(|_| PathBuf::from(".themis/logs"))
        } else if cfg!(target_os = "macos") {
            dirs_home().join("Library/Logs/Themis")
        } else {
            dirs_home().join(".local/share/themis/logs")
        }
    }

    pub fn data_dir() -> PathBuf {
        if cfg!(target_os = "windows") {
            std::env::var("LOCALAPPDATA")
                .map(|p| PathBuf::from(p).join("Themis"))
                .unwrap_or_else(|_| PathBuf::from(".themis"))
        } else if cfg!(target_os = "macos") {
            dirs_home().join("Library/Application Support/Themis")
        } else {
            dirs_home().join(".local/share/themis")
        }
    }
}

fn dirs_home() -> PathBuf {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_expected_sample_rate() {
        let cfg = ThemisConfig::default();
        assert_eq!(cfg.sample_rate, 16_000);
        assert_eq!(cfg.grpc_port, 50051);
        assert_eq!(cfg.insight_dwell_secs, 600);
        assert_eq!(cfg.session_summary_interval_secs, 20);
    }

    #[test]
    fn parse_session_summary_interval_secs_clamps_and_defaults() {
        assert_eq!(parse_session_summary_interval_secs(None), 20);
        assert_eq!(parse_session_summary_interval_secs(Some("10".into())), 10);
        assert_eq!(parse_session_summary_interval_secs(Some("5".into())), 20);
        assert_eq!(parse_session_summary_interval_secs(Some("200".into())), 120);
    }

    #[test]
    fn parse_insight_dwell_secs_clamps_and_defaults() {
        assert_eq!(parse_insight_dwell_secs(None), 600);
        assert_eq!(parse_insight_dwell_secs(Some("30".into())), 30);
        assert_eq!(parse_insight_dwell_secs(Some("3".into())), 600);
        assert_eq!(parse_insight_dwell_secs(Some("999".into())), 999);
        assert_eq!(parse_insight_dwell_secs(Some("5000".into())), 3600);
        assert_eq!(parse_insight_dwell_secs(Some("bad".into())), 600);
    }

    #[test]
    fn llm_not_configured_for_env_example_placeholders() {
        assert!(is_env_placeholder("your_openai_key"));
        assert!(is_env_placeholder("https://your-resource.openai.azure.com"));
        assert!(!is_env_placeholder("eastus"));
        let cfg = ThemisConfig {
            foundry_endpoint: Some("https://your-resource.openai.azure.com".into()),
            foundry_api_key: Some("your_openai_key".into()),
            foundry_deployment: Some("gpt-4o-mini".into()),
            ..ThemisConfig::default()
        };
        assert!(!cfg.llm_configured());
        let snap = cfg.config_snapshot();
        assert!(!snap.llm_configured);
        assert!(snap.foundry_deployment.is_empty());
    }

    #[test]
    fn dotenv_loads_foundry_after_quoted_corrections() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap();
        std::env::set_current_dir(root).unwrap();
        for k in [
            "AZURE_SPEECH_KEY",
            "FOUNDRY_ENDPOINT",
            "FOUNDRY_API_KEY",
            "FOUNDRY_DEPLOYMENT",
        ] {
            unsafe { std::env::remove_var(k) };
        }
        load_dotenv();
        let cfg = ThemisConfig::from_env();
        assert!(
            cfg.llm_configured(),
            "FOUNDRY_* must parse from .env (quote AZURE_SPEECH_CORRECTIONS if it contains spaces)"
        );
    }
}
