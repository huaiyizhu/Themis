mod mock;
mod recognizer;
mod rest;
mod streaming;

pub use mock::MockSpeechRecognizer;
pub use recognizer::{SpeechEvent, SpeechRecognizer};
pub use rest::{check_connectivity, AzureRestRecognizer};
pub use streaming::AzureStreamingRecognizer;

use themis_core::ThemisConfig;

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
        Box::new(AzureStreamingRecognizer::new(
            key.clone(),
            region.clone(),
            config.speech_language.clone(),
        ))
    } else {
        Box::new(AzureRestRecognizer::new(
            key.clone(),
            region.clone(),
            config.speech_language.clone(),
        ))
    }
}
