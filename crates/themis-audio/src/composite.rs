//! Mix multiple [`AudioSource`] streams into one mono PCM feed.

use crate::AudioSource;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;
use themis_core::{AudioFrame, CaptureDiagnostics};
use tokio::sync::mpsc;
use tracing::info;

fn mix_many(samples: &[i16]) -> i16 {
    samples
        .iter()
        .fold(0i32, |acc, &s| acc + s as i32)
        .clamp(i16::MIN as i32, i16::MAX as i32) as i16
}

fn frame_peak(samples: &[i16]) -> u32 {
    samples
        .iter()
        .map(|&s| u32::from(s.unsigned_abs()))
        .max()
        .unwrap_or(0)
}

pub struct CompositeAudioSource {
    sources: Vec<Box<dyn AudioSource + Send>>,
    mode_label: String,
    detail: String,
    sample_rate: u32,
    channels: u16,
    diagnostics: Option<Arc<CaptureDiagnostics>>,
    running: Arc<AtomicBool>,
    mixer: Option<JoinHandle<()>>,
}

impl CompositeAudioSource {
    pub fn new(
        sources: Vec<Box<dyn AudioSource + Send>>,
        mode_label: impl Into<String>,
        detail: impl Into<String>,
        sample_rate: u32,
        channels: u16,
        diagnostics: Option<Arc<CaptureDiagnostics>>,
    ) -> Self {
        Self {
            sources,
            mode_label: mode_label.into(),
            detail: detail.into(),
            sample_rate,
            channels,
            diagnostics,
            running: Arc::new(AtomicBool::new(false)),
            mixer: None,
        }
    }
}

impl AudioSource for CompositeAudioSource {
    fn start(&mut self, tx: mpsc::Sender<AudioFrame>) -> anyhow::Result<()> {
        if self.running.load(Ordering::SeqCst) {
            return Ok(());
        }

        let stream_count = self.sources.len();
        if stream_count == 0 {
            anyhow::bail!("composite source has no child streams");
        }

        if let Some(d) = &self.diagnostics {
            d.set_mode(&self.mode_label);
            d.set_detail(self.detail.clone());
            d.set_sessions(stream_count as u32);
        }

        let mut receivers = Vec::with_capacity(stream_count);
        for source in &mut self.sources {
            let (itx, irx) = mpsc::channel(512);
            source.start(itx)?;
            receivers.push(irx);
        }

        let running = Arc::clone(&self.running);
        running.store(true, Ordering::SeqCst);

        let target_rate = self.sample_rate;
        let target_channels = self.channels;
        let diagnostics = self.diagnostics.clone();

        let handle = thread::spawn(move || {
            let mut queues: Vec<VecDeque<i16>> = (0..stream_count).map(|_| VecDeque::new()).collect();
            const EMIT_SAMPLES: usize = 320;

            while running.load(Ordering::SeqCst) {
                for (rx, queue) in receivers.iter_mut().zip(queues.iter_mut()) {
                    while let Ok(frame) = rx.try_recv() {
                        let mono = frame.to_mono_pcm16(target_rate);
                        queue.extend(mono);
                    }
                }

                while queues.iter().any(|q| q.len() >= EMIT_SAMPLES) {
                    let mut chunk = Vec::with_capacity(EMIT_SAMPLES);
                    for _ in 0..EMIT_SAMPLES {
                        let samples: Vec<i16> = queues
                            .iter_mut()
                            .map(|q| q.pop_front().unwrap_or(0))
                            .collect();
                        chunk.push(if samples.len() == 1 {
                            samples[0]
                        } else {
                            mix_many(&samples)
                        });
                    }

                    if let Some(d) = &diagnostics {
                        d.record_frame(frame_peak(&chunk));
                    }

                    let frame = AudioFrame::new(chunk, target_rate, target_channels);
                    if tx.blocking_send(frame).is_err() {
                        return;
                    }
                }

                thread::sleep(Duration::from_millis(5));
            }
        });

        self.mixer = Some(handle);
        info!(
            mode = %self.mode_label,
            streams = stream_count,
            %self.detail,
            "composite audio capture started"
        );
        Ok(())
    }

    fn stop(&mut self) -> anyhow::Result<()> {
        self.running.store(false, Ordering::SeqCst);
        if let Some(h) = self.mixer.take() {
            let _ = h.join();
        }
        for source in &mut self.sources {
            source.stop()?;
        }
        Ok(())
    }
}
