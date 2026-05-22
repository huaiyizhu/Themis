mod mock;
mod recognizer;
mod rest;

pub use mock::MockSpeechRecognizer;
pub use recognizer::{SpeechEvent, SpeechRecognizer};
pub use rest::{check_connectivity, AzureRestRecognizer};

use themis_core::ThemisConfig;

pub fn create_recognizer(config: &ThemisConfig) -> Box<dyn SpeechRecognizer + Send> {
    if config.use_mock_speech {
        Box::new(MockSpeechRecognizer::new())
    } else if let (Some(key), Some(region)) =
        (&config.azure_speech_key, &config.azure_speech_region)
    {
        Box::new(AzureRestRecognizer::new(
            key.clone(),
            region.clone(),
            config.speech_language.clone(),
        ))
    } else {
        Box::new(MockSpeechRecognizer::new())
    }
}
