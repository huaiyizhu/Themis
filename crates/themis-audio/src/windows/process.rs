//! Per-process WASAPI loopback — captures app audio before endpoint mute/volume in many cases.

use crate::windows::pcm;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use themis_core::{normalize_pcm16, AudioFrame, CaptureDiagnostics};
use tokio::sync::mpsc;
use tracing::{info, warn};
use wasapi::*;

const PROCESS_RATE: u32 = 44_100;
const PROCESS_CHANNELS: u16 = 2;

pub fn spawn_process_capture(
    pid: u32,
    running: Arc<AtomicBool>,
    tx: mpsc::Sender<AudioFrame>,
    gain_max: f32,
    diagnostics: Arc<CaptureDiagnostics>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        if let Err(e) = process_thread(pid, running, tx, gain_max, diagnostics) {
            warn!(pid, error = %e, "process loopback ended");
        }
    })
}

fn process_thread(
    pid: u32,
    running: Arc<AtomicBool>,
    tx: mpsc::Sender<AudioFrame>,
    gain_max: f32,
    diagnostics: Arc<CaptureDiagnostics>,
) -> anyhow::Result<()> {
    let _ = initialize_mta().ok();

    let mut audio_client = AudioClient::new_application_loopback_client(pid, true)
        .map_err(|e| anyhow::anyhow!("process {pid} loopback: {e}"))?;

    // Microsoft Application Loopback sample format (44.1 kHz stereo PCM).
    let format = WaveFormat::new(16, 16, &SampleType::Int, 44_100, 2, None);
    let mode = StreamMode::EventsShared {
        autoconvert: true,
        buffer_duration_hns: 200_000,
    };
    audio_client
        .initialize_client(&format, &Direction::Capture, &mode)
        .map_err(|e| anyhow::anyhow!("process {pid} init: {e}"))?;

    let event = audio_client.set_get_eventhandle()?;
    let capture_client = audio_client.get_audiocaptureclient()?;
    audio_client.start_stream()?;

    info!(pid, "process loopback active");
    diagnostics.set_detail(format!("process pid={pid}"));

    let block_align = (PROCESS_CHANNELS as usize) * 2;
    let mut queue: VecDeque<u8> = VecDeque::new();
    let min_chunk = block_align * 256;

    while running.load(Ordering::SeqCst) {
        capture_client.read_from_device_to_deque(&mut queue)?;
        while queue.len() >= min_chunk {
            let chunk: Vec<u8> = queue.drain(..min_chunk).collect();
            let mut samples =
                pcm::bytes_to_pcm16(&chunk, SampleType::Int, 16, PROCESS_CHANNELS);
            if samples.is_empty() {
                continue;
            }
            let raw_peak = normalize_pcm16(&mut samples, 12_000, gain_max);
            diagnostics.record_frame(raw_peak);
            pcm::send_frame(&tx, samples, PROCESS_RATE);
        }
        if event.wait_for_event(2000).is_err() {
            break;
        }
    }

    let _ = audio_client.stop_stream();
    Ok(())
}
