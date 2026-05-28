mod source;
pub(crate) mod stub;

#[cfg(target_os = "macos")]
mod macos;

#[cfg(windows)]
mod windows;

pub use source::{AudioSource, LoopbackSource};

use std::sync::Arc;
use themis_core::CaptureDiagnostics;

/// Options for capturing **system audio output** (playback), not the microphone.
#[derive(Debug, Clone)]
pub struct SystemAudioOptions {
    /// Windows: friendly name substring or endpoint device ID.
    pub output_device: Option<String>,
    /// `auto` | `process` | `endpoint` (Windows). Default: `auto`.
    pub capture_mode: String,
    /// Auto-gain ceiling for quiet loopback (see `THEMIS_AUDIO_GAIN_MAX`).
    pub gain_max: f32,
    /// Shared diagnostics sink (peak, frame count, mode).
    pub diagnostics: Option<Arc<CaptureDiagnostics>>,
}

impl Default for SystemAudioOptions {
    fn default() -> Self {
        Self {
            output_device: None,
            capture_mode: "auto".into(),
            gain_max: 16.0,
            diagnostics: None,
        }
    }
}

pub fn create_loopback(
    sample_rate: u32,
    channels: u16,
    options: SystemAudioOptions,
) -> anyhow::Result<Box<dyn AudioSource + Send>> {
    #[cfg(windows)]
    {
        Ok(Box::new(windows::WasapiSystemOutput::new(
            sample_rate,
            channels,
            options,
        )?))
    }
    #[cfg(target_os = "macos")]
    {
        macos::create_macos_source(
            sample_rate,
            channels,
            options.diagnostics,
            std::env::var("THEMIS_AUDIO_INPUT_DEVICE")
                .ok()
                .filter(|s| !s.is_empty()),
            &options.capture_mode,
        )
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        let _ = options;
        Ok(Box::new(stub::StubAudioSource::new(sample_rate, channels)))
    }
}
