use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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
    /// Windows capture strategy: `auto` | `process` | `endpoint`
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
            insight_dwell_secs: 20,
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
    const DEFAULT: u32 = 20;
    const MIN: u32 = 5;
    const MAX: u32 = 300;
    let Some(s) = raw.filter(|v| !v.is_empty()) else {
        return DEFAULT;
    };
    match s.parse::<u32>() {
        Ok(v) if v >= MIN => v.min(MAX),
        _ => DEFAULT,
    }
}

fn load_dotenv() {
    if dotenvy::dotenv().is_ok() {
        return;
    }
    let Ok(mut dir) = std::env::current_dir() else {
        return;
    };
    for _ in 0..8 {
        let candidate = dir.join(".env");
        if candidate.is_file() {
            let _ = dotenvy::from_path(candidate);
            return;
        }
        if !dir.pop() {
            break;
        }
    }
}

impl ThemisConfig {
    pub fn from_env() -> Self {
        load_dotenv();

        let key = std::env::var("AZURE_SPEECH_KEY")
            .ok()
            .filter(|s| !s.is_empty());
        let region = std::env::var("AZURE_SPEECH_REGION")
            .ok()
            .filter(|s| !s.is_empty());

        let use_mock = std::env::var("THEMIS_USE_MOCK_SPEECH")
            .ok()
            .map(|v| v == "true" || v == "1")
            .unwrap_or_else(|| key.is_none() || region.is_none());

        Self {
            azure_speech_key: key,
            azure_speech_region: region,
            foundry_endpoint: std::env::var("FOUNDRY_ENDPOINT").ok(),
            foundry_api_key: std::env::var("FOUNDRY_API_KEY").ok(),
            foundry_deployment: std::env::var("FOUNDRY_DEPLOYMENT").ok(),
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
        assert_eq!(cfg.insight_dwell_secs, 20);
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
        assert_eq!(parse_insight_dwell_secs(None), 20);
        assert_eq!(parse_insight_dwell_secs(Some("30".into())), 30);
        assert_eq!(parse_insight_dwell_secs(Some("3".into())), 20);
        assert_eq!(parse_insight_dwell_secs(Some("999".into())), 300);
        assert_eq!(parse_insight_dwell_secs(Some("bad".into())), 20);
    }
}
