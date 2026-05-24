//! Azure Speech recognition via REST with overlapping chunks.

use crate::recognition;
use crate::{SpeechEvent, SpeechRecognizer};
use async_trait::async_trait;
use std::collections::VecDeque;
use std::sync::Arc;
use themis_core::LatencyBreakdown;
use tokio::sync::{broadcast, Mutex};
use tracing::{debug, warn};

const CHUNK_SAMPLES: usize = 16_000 * 4;
const OVERLAP_SAMPLES: usize = 16_000;

pub struct AzureRestRecognizer {
    key: String,
    region: String,
    language: String,
    tx: broadcast::Sender<SpeechEvent>,
    buffer: Arc<Mutex<VecDeque<i16>>>,
    running: Arc<Mutex<bool>>,
    client: reqwest::Client,
}

impl AzureRestRecognizer {
    pub fn new(key: String, region: String, language: String) -> Self {
        let (tx, _) = broadcast::channel(64);
        Self {
            key,
            region,
            language,
            tx,
            buffer: Arc::new(Mutex::new(VecDeque::new())),
            running: Arc::new(Mutex::new(false)),
            client: reqwest::Client::new(),
        }
    }

    pub async fn recognize_pcm(&self, pcm: Vec<i16>) -> anyhow::Result<Option<String>> {
        Ok(self.recognize_chunk(pcm).await?.map(|(r, _)| r.text))
    }

    async fn recognize_chunk(
        &self,
        pcm: Vec<i16>,
    ) -> anyhow::Result<Option<(recognition::ParsedRecognition, LatencyBreakdown)>> {
        let buffer_ms = (CHUNK_SAMPLES as u32 * 1000) / 16_000;
        let (parsed, azure_ms) =
            recognition::recognize_pcm(&self.client, &self.key, &self.region, &self.language, &pcm)
                .await?;
        let Some(parsed) = parsed else {
            return Ok(None);
        };
        let breakdown = LatencyBreakdown {
            buffer_ms,
            azure_ms,
            stt_wall_ms: azure_ms,
            estimated_e2e_ms: buffer_ms.saturating_add(azure_ms),
            language: self.language.clone(),
        };
        Ok(Some((parsed, breakdown)))
    }
}

#[async_trait]
impl SpeechRecognizer for AzureRestRecognizer {
    async fn start(&mut self) -> anyhow::Result<()> {
        *self.running.lock().await = true;
        let _ = self.tx.send(SpeechEvent {
            text: format!(
                "Azure REST ({}) — transcribing every ~4s",
                self.language
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
                    debug!("azure rest: no speech in chunk");
                }
                Err(e) => {
                    warn!(error = %e, "azure rest recognition failed");
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

impl AzureRestRecognizer {
    fn clone_inner(&self) -> Self {
        Self {
            key: self.key.clone(),
            region: self.region.clone(),
            language: self.language.clone(),
            tx: self.tx.clone(),
            buffer: Arc::clone(&self.buffer),
            running: Arc::clone(&self.running),
            client: self.client.clone(),
        }
    }
}

pub async fn check_connectivity(key: &str, region: &str) -> anyhow::Result<()> {
    let url = format!(
        "https://{}.api.cognitive.microsoft.com/sts/v1.0/issueToken",
        region
    );
    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .header("Ocp-Apim-Subscription-Key", key)
        .send()
        .await?;
    if resp.status().is_success() {
        debug!("Azure Speech token endpoint OK");
        Ok(())
    } else {
        anyhow::bail!("Azure connectivity check failed: {}", resp.status())
    }
}
