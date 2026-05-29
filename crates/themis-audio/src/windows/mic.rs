//! WASAPI capture from the default microphone (or named input device).

use crate::windows::pcm;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use themis_core::{normalize_pcm16, AudioFrame, CaptureDiagnostics};
use tokio::sync::mpsc;
use tracing::{info, warn};
use wasapi::*;

pub struct WasapiMicCapture {
    device_hint: Option<String>,
    gain_max: f32,
    running: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl WasapiMicCapture {
    pub fn new(device_hint: Option<String>, gain_max: f32) -> Self {
        Self {
            device_hint,
            gain_max,
            running: Arc::new(AtomicBool::new(false)),
            thread: None,
        }
    }
}

impl crate::AudioSource for WasapiMicCapture {
    fn start(&mut self, tx: mpsc::Sender<AudioFrame>) -> anyhow::Result<()> {
        if self.running.load(Ordering::SeqCst) {
            return Ok(());
        }

        let running = Arc::new(AtomicBool::new(true));
        self.running = Arc::clone(&running);
        let device_hint = self.device_hint.clone();
        let gain_max = self.gain_max;

        let handle = thread::spawn(move || {
            if let Err(e) = mic_thread(running, tx, device_hint, gain_max) {
                warn!(error = %e, "microphone capture ended");
            }
        });
        self.thread = Some(handle);
        Ok(())
    }

    fn stop(&mut self) -> anyhow::Result<()> {
        self.running.store(false, Ordering::SeqCst);
        if let Some(h) = self.thread.take() {
            let _ = h.join();
        }
        Ok(())
    }
}

fn open_capture_device(hint: Option<String>) -> anyhow::Result<Device> {
    if let Some(hint) = hint {
        let collection = DeviceCollection::new(&Direction::Capture)?;
        let count = collection.get_nbr_devices()?;
        for i in 0..count {
            let dev = collection.get_device_at_index(i)?;
            let id = dev.get_id()?;
            if id == hint {
                return Ok(dev);
            }
            let name = dev.get_friendlyname()?;
            if name.to_ascii_lowercase().contains(&hint.to_ascii_lowercase()) {
                return Ok(dev);
            }
        }
        anyhow::bail!("capture device not found: {hint}");
    }
    get_default_device(&Direction::Capture).map_err(|e| anyhow::anyhow!("{e}"))
}

fn mic_thread(
    running: Arc<AtomicBool>,
    tx: mpsc::Sender<AudioFrame>,
    device_hint: Option<String>,
    gain_max: f32,
) -> anyhow::Result<()> {
    let _ = initialize_mta().ok();

    let device = open_capture_device(device_hint)?;
    let device_name = device.get_friendlyname().unwrap_or_else(|_| "?".into());
    info!(device = %device_name, "microphone capture active");

    let mut audio_client = device.get_iaudioclient()?;
    let format = audio_client.get_mixformat()?;
    let (_, min_time) = audio_client.get_device_period()?;

    let sample_type = format.get_subformat().unwrap_or(SampleType::Float);
    let bits = format.get_bitspersample();
    let sample_rate = format.get_samplespersec();
    let channels = format.get_nchannels() as u16;
    let block_align = format.get_blockalign() as usize;

    let mode = StreamMode::EventsShared {
        autoconvert: true,
        buffer_duration_hns: min_time,
    };
    audio_client.initialize_client(&format, &Direction::Capture, &mode)?;

    let event = audio_client.set_get_eventhandle()?;
    let capture_client = audio_client.get_audiocaptureclient()?;
    audio_client.start_stream()?;

    let mut queue: std::collections::VecDeque<u8> = std::collections::VecDeque::new();
    let min_chunk = block_align * 256;

    while running.load(Ordering::SeqCst) {
        capture_client.read_from_device_to_deque(&mut queue)?;
        while queue.len() >= min_chunk {
            let chunk: Vec<u8> = queue.drain(..min_chunk).collect();
            let mut samples = pcm::bytes_to_pcm16(&chunk, sample_type, bits, channels);
            if samples.is_empty() {
                continue;
            }
            let _ = normalize_pcm16(&mut samples, 12_000, gain_max);
            pcm::send_frame(&tx, samples, sample_rate);
        }
        if event.wait_for_event(2000).is_err() {
            break;
        }
    }

    let _ = audio_client.stop_stream();
    Ok(())
}
