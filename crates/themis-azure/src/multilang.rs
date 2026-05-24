//! Azure REST with per-chunk language competition (e.g. en-US vs zh-CN).

use crate::chunk::{self, CHUNK_SAMPLES, OVERLAP_SAMPLES};
use crate::recognition::{self, ParsedRecognition};
use crate::{SpeechEvent, SpeechRecognizer};
use async_trait::async_trait;
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Instant;
use themis_core::LatencyBreakdown;
use tokio::sync::{broadcast, Mutex};
use tracing::{debug, warn};

pub struct AzureMultiLangRestRecognizer {
    key: String,
    region: String,
    languages: Vec<String>,
    tx: broadcast::Sender<SpeechEvent>,
    buffer: Arc<Mutex<VecDeque<i16>>>,
    running: Arc<Mutex<bool>>,
    client: reqwest::Client,
}

impl AzureMultiLangRestRecognizer {
    pub fn new(key: String, region: String, languages: Vec<String>) -> Self {
        let (tx, _) = broadcast::channel(64);
        Self {
            key,
            region,
            languages,
            tx,
            buffer: Arc::new(Mutex::new(VecDeque::new())),
            running: Arc::new(Mutex::new(false)),
            client: reqwest::Client::new(),
        }
    }

    async fn recognize_chunk(
        &self,
        pcm: Vec<i16>,
    ) -> anyhow::Result<Option<(ParsedRecognition, LatencyBreakdown)>> {
        let wall = Instant::now();
        let buffer_ms = chunk::chunk_buffer_ms();
        let mut tasks = Vec::with_capacity(self.languages.len());
        for lang in &self.languages {
            let client = self.client.clone();
            let key = self.key.clone();
            let region = self.region.clone();
            let lang = lang.clone();
            let pcm_ref = pcm.clone();
            tasks.push(tokio::spawn(async move {
                recognition::recognize_pcm(&client, &key, &region, &lang, &pcm_ref).await
            }));
        }

        let mut scored: Vec<(ParsedRecognition, u32)> = Vec::new();
        for t in tasks {
            match t.await {
                Ok(Ok((opt, azure_ms))) => {
                    if let Some(p) = opt {
                        scored.push((p, azure_ms));
                    }
                }
                Ok(Err(e)) => debug!(error = %e, "one language candidate failed"),
                Err(e) => debug!(error = %e, "language task join failed"),
            }
        }

        let Some((best, azure_ms)) = scored.into_iter().max_by(|a, b| {
            a.0.confidence
                .partial_cmp(&b.0.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        }) else {
            return Ok(None);
        };

        let stt_wall_ms = wall.elapsed().as_millis() as u32;
        let breakdown = LatencyBreakdown {
            buffer_ms,
            azure_ms,
            stt_wall_ms,
            estimated_e2e_ms: buffer_ms.saturating_add(azure_ms),
            language: best.language.clone(),
        };
        Ok(Some((best, breakdown)))
    }

    /// One-shot probe (CLI).
    pub async fn recognize_pcm(&self, pcm: Vec<i16>) -> anyhow::Result<Option<String>> {
        Ok(self
            .recognize_chunk(pcm)
            .await?
            .map(|(r, _)| r.text))
    }
}

#[async_trait]
impl SpeechRecognizer for AzureMultiLangRestRecognizer {
    async fn start(&mut self) -> anyhow::Result<()> {
        *self.running.lock().await = true;
        let langs = self.languages.join(", ");
        let _ = self.tx.send(SpeechEvent {
            text: format!(
                "Azure auto-language ({langs}) — picking best match every ~{}s",
                chunk::CHUNK_SECS
            ),
            is_final: false,
            latency: None,
        });
        Ok(())
    }

    async fn stop(&mut self) -> anyhow::Result<()> {
        *self.running.lock().await = false;
        Ok(())
    }

    async fn push_audio(&mut self, pcm16: &[i16]) -> anyhow::Result<()> {
        if !*self.running.lock().await {
            return Ok(());
        }

        let mut buf = self.buffer.lock().await;
        buf.extend(pcm16.iter().copied());

        if buf.len() < CHUNK_SAMPLES {
            return Ok(());
        }

        let chunk: Vec<i16> = buf.drain(..CHUNK_SAMPLES).collect();
        if OVERLAP_SAMPLES > 0 {
            let start = chunk.len().saturating_sub(OVERLAP_SAMPLES);
            for &sample in chunk[start..].iter().rev() {
                buf.push_front(sample);
            }
        }
        drop(buf);

        let this = self.clone_inner();
        tokio::spawn(async move {
            match this.recognize_chunk(chunk).await {
                Ok(Some((result, latency))) => {
                    let _ = this.tx.send(SpeechEvent {
                        text: result.text.clone(),
                        is_final: false,
                        latency: Some(latency.clone()),
                    });
                    let _ = this.tx.send(SpeechEvent {
                        text: result.text,
                        is_final: true,
                        latency: Some(latency),
                    });
                }
                Ok(None) => {
                    debug!("multilang: no speech in chunk");
                }
                Err(e) => {
                    warn!(error = %e, "multilang recognition failed");
                    let _ = this.tx.send(SpeechEvent {
                        text: format!("Azure error: {e}"),
                        is_final: true,
                        latency: None,
                    });
                }
            }
        });

        Ok(())
    }

    fn subscribe(&self) -> broadcast::Receiver<SpeechEvent> {
        self.tx.subscribe()
    }
}

impl AzureMultiLangRestRecognizer {
    fn clone_inner(&self) -> Self {
        Self {
            key: self.key.clone(),
            region: self.region.clone(),
            languages: self.languages.clone(),
            tx: self.tx.clone(),
            buffer: Arc::clone(&self.buffer),
            running: Arc::clone(&self.running),
            client: self.client.clone(),
        }
    }
}
