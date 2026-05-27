//! macOS system audio capture.
//!
//! Native output tap requires additional entitlements. This implementation uses
//! cpal with the default input device. For full system-audio loopback, install
//! [BlackHole](https://existential.audio/blackhole/) and select it as input — see
//! `docs/platform-notes.md`.

use crate::stub::StubAudioSource;
use crate::AudioSource;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, SampleRate};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use std::thread::{self, JoinHandle};
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

fn send_frame(tx: &mpsc::Sender<AudioFrame>, samples: Vec<i16>, rate: u32, channels: u16) {
    let frame = AudioFrame::new(samples, rate, channels);
    let tx = tx.clone();
    runtime().spawn(async move {
        let _ = tx.send(frame).await;
    });
}

pub struct CoreAudioLoopback {
    sample_rate: u32,
    channels: u16,
    input_device_hint: Option<String>,
    diagnostics: Option<Arc<CaptureDiagnostics>>,
    running: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
    fallback: Option<StubAudioSource>,
}

fn resolve_input_device(
    host: &cpal::Host,
    hint: Option<&str>,
) -> Option<cpal::Device> {
    if let Some(sub) = hint.filter(|s| !s.is_empty()) {
        if let Ok(devices) = host.input_devices() {
            for device in devices {
                if device
                    .name()
                    .ok()
                    .is_some_and(|name| name.contains(sub))
                {
                    info!(device = %device.name().unwrap_or_default(), "macOS input device matched hint");
                    return Some(device);
                }
            }
            warn!(hint = %sub, "no input device matched THEMIS_AUDIO_INPUT_DEVICE");
        }
    }
    host.default_input_device()
}

impl CoreAudioLoopback {
    pub fn new(
        sample_rate: u32,
        channels: u16,
        diagnostics: Option<Arc<CaptureDiagnostics>>,
        input_device_hint: Option<String>,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            sample_rate,
            channels,
            input_device_hint,
            diagnostics,
            running: Arc::new(AtomicBool::new(false)),
            thread: None,
            fallback: None,
        })
    }
}

impl AudioSource for CoreAudioLoopback {
    fn start(&mut self, tx: mpsc::Sender<AudioFrame>) -> anyhow::Result<()> {
        if self.running.load(Ordering::SeqCst) {
            return Ok(());
        }

        let host = cpal::default_host();
        let Some(device) =
            resolve_input_device(&host, self.input_device_hint.as_deref())
        else {
            warn!("no input device; falling back to stub audio source");
            if let Some(d) = &self.diagnostics {
                d.set_mode("stub");
                d.set_detail("no input device");
            }
            let mut stub = StubAudioSource::new(self.sample_rate, self.channels);
            stub.start(tx)?;
            self.fallback = Some(stub);
            return Ok(());
        };

        let device_name = device.name().unwrap_or_else(|_| "unknown".into());
        if let Some(d) = &self.diagnostics {
            d.set_mode("input");
            d.set_detail(format!(
                "default input: {device_name} (route system audio via BlackHole — docs/platform-notes.md)"
            ));
            d.set_sessions(1);
        }

        let sample_rate = self.sample_rate;
        let channels = self.channels;
        let running = Arc::clone(&self.running);
        let diagnostics = self.diagnostics.clone();
        running.store(true, Ordering::SeqCst);

        let handle = thread::spawn(move || {
            if let Err(e) = capture_thread(device, running.clone(), tx, sample_rate, channels, diagnostics) {
                tracing::error!(error = %e, "macOS capture thread exited");
            }
            running.store(false, Ordering::SeqCst);
        });

        self.thread = Some(handle);
        info!("macOS cpal input capture started");
        Ok(())
    }

    fn stop(&mut self) -> anyhow::Result<()> {
        self.running.store(false, Ordering::SeqCst);
        if let Some(h) = self.thread.take() {
            let _ = h.join();
        }
        if let Some(mut stub) = self.fallback.take() {
            stub.stop()?;
        }
        Ok(())
    }
}

fn frame_peak(samples: &[i16]) -> u32 {
    samples
        .iter()
        .map(|&s| u32::from(s.unsigned_abs()))
        .max()
        .unwrap_or(0)
}

fn capture_thread(
    device: cpal::Device,
    running: Arc<AtomicBool>,
    tx: mpsc::Sender<AudioFrame>,
    target_rate: u32,
    target_channels: u16,
    diagnostics: Option<Arc<CaptureDiagnostics>>,
) -> anyhow::Result<()> {
    let config = device.default_input_config()?;
    let sample_format = config.sample_format();
    let stream_config = cpal::StreamConfig {
        channels: config.channels(),
        sample_rate: SampleRate(target_rate),
        buffer_size: cpal::BufferSize::Default,
    };

    let err_fn = |e| tracing::error!(error = %e, "cpal macOS stream error");

    let stream = match sample_format {
        SampleFormat::F32 => {
            let diag = diagnostics.clone();
            device.build_input_stream(
                &stream_config,
                {
                    let tx = tx.clone();
                    move |data: &[f32], _| {
                        let samples: Vec<i16> = data
                            .iter()
                            .map(|&s| (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)
                            .collect();
                        if let Some(d) = &diag {
                            d.record_frame(frame_peak(&samples));
                        }
                        send_frame(&tx, samples, target_rate, target_channels);
                    }
                },
                err_fn,
                None,
            )?
        }
        SampleFormat::I16 => {
            let diag = diagnostics.clone();
            device.build_input_stream(
                &stream_config,
                {
                    let tx = tx.clone();
                    move |data: &[i16], _| {
                        if let Some(d) = &diag {
                            d.record_frame(frame_peak(data));
                        }
                        send_frame(&tx, data.to_vec(), target_rate, target_channels);
                    }
                },
                err_fn,
                None,
            )?
        }
        _ => anyhow::bail!("unsupported sample format"),
    };

    stream.play()?;
    while running.load(Ordering::SeqCst) {
        thread::sleep(std::time::Duration::from_millis(50));
    }
    drop(stream);
    Ok(())
}
