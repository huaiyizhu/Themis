use crate::{SpeechEvent, SpeechRecognizer};
use async_trait::async_trait;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::broadcast;
use tracing::debug;

pub struct MockSpeechRecognizer {
    tx: broadcast::Sender<SpeechEvent>,
    chunk_count: AtomicU64,
    running: bool,
}

impl MockSpeechRecognizer {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(64);
        Self {
            tx,
            chunk_count: AtomicU64::new(0),
            running: false,
        }
    }
}

#[async_trait]
impl SpeechRecognizer for MockSpeechRecognizer {
    async fn start(&mut self) -> anyhow::Result<()> {
        self.running = true;
        Ok(())
    }

    async fn stop(&mut self) -> anyhow::Result<()> {
        self.running = false;
        Ok(())
    }

    async fn push_audio(&mut self, pcm16: &[i16]) -> anyhow::Result<()> {
        if !self.running || pcm16.is_empty() {
            return Ok(());
        }

        let n = self.chunk_count.fetch_add(1, Ordering::SeqCst);
        if !n.is_multiple_of(50) {
            return Ok(());
        }

        let energy: f64 =
            pcm16.iter().map(|&s| (s as f64).powi(2)).sum::<f64>() / pcm16.len().max(1) as f64;

        let partial = format!("[mock] audio level {:.0}", energy.sqrt());
        let _ = self.tx.send(SpeechEvent {
            text: partial.clone(),
            is_final: false,
        });

        if n.is_multiple_of(150) {
            let _ = self.tx.send(SpeechEvent {
                text: format!("{partial} — mock transcript segment {}", n / 150),
                is_final: true,
            });
        }

        debug!(chunks = n, "mock speech pushed");
        Ok(())
    }

    fn subscribe(&self) -> broadcast::Receiver<SpeechEvent> {
        self.tx.subscribe()
    }
}

impl Default for MockSpeechRecognizer {
    fn default() -> Self {
        Self::new()
    }
}
