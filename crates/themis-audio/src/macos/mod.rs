mod core_audio;
mod process_tap;

use crate::call_detect;
use crate::composite::CompositeAudioSource;
use crate::AudioSource;
use std::sync::Arc;
use themis_core::CaptureDiagnostics;
use tracing::{info, warn};

pub use core_audio::CoreAudioLoopback;

fn wants_dual_capture(mode: &str) -> bool {
    call_detect::wants_dual_capture(mode)
        || (mode.trim().eq_ignore_ascii_case("auto") && call_detect::voice_call_active())
}

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
    let want_dual = wants_dual_capture(&mode);

    if want_dual {
        if let Some(tap) =
            process_tap::ProcessTapCapture::try_new(sample_rate, channels, None)
        {
            let mic = CoreAudioLoopback::new(
                sample_rate,
                channels,
                None,
                input_device_hint.clone(),
            )?;
            let detail: String = if call_detect::voice_call_active() {
                "call detected: process tap (output) + microphone (input)".into()
            } else {
                "process tap (output) + microphone (input)".into()
            };
            info!("macOS dual capture: system output + microphone");
            return Ok(Box::new(CompositeAudioSource::new(
                vec![Box::new(tap), Box::new(mic)],
                "dual",
                detail,
                sample_rate,
                channels,
                diagnostics,
            )));
        }
        if mode != "auto" && !call_detect::wants_dual_capture(&mode) {
            warn!("process tap unavailable for dual capture; falling back to input only");
        } else if call_detect::wants_dual_capture(&mode) {
            anyhow::bail!(
                "THEMIS_AUDIO_CAPTURE_MODE={mode} requires macOS 14.2+ process tap"
            );
        }
    }

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
