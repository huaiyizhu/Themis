mod mock;
mod multilang;
mod recognition;
mod recognizer;
mod rest;
mod streaming;

pub use mock::MockSpeechRecognizer;
pub use multilang::AzureMultiLangRestRecognizer;
pub use recognizer::{SpeechEvent, SpeechRecognizer};
pub use rest::{check_connectivity, AzureRestRecognizer};
pub use streaming::AzureStreamingRecognizer;

use themis_core::ThemisConfig;

/// Resolve Azure languages from `AZURE_SPEECH_LANGUAGE`.
/// - `auto` → en-US + zh-CN
/// - `en-US,zh-CN` → explicit list
/// - `en-US` → single language
pub fn resolve_speech_languages(config: &ThemisConfig) -> Vec<String> {
    let raw = config.speech_language.trim();
    if raw.eq_ignore_ascii_case("auto") {
        return vec!["en-US".into(), "zh-CN".into()];
    }
    if raw.contains(',') {
        return raw
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
    }
    vec![raw.to_string()]
}

pub fn create_recognizer(config: &ThemisConfig) -> Box<dyn SpeechRecognizer + Send> {
    if config.use_mock_speech {
        return Box::new(MockSpeechRecognizer::new());
    }

    let (Some(key), Some(region)) = (&config.azure_speech_key, &config.azure_speech_region) else {
        return Box::new(MockSpeechRecognizer::new());
    };

    let mode = std::env::var("AZURE_SPEECH_MODE")
        .unwrap_or_else(|_| "rest".into())
        .to_lowercase();

    if mode == "streaming" {
        let langs = resolve_speech_languages(config);
        let language = langs.first().cloned().unwrap_or_else(|| "en-US".into());
        return Box::new(AzureStreamingRecognizer::new(
            key.clone(),
            region.clone(),
            language,
        ));
    }

    let languages = resolve_speech_languages(config);
    if languages.len() > 1 {
        Box::new(AzureMultiLangRestRecognizer::new(
            key.clone(),
            region.clone(),
            languages,
        ))
    } else {
        let language = languages
            .into_iter()
            .next()
            .unwrap_or_else(|| "en-US".into());
        Box::new(AzureRestRecognizer::new(
            key.clone(),
            region.clone(),
            language,
        ))
    }
}
