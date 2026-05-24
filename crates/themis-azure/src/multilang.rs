//! Azure REST with per-chunk language competition (e.g. en-US vs zh-CN).

use crate::recognition::{self, ParsedRecognition};
use crate::{SpeechEvent, SpeechRecognizer};
use async_trait::async_trait;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};
use tracing::{debug, warn};

const CHUNK_SAMPLES: usize = 16_000 * 4;
const OVERLAP_SAMPLES: usize = 16_000;

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

    async fn recognize_chunk(&self, pcm: Vec<i16>) -> anyhow::Result<Option<ParsedRecognition>> {
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

        let mut parsed = Vec::new();
        for t in tasks {
            match t.await {
                Ok(Ok(opt)) => parsed.push(opt),
                Ok(Err(e)) => {
                    debug!(error = %e, "one language candidate failed");
                    parsed.push(None);
                }
                Err(e) => {
                    debug!(error = %e, "language task join failed");
                    parsed.push(None);
                }
            }
        }

        Ok(recognition::pick_best(parsed))
    }

    /// One-shot probe (CLI).
    pub async fn recognize_pcm(&self, pcm: Vec<i16>) -> anyhow::Result<Option<String>> {
        Ok(self
            .recognize_chunk(pcm)
            .await?
            .map(|r| r.text))
    }
}

#[async_trait]
impl SpeechRecognizer for AzureMultiLangRestRecognizer {
    async fn start(&mut self) -> anyhow::Result<()> {
        *self.running.lock().await = true;
        let langs = self.languages.join(", ");
        let _ = self.tx.send(SpeechEvent {
            text: format!(
                "Azure auto-language ({langs}) — picking best match every ~4s"
            ),
            is_final: false,
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
                Ok(Some(result)) => {
                    let _ = this.tx.send(SpeechEvent {
                        text: result.text.clone(),
                        is_final: false,
                    });
                    let _ = this.tx.send(SpeechEvent {
                        text: result.text,
                        is_final: true,
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
