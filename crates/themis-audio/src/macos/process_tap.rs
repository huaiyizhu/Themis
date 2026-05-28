//! macOS 14.2+ system audio via Core Audio Process Tap (no BlackHole).

use crate::AudioSource;
use std::ffi::CStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use themis_core::{AudioFrame, CaptureDiagnostics};
use tokio::sync::mpsc;
use tracing::{info, warn};

static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();

fn runtime() -> &'static tokio::runtime::Runtime {
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .worker_threads(1)
            .build()
            .expect("tokio runtime")
    })
}

#[repr(C)]
struct TapCallbackCtx {
    tx: mpsc::Sender<AudioFrame>,
    target_rate: u32,
    target_channels: u16,
    diagnostics: Option<Arc<CaptureDiagnostics>>,
}

extern "C" {
    fn themis_tap_create_system() -> *mut std::ffi::c_void;
    fn themis_tap_start(
        tap: *mut std::ffi::c_void,
        callback: Option<extern "C" fn(*const i16, u32, u32, u16, *mut std::ffi::c_void)>,
        userdata: *mut std::ffi::c_void,
    ) -> i32;
    fn themis_tap_stop(tap: *mut std::ffi::c_void);
    fn themis_tap_destroy(tap: *mut std::ffi::c_void);
    fn themis_tap_detail(tap: *const std::ffi::c_void) -> *const std::ffi::c_char;
}

fn format_osstatus(st: i32) -> String {
    let b = (st as u32).to_be_bytes();
    let four: String = b
        .iter()
        .map(|&c| {
            if c.is_ascii_graphic() {
                c as char
            } else {
                '?'
            }
        })
        .collect();
    format!("{st} ('{four}')")
}

/// User-facing steps when `AudioDeviceStart` is denied (common: missing TCC / plist).
pub fn system_audio_recording_help() -> &'static str {
    "macOS blocked System Audio Recording (OSStatus 'nope').\n\
     1) Rebuild so the binary embeds Info.plist: `cargo build -p themis-cli -p themis-service`\n\
     2) Run `./scripts/themis.sh probe` again — allow the permission dialog if shown\n\
     3) If no dialog: System Settings → Privacy & Security → System Audio Recording\n\
        → enable **themis-cli** (or your terminal app), then retry\n\
     4) If you denied before: toggle off/on, or run `tccutil reset SystemAudioRecording` and probe again"
}

fn tap_start_error(st: i32) -> anyhow::Error {
    let label = format_osstatus(st);
    let mut msg = format!("themis_tap_start failed: {label}");
    if st == 1852797029 || label.contains("'nope'") {
        msg.push_str("\n\n");
        msg.push_str(system_audio_recording_help());
    }
    anyhow::anyhow!(msg)
}

extern "C" fn tap_audio_callback(
    samples: *const i16,
    num_samples: u32,
    sample_rate: u32,
    channels: u16,
    userdata: *mut std::ffi::c_void,
) {
    if samples.is_null() || userdata.is_null() || num_samples == 0 {
        return;
    }
    let ctx = unsafe { &*(userdata as *const TapCallbackCtx) };
    let slice = unsafe { std::slice::from_raw_parts(samples, num_samples as usize) };

    let peak = slice
        .iter()
        .map(|&s| u32::from(s.unsigned_abs()))
        .max()
        .unwrap_or(0);
    if let Some(d) = &ctx.diagnostics {
        d.record_frame(peak);
    }

    let frame = AudioFrame::new(slice.to_vec(), sample_rate, channels);
    let mono = frame.to_mono_pcm16(ctx.target_rate);
    let out = AudioFrame::new(mono, ctx.target_rate, ctx.target_channels);
    let tx = ctx.tx.clone();
    runtime().spawn(async move {
        let _ = tx.send(out).await;
    });
}

pub struct ProcessTapCapture {
    sample_rate: u32,
    channels: u16,
    diagnostics: Option<Arc<CaptureDiagnostics>>,
    tap: Option<*mut std::ffi::c_void>,
    ctx: Option<Box<TapCallbackCtx>>,
    running: Arc<AtomicBool>,
    #[allow(dead_code)]
    detail: String,
}

impl ProcessTapCapture {
    pub fn try_new(
        sample_rate: u32,
        channels: u16,
        diagnostics: Option<Arc<CaptureDiagnostics>>,
    ) -> Option<Self> {
        let tap = unsafe { themis_tap_create_system() };
        if tap.is_null() {
            warn!("Core Audio process tap unavailable (requires macOS 14.2+)");
            return None;
        }
        let detail = unsafe {
            CStr::from_ptr(themis_tap_detail(tap))
                .to_string_lossy()
                .into_owned()
        };
        if let Some(d) = &diagnostics {
            d.set_mode("process_tap");
            d.set_detail(detail.clone());
            d.set_sessions(1);
        }
        info!(%detail, "macOS process tap created");
        Some(Self {
            sample_rate,
            channels,
            diagnostics,
            tap: Some(tap),
            ctx: None,
            running: Arc::new(AtomicBool::new(false)),
            detail,
        })
    }
}

unsafe impl Send for ProcessTapCapture {}

impl AudioSource for ProcessTapCapture {
    fn start(&mut self, tx: mpsc::Sender<AudioFrame>) -> anyhow::Result<()> {
        let tap = self.tap.ok_or_else(|| anyhow::anyhow!("process tap not initialized"))?;
        let ctx = Box::new(TapCallbackCtx {
            tx,
            target_rate: self.sample_rate,
            target_channels: self.channels,
            diagnostics: self.diagnostics.clone(),
        });
        let ctx_ptr = &*ctx as *const TapCallbackCtx as *mut std::ffi::c_void;
        let st = unsafe { themis_tap_start(tap, Some(tap_audio_callback), ctx_ptr) };
        if st != 0 {
            return Err(tap_start_error(st));
        }
        self.ctx = Some(ctx);
        self.running.store(true, Ordering::SeqCst);
        info!("macOS process tap IO started");
        Ok(())
    }

    fn stop(&mut self) -> anyhow::Result<()> {
        self.running.store(false, Ordering::SeqCst);
        if let Some(tap) = self.tap {
            unsafe { themis_tap_stop(tap) };
        }
        self.ctx = None;
        Ok(())
    }
}

impl Drop for ProcessTapCapture {
    fn drop(&mut self) {
        if let Some(tap) = self.tap.take() {
            unsafe { themis_tap_destroy(tap) };
        }
    }
}
