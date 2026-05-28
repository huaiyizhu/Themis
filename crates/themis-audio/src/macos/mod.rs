mod core_audio;
mod process_tap;

use crate::AudioSource;
use std::sync::Arc;
use themis_core::CaptureDiagnostics;
use tracing::warn;

pub use core_audio::CoreAudioLoopback;

pub fn create_macos_source(
    sample_rate: u32,
    channels: u16,
    diagnostics: Option<Arc<CaptureDiagnostics>>,
    input_device_hint: Option<String>,
    capture_mode: &str,
) -> anyhow::Result<Box<dyn AudioSource + Send>> {
    let mode = capture_mode.trim().to_lowercase();
    let want_tap = matches!(mode.as_str(), "auto" | "tap" | "process_tap" | "process");
    let want_input = matches!(mode.as_str(), "input" | "endpoint");

    if want_tap && !want_input {
        if let Some(tap) =
            process_tap::ProcessTapCapture::try_new(sample_rate, channels, diagnostics.clone())
        {
            return Ok(Box::new(tap));
        }
        if mode != "auto" {
            anyhow::bail!(
                "THEMIS_AUDIO_CAPTURE_MODE={mode} requires macOS 14.2+ process tap"
            );
        }
        warn!("process tap failed; falling back to default input device");
    }

    Ok(Box::new(CoreAudioLoopback::new(
        sample_rate,
        channels,
        diagnostics,
        input_device_hint,
    )?))
}
