use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::config::find_dotenv_directory;

/// Editable `.env` fields exposed in the tray settings UI.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct EnvSettings {
    pub azure_speech_key: String,
    pub azure_speech_region: String,
    pub azure_speech_language: String,
    pub azure_speech_mode: String,
    pub themis_stt_fixup: String,
    pub azure_speech_corrections: String,
    pub foundry_endpoint: String,
    pub foundry_api_key: String,
    pub foundry_deployment: String,
    pub themis_analysis_enabled: String,
    pub themis_insight_dwell_secs: String,
    pub themis_session_summary_interval_secs: String,
    pub themis_audio_capture_mode: String,
    pub themis_audio_output_device: String,
    pub themis_audio_input_device: String,
    pub themis_audio_gain_max: String,
    pub themis_grpc_port: String,
    pub themis_log_level: String,
    pub themis_use_mock_speech: String,
}

impl EnvSettings {
    pub fn to_map(&self) -> HashMap<String, String> {
        let mut m = HashMap::new();
        insert(&mut m, "AZURE_SPEECH_KEY", &self.azure_speech_key);
        insert(&mut m, "AZURE_SPEECH_REGION", &self.azure_speech_region);
        insert(&mut m, "AZURE_SPEECH_LANGUAGE", &self.azure_speech_language);
        insert(&mut m, "AZURE_SPEECH_MODE", &self.azure_speech_mode);
        insert(&mut m, "THEMIS_STT_FIXUP", &self.themis_stt_fixup);
        insert(&mut m, "AZURE_SPEECH_CORRECTIONS", &self.azure_speech_corrections);
        insert(&mut m, "FOUNDRY_ENDPOINT", &self.foundry_endpoint);
        insert(&mut m, "FOUNDRY_API_KEY", &self.foundry_api_key);
        insert(&mut m, "FOUNDRY_DEPLOYMENT", &self.foundry_deployment);
        insert(&mut m, "THEMIS_ANALYSIS_ENABLED", &self.themis_analysis_enabled);
        insert(&mut m, "THEMIS_INSIGHT_DWELL_SECS", &self.themis_insight_dwell_secs);
        insert(
            &mut m,
            "THEMIS_SESSION_SUMMARY_INTERVAL_SECS",
            &self.themis_session_summary_interval_secs,
        );
        insert(&mut m, "THEMIS_AUDIO_CAPTURE_MODE", &self.themis_audio_capture_mode);
        insert(
            &mut m,
            "THEMIS_AUDIO_OUTPUT_DEVICE",
            &self.themis_audio_output_device,
        );
        insert(
            &mut m,
            "THEMIS_AUDIO_INPUT_DEVICE",
            &self.themis_audio_input_device,
        );
        insert(&mut m, "THEMIS_AUDIO_GAIN_MAX", &self.themis_audio_gain_max);
        insert(&mut m, "THEMIS_GRPC_PORT", &self.themis_grpc_port);
        insert(&mut m, "THEMIS_LOG_LEVEL", &self.themis_log_level);
        insert(&mut m, "THEMIS_USE_MOCK_SPEECH", &self.themis_use_mock_speech);
        m
    }

    pub fn from_map(map: &HashMap<String, String>) -> Self {
        let g = |k: &str| map.get(k).cloned().unwrap_or_default();
        Self {
            azure_speech_key: g("AZURE_SPEECH_KEY"),
            azure_speech_region: g("AZURE_SPEECH_REGION"),
            azure_speech_language: g("AZURE_SPEECH_LANGUAGE"),
            azure_speech_mode: g("AZURE_SPEECH_MODE"),
            themis_stt_fixup: g("THEMIS_STT_FIXUP"),
            azure_speech_corrections: g("AZURE_SPEECH_CORRECTIONS"),
            foundry_endpoint: g("FOUNDRY_ENDPOINT"),
            foundry_api_key: g("FOUNDRY_API_KEY"),
            foundry_deployment: g("FOUNDRY_DEPLOYMENT"),
            themis_analysis_enabled: g("THEMIS_ANALYSIS_ENABLED"),
            themis_insight_dwell_secs: g("THEMIS_INSIGHT_DWELL_SECS"),
            themis_session_summary_interval_secs: g("THEMIS_SESSION_SUMMARY_INTERVAL_SECS"),
            themis_audio_capture_mode: g("THEMIS_AUDIO_CAPTURE_MODE"),
            themis_audio_output_device: g("THEMIS_AUDIO_OUTPUT_DEVICE"),
            themis_audio_input_device: g("THEMIS_AUDIO_INPUT_DEVICE"),
            themis_audio_gain_max: g("THEMIS_AUDIO_GAIN_MAX"),
            themis_grpc_port: g("THEMIS_GRPC_PORT"),
            themis_log_level: g("THEMIS_LOG_LEVEL"),
            themis_use_mock_speech: g("THEMIS_USE_MOCK_SPEECH"),
        }
    }
}

fn insert(map: &mut HashMap<String, String>, key: &str, value: &str) {
    map.insert(key.to_string(), value.trim().to_string());
}

/// Keys managed by the settings UI (order used when appending new keys).
pub const MANAGED_ENV_KEYS: &[&str] = &[
    "AZURE_SPEECH_KEY",
    "AZURE_SPEECH_REGION",
    "AZURE_SPEECH_LANGUAGE",
    "AZURE_SPEECH_MODE",
    "THEMIS_STT_FIXUP",
    "AZURE_SPEECH_CORRECTIONS",
    "FOUNDRY_ENDPOINT",
    "FOUNDRY_API_KEY",
    "FOUNDRY_DEPLOYMENT",
    "THEMIS_ANALYSIS_ENABLED",
    "THEMIS_INSIGHT_DWELL_SECS",
    "THEMIS_SESSION_SUMMARY_INTERVAL_SECS",
    "THEMIS_AUDIO_CAPTURE_MODE",
    "THEMIS_AUDIO_OUTPUT_DEVICE",
    "THEMIS_AUDIO_INPUT_DEVICE",
    "THEMIS_AUDIO_GAIN_MAX",
    "THEMIS_GRPC_PORT",
    "THEMIS_LOG_LEVEL",
    "THEMIS_USE_MOCK_SPEECH",
];

pub fn env_file_path() -> Option<PathBuf> {
    find_dotenv_directory().map(|d| d.join(".env"))
}

/// Directory for `.env` — existing file location, or exe/cwd directory to create one.
pub fn env_file_directory() -> PathBuf {
    if let Some(dir) = find_dotenv_directory() {
        return dir;
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            return parent.to_path_buf();
        }
    }
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

pub fn env_file_path_or_default() -> PathBuf {
    env_file_path().unwrap_or_else(|| env_file_directory().join(".env"))
}

/// `.env.example` beside the exe / cwd (release bundle template).
pub fn env_example_path() -> Option<PathBuf> {
    let path = env_file_directory().join(".env.example");
    if path.is_file() {
        Some(path)
    } else {
        None
    }
}

pub fn parse_env_file(content: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = split_assignment(trimmed) {
            map.insert(key, unquote_value(&value));
        }
    }
    map
}

fn split_assignment(line: &str) -> Option<(String, String)> {
    let (key, rest) = line.split_once('=')?;
    let key = key.trim();
    if key.is_empty() {
        return None;
    }
    Some((key.to_string(), rest.trim().to_string()))
}

fn unquote_value(raw: &str) -> String {
    let s = raw.trim();
    if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
        s[1..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
}

fn quote_value(value: &str) -> String {
    let needs_quotes = value.contains(' ')
        || value.contains(',')
        || value.contains('#')
        || value.contains('"');
    if needs_quotes {
        format!("\"{}\"", value.replace('"', "\\\""))
    } else {
        value.to_string()
    }
}

pub fn read_env_settings() -> Result<(PathBuf, EnvSettings), String> {
    let path = env_file_path_or_default();
    let content = if path.is_file() {
        fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?
    } else if let Some(example) = env_example_path() {
        fs::read_to_string(&example)
            .map_err(|e| format!("read {}: {e}", example.display()))?
    } else {
        String::new()
    };
    let map = parse_env_file(&content);
    Ok((path, EnvSettings::from_map(&map)))
}

pub fn write_env_settings(path: &Path, settings: &EnvSettings) -> Result<(), String> {
    let updates = settings.to_map();
    let managed: HashSet<&str> = MANAGED_ENV_KEYS.iter().copied().collect();

    let existing = if path.is_file() {
        fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?
    } else {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("create dir: {e}"))?;
        }
        String::new()
    };

    let mut out = String::new();
    let mut written = HashSet::new();

    for line in existing.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            out.push_str(line);
            out.push('\n');
            continue;
        }
        if let Some((key, _)) = split_assignment(trimmed) {
            if managed.contains(key.as_str()) {
                if let Some(val) = updates.get(&key) {
                    if !val.is_empty() {
                        out.push_str(&key);
                        out.push('=');
                        out.push_str(&quote_value(val));
                        out.push('\n');
                    }
                    written.insert(key);
                }
                continue;
            }
        }
        out.push_str(line);
        out.push('\n');
    }

    for key in MANAGED_ENV_KEYS {
        if written.contains(*key) {
            continue;
        }
        if let Some(val) = updates.get(*key) {
            if !val.is_empty() {
                out.push_str(key);
                out.push('=');
                out.push_str(&quote_value(val));
                out.push('\n');
            }
        }
    }

    fs::write(path, out).map_err(|e| format!("write {}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_quoted_corrections() {
        let content = r#"
# comment
AZURE_SPEECH_KEY=k1
AZURE_SPEECH_CORRECTIONS="Reg:RAG,L L M:LLM"
FOUNDRY_ENDPOINT=https://x.openai.azure.com
"#;
        let map = parse_env_file(content);
        assert_eq!(map.get("AZURE_SPEECH_KEY").unwrap(), "k1");
        assert_eq!(
            map.get("AZURE_SPEECH_CORRECTIONS").unwrap(),
            "Reg:RAG,L L M:LLM"
        );
    }

    #[test]
    fn write_preserves_unknown_keys_and_comments() {
        let path = std::env::temp_dir().join(format!(
            "themis-env-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::write(
            &path,
            "# header\nCUSTOM=keep\nAZURE_SPEECH_KEY=old\nAZURE_SPEECH_REGION=eastus\n",
        )
        .unwrap();
        let mut settings = EnvSettings::default();
        settings.azure_speech_key = "new-key".into();
        settings.azure_speech_region = "westus2".into();
        write_env_settings(&path, &settings).unwrap();
        let body = fs::read_to_string(&path).unwrap();
        assert!(body.contains("# header"));
        assert!(body.contains("CUSTOM=keep"));
        assert!(body.contains("AZURE_SPEECH_KEY=new-key"));
        assert!(body.contains("AZURE_SPEECH_REGION=westus2"));
        let _ = fs::remove_file(&path);
    }
}
