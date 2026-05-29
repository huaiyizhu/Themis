mod source;
pub(crate) mod stub;
mod call_detect;
mod composite;

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
    /// macOS / Windows: optional input device name substring for microphone capture.
    pub input_device: Option<String>,
    /// `auto` | `process` | `endpoint` | `call` | `dual` (Windows).
    /// macOS: `auto` | `process_tap` | `input` | `call` | `dual`.
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
            input_device: None,
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
        use crate::call_detect;
        use crate::composite::CompositeAudioSource;
        use crate::windows::{WasapiMicCapture, WasapiSystemOutput};

        let mode = options.capture_mode.trim().to_lowercase();
        let want_dual = call_detect::wants_dual_capture(&mode)
            || (mode == "auto" && call_detect::voice_call_active());

        let output = WasapiSystemOutput::new(sample_rate, channels, options.clone())?;
        if want_dual {
            let mic = WasapiMicCapture::new(options.input_device.clone(), options.gain_max);
            let detail = if call_detect::voice_call_active() {
                "call detected: output loopback + microphone"
            } else {
                "output loopback + microphone"
            };
            return Ok(Box::new(CompositeAudioSource::new(
                vec![Box::new(output), Box::new(mic)],
                "dual",
                detail,
                sample_rate,
                channels,
                options.diagnostics,
            )));
        }
        Ok(Box::new(output))
    }
    #[cfg(target_os = "macos")]
    {
        macos::create_macos_source(
            sample_rate,
            channels,
            options.diagnostics,
            options.input_device,
            &options.capture_mode,
        )
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        let _ = options;
        Ok(Box::new(stub::StubAudioSource::new(sample_rate, channels)))
    }
}
