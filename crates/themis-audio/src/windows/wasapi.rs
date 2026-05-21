use crate::stub::StubAudioSource;
use crate::AudioSource;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, SampleRate, Stream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use std::thread::{self, JoinHandle};
use std::time::Duration;
use themis_core::AudioFrame;
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

pub struct WasapiLoopback {
    sample_rate: u32,
    channels: u16,
    running: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
    fallback: Option<StubAudioSource>,
}

impl WasapiLoopback {
    pub fn new(sample_rate: u32, channels: u16) -> anyhow::Result<Self> {
        Ok(Self {
            sample_rate,
            channels,
            running: Arc::new(AtomicBool::new(false)),
            thread: None,
            fallback: None,
        })
    }
}

impl AudioSource for WasapiLoopback {
    fn start(&mut self, tx: mpsc::Sender<AudioFrame>) -> anyhow::Result<()> {
        if self.running.load(Ordering::SeqCst) {
            return Ok(());
        }

        let sample_rate = self.sample_rate;
        let channels = self.channels;
        let running = Arc::new(AtomicBool::new(true));
        let running_flag = Arc::clone(&running);
        let (ready_tx, ready_rx) = std::sync::mpsc::channel::<bool>();

        let tx_thread = tx.clone();
        let handle = thread::spawn(move || {
            let _ = capture_thread(
                running_flag.clone(),
                tx_thread,
                ready_tx,
                sample_rate,
                channels,
            );
            running_flag.store(false, Ordering::SeqCst);
        });

        match ready_rx.recv_timeout(Duration::from_secs(2)) {
            Ok(true) => {
                self.running = running;
                self.thread = Some(handle);
                info!("WASAPI loopback capture started");
            }
            _ => {
                let _ = handle.join();
                warn!("loopback unavailable; using stub source");
                let mut stub = StubAudioSource::new(sample_rate, channels);
                stub.start(tx)?;
                self.fallback = Some(stub);
            }
        }

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

fn capture_thread(
    running: Arc<AtomicBool>,
    tx: mpsc::Sender<AudioFrame>,
    ready_tx: std::sync::mpsc::Sender<bool>,
    target_rate: u32,
    target_channels: u16,
) -> anyhow::Result<()> {
    let stream = match try_start_loopback(&tx, target_rate, target_channels) {
        Ok(s) => {
            let _ = ready_tx.send(true);
            s
        }
        Err(e) => {
            let _ = ready_tx.send(false);
            return Err(e);
        }
    };
    while running.load(Ordering::SeqCst) {
        thread::sleep(Duration::from_millis(50));
    }
    drop(stream);
    Ok(())
}

fn try_start_loopback(
    tx: &mpsc::Sender<AudioFrame>,
    target_rate: u32,
    target_channels: u16,
) -> anyhow::Result<Stream> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or_else(|| anyhow::anyhow!("no default output device"))?;

    let output = device.default_output_config()?;
    let sample_format = output.sample_format();
    let stream_config = cpal::StreamConfig {
        channels: output.channels(),
        sample_rate: SampleRate(target_rate.min(output.sample_rate().0)),
        buffer_size: cpal::BufferSize::Default,
    };

    let err_fn = |e| tracing::error!(error = %e, "cpal stream error");

    let stream = match sample_format {
        SampleFormat::F32 => device.build_input_stream(
            &stream_config,
            {
                let tx = tx.clone();
                move |data: &[f32], _| {
                    let samples: Vec<i16> = data
                        .iter()
                        .map(|&s| (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)
                        .collect();
                    send_frame(&tx, samples, target_rate, target_channels);
                }
            },
            err_fn,
            None,
        )?,
        SampleFormat::I16 => device.build_input_stream(
            &stream_config,
            {
                let tx = tx.clone();
                move |data: &[i16], _| {
                    send_frame(&tx, data.to_vec(), target_rate, target_channels);
                }
            },
            err_fn,
            None,
        )?,
        _ => anyhow::bail!("unsupported sample format"),
    };

    stream.play()?;
    Ok(stream)
}

fn send_frame(tx: &mpsc::Sender<AudioFrame>, samples: Vec<i16>, rate: u32, channels: u16) {
    let frame = AudioFrame::new(samples, rate, channels);
    let tx = tx.clone();
    runtime().spawn(async move {
        let _ = tx.send(frame).await;
    });
}
