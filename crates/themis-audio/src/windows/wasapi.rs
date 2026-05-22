//! Windows system audio loopback via WASAPI (captures default speaker output mix).

use crate::stub::StubAudioSource;
use crate::AudioSource;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use std::thread::{self, JoinHandle};
use std::time::Duration;
use themis_core::AudioFrame;
use tokio::sync::mpsc;
use tracing::{info, warn};
use wasapi::*;

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
        let ready_fail = ready_tx.clone();

        let tx_thread = tx.clone();
        let handle = thread::spawn(move || {
            if wasapi_capture_thread(running_flag.clone(), tx_thread, ready_tx, sample_rate, channels)
                .is_err()
            {
                let _ = ready_fail.send(false);
            }
            running_flag.store(false, Ordering::SeqCst);
        });

        match ready_rx.recv_timeout(Duration::from_secs(3)) {
            Ok(true) => {
                self.running = running;
                self.thread = Some(handle);
                info!("WASAPI system loopback capture started");
            }
            _ => {
                running.store(false, Ordering::SeqCst);
                let _ = handle.join();
                warn!("WASAPI loopback failed; using stub source");
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

fn wasapi_capture_thread(
    running: Arc<AtomicBool>,
    tx: mpsc::Sender<AudioFrame>,
    ready_tx: std::sync::mpsc::Sender<bool>,
    _target_rate: u32,
    _target_channels: u16,
) -> anyhow::Result<()> {
    initialize_mta().ok().map_err(|e| anyhow::anyhow!("WASAPI MTA init failed: {e:?}"))?;

    let device = get_default_device(&Direction::Render)
        .map_err(|e| anyhow::anyhow!("no default render device: {e}"))?;
    let mut audio_client = device
        .get_iaudioclient()
        .map_err(|e| anyhow::anyhow!("get_iaudioclient: {e}"))?;
    let format = audio_client
        .get_mixformat()
        .map_err(|e| anyhow::anyhow!("get_mixformat: {e}"))?;
    let (_, min_time) = audio_client
        .get_device_period()
        .map_err(|e| anyhow::anyhow!("get_device_period: {e}"))?;

    let sample_type = format.get_subformat().unwrap_or(SampleType::Float);
    let bits_per_sample = format.get_bitspersample();
    let sample_rate = format.get_samplespersec();
    let channels = format.get_nchannels() as u16;
    let block_align = format.get_blockalign() as usize;

    let mode = StreamMode::EventsShared {
        autoconvert: true,
        buffer_duration_hns: min_time,
    };
    audio_client
        .initialize_client(&format, &Direction::Capture, &mode)
        .map_err(|e| anyhow::anyhow!("initialize_client loopback: {e}"))?;

    let event = audio_client
        .set_get_eventhandle()
        .map_err(|e| anyhow::anyhow!("event handle: {e}"))?;
    let capture_client = audio_client
        .get_audiocaptureclient()
        .map_err(|e| anyhow::anyhow!("capture client: {e}"))?;

    audio_client
        .start_stream()
        .map_err(|e| anyhow::anyhow!("start_stream: {e}"))?;

    let _ = ready_tx.send(true);
    info!(
        sample_rate,
        channels,
        ?sample_type,
        bits_per_sample,
        "WASAPI loopback active (speaker output)"
    );

    let mut sample_queue: VecDeque<u8> = VecDeque::new();
    let min_chunk = block_align * 256;

    while running.load(Ordering::SeqCst) {
        capture_client
            .read_from_device_to_deque(&mut sample_queue)
            .map_err(|e| anyhow::anyhow!("read loopback: {e}"))?;

        while sample_queue.len() >= min_chunk {
            let chunk: Vec<u8> = sample_queue.drain(..min_chunk).collect();
            let samples_i16 = bytes_to_pcm16(&chunk, sample_type, bits_per_sample, channels);
            if !samples_i16.is_empty() {
                send_frame(&tx, samples_i16, sample_rate, channels.max(1));
            }
        }

        if event.wait_for_event(2000).is_err() {
            break;
        }
    }

    let _ = audio_client.stop_stream();
    Ok(())
}

/// Windows mix format is usually IEEE float; misreading as i16 breaks Azure STT.
fn bytes_to_pcm16(bytes: &[u8], sample_type: SampleType, bits_per_sample: u16, channels: u16) -> Vec<i16> {
    let mut out = Vec::new();
    match (sample_type, bits_per_sample) {
        (SampleType::Float, 32) => {
            for frame in bytes.chunks_exact(4) {
                let f = f32::from_le_bytes([frame[0], frame[1], frame[2], frame[3]]);
                let s = (f.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
                out.push(s);
            }
        }
        (SampleType::Int, 16) => {
            for frame in bytes.chunks_exact(2) {
                out.push(i16::from_le_bytes([frame[0], frame[1]]));
            }
        }
        _ => {
            warn!(
                ?sample_type,
                bits_per_sample,
                "unsupported WASAPI format; skipping chunk"
            );
        }
    }

    // Downmix interleaved multichannel to mono for STT pipeline.
    if channels > 1 && !out.is_empty() {
        out = out
            .chunks(channels as usize)
            .map(|c| c.iter().map(|&s| s as i32).sum::<i32>() / c.len() as i32)
            .map(|s| s as i16)
            .collect();
    }

    out
}

fn send_frame(tx: &mpsc::Sender<AudioFrame>, samples: Vec<i16>, rate: u32, channels: u16) {
    let frame = AudioFrame::new(samples, rate, channels.min(1).max(1));
    let tx = tx.clone();
    runtime().spawn(async move {
        let _ = tx.send(frame).await;
    });
}
