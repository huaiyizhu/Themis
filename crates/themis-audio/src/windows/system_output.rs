//! System audio output capture for Windows.
//!
//! **Process loopback** (per application) is preferred — it captures app audio even when
//! the default playback device is muted or very quiet on many systems.
//! **Endpoint loopback** is the fallback when no active audio sessions exist.

use crate::stub::StubAudioSource;
use crate::{AudioSource, SystemAudioOptions};
use crate::windows::endpoint;
use crate::windows::process;
use crate::windows::sessions::active_audio_session_pids;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use themis_core::{AudioFrame, CaptureDiagnostics};
use tokio::sync::mpsc;
use tracing::{info, warn};

#[derive(Clone, Copy, PartialEq, Eq)]
enum CaptureStrategy {
    Auto,
    Process,
    Endpoint,
}

pub struct WasapiSystemOutput {
    _sample_rate: u32,
    _channels: u16,
    options: SystemAudioOptions,
    running: Arc<AtomicBool>,
    coordinator: Option<JoinHandle<()>>,
    fallback: Option<StubAudioSource>,
}

impl WasapiSystemOutput {
    pub fn new(
        sample_rate: u32,
        channels: u16,
        options: SystemAudioOptions,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            _sample_rate: sample_rate,
            _channels: channels,
            options,
            running: Arc::new(AtomicBool::new(false)),
            coordinator: None,
            fallback: None,
        })
    }

    fn strategy(&self) -> CaptureStrategy {
        match self.options.capture_mode.as_str() {
            "process" => CaptureStrategy::Process,
            "endpoint" => CaptureStrategy::Endpoint,
            _ => CaptureStrategy::Auto,
        }
    }
}

impl AudioSource for WasapiSystemOutput {
    fn start(&mut self, tx: mpsc::Sender<AudioFrame>) -> anyhow::Result<()> {
        if self.running.load(Ordering::SeqCst) {
            return Ok(());
        }

        let running = Arc::new(AtomicBool::new(true));
        let running_flag = Arc::clone(&running);
        let (ready_tx, ready_rx) = std::sync::mpsc::channel::<bool>();

        let options = self.options.clone();
        let strategy = self.strategy();
        let gain_max = options.gain_max;

        let tx_coord = tx.clone();
        let handle = thread::spawn(move || {
            coordinator_thread(running_flag.clone(), tx_coord, options, strategy, gain_max, ready_tx);
            running_flag.store(false, Ordering::SeqCst);
        });

        match ready_rx.recv_timeout(Duration::from_secs(8)) {
            Ok(true) => {
                self.running = running;
                self.coordinator = Some(handle);
                info!("Windows system audio output capture started");
            }
            _ => {
                running.store(false, Ordering::SeqCst);
                let _ = handle.join();
                warn!("system audio capture failed to start; using stub");
                let mut stub = StubAudioSource::new(self._sample_rate, self._channels);
                stub.start(tx)?;
                self.fallback = Some(stub);
            }
        }

        Ok(())
    }

    fn stop(&mut self) -> anyhow::Result<()> {
        self.running.store(false, Ordering::SeqCst);
        if let Some(h) = self.coordinator.take() {
            let _ = h.join();
        }
        if let Some(mut stub) = self.fallback.take() {
            stub.stop()?;
        }
        Ok(())
    }
}

fn coordinator_thread(
    running: Arc<AtomicBool>,
    tx: mpsc::Sender<AudioFrame>,
    options: SystemAudioOptions,
    strategy: CaptureStrategy,
    gain_max: f32,
    ready_tx: std::sync::mpsc::Sender<bool>,
) {
    let diagnostics = options
        .diagnostics
        .clone()
        .unwrap_or_else(|| Arc::new(CaptureDiagnostics::new()));

    let pids = if strategy == CaptureStrategy::Endpoint {
        Vec::new()
    } else {
        active_audio_session_pids().unwrap_or_default()
    };
    diagnostics.set_sessions(pids.len() as u32);

    let use_process =
        (strategy == CaptureStrategy::Process || strategy == CaptureStrategy::Auto) && !pids.is_empty();

    let mut handles: Vec<JoinHandle<()>> = Vec::new();

    if use_process {
        diagnostics.set_mode("process");
        for pid in pids.iter().take(6) {
            info!(pid, "starting process loopback capture");
            handles.push(process::spawn_process_capture(
                *pid,
                Arc::clone(&running),
                tx.clone(),
                gain_max,
                Arc::clone(&diagnostics),
            ));
        }
        diagnostics.set_detail(format!("process loopback: {} session(s)", pids.len()));
    } else {
        diagnostics.set_mode("endpoint");
        info!("using endpoint loopback");
        handles.push(endpoint::spawn_endpoint_capture(
            Arc::clone(&running),
            tx,
            options.output_device,
            gain_max,
            Arc::clone(&diagnostics),
        ));
    }

    let _ = ready_tx.send(true);

    let mut last_scan = Instant::now();
    while running.load(Ordering::SeqCst) {
        if last_scan.elapsed() >= Duration::from_millis(1500) {
            if let Ok(cur) = active_audio_session_pids() {
                diagnostics.set_sessions(cur.len() as u32);
            }
            last_scan = Instant::now();
        }
        thread::sleep(Duration::from_millis(200));
    }

    for h in handles {
        let _ = h.join();
    }
}
