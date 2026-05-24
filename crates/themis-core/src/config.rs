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
    /// Max gain applied when loopback signal is quiet (default 16).
    pub audio_gain_max: f32,
    /// Windows capture strategy: `auto` | `process` | `endpoint`
    pub audio_capture_mode: String,
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
            audio_gain_max: 16.0,
            audio_capture_mode: "auto".into(),
        }
    }
}

impl ThemisConfig {
    pub fn from_env() -> Self {
        let _ = dotenvy::dotenv();

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
            audio_gain_max: std::env::var("THEMIS_AUDIO_GAIN_MAX")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(16.0),
            audio_capture_mode: std::env::var("THEMIS_AUDIO_CAPTURE_MODE")
                .unwrap_or_else(|_| "auto".into())
                .to_lowercase(),
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
    }
}
