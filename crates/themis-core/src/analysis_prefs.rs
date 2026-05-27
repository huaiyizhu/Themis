use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::ThemisConfig;

fn default_localize_zh() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisPrefs {
    /// When true, term glosses and Q&A answers are generated in Chinese.
    #[serde(default = "default_localize_zh")]
    pub localize_zh: bool,
}

impl Default for AnalysisPrefs {
    fn default() -> Self {
        Self {
            localize_zh: true,
        }
    }
}

impl AnalysisPrefs {
    pub fn path() -> PathBuf {
        ThemisConfig::data_dir().join("analysis-prefs.json")
    }

    pub fn load() -> Self {
        let path = Self::path();
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) -> std::io::Result<()> {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".into()))
    }
}
