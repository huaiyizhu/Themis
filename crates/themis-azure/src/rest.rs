//! Azure Speech recognition via REST with longer overlapping chunks.
//! Fallback when WebSocket streaming is unavailable.

use crate::{SpeechEvent, SpeechRecognizer};
use async_trait::async_trait;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};
use tracing::{debug, warn};

/// ~4 seconds at 16 kHz mono — balance latency vs sentence context.
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

    fn wav_bytes(pcm: &[i16], sample_rate: u32) -> Vec<u8> {
        let data_size = (pcm.len() * 2) as u32;
        let mut wav = Vec::with_capacity(44 + data_size as usize);
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&(36 + data_size).to_le_bytes());
        wav.extend_from_slice(b"WAVEfmt ");
        wav.extend_from_slice(&16u32.to_le_bytes());
        wav.extend_from_slice(&1u16.to_le_bytes());
        wav.extend_from_slice(&1u16.to_le_bytes());
        wav.extend_from_slice(&sample_rate.to_le_bytes());
        wav.extend_from_slice(&(sample_rate * 2).to_le_bytes());
        wav.extend_from_slice(&2u16.to_le_bytes());
        wav.extend_from_slice(&16u16.to_le_bytes());
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&data_size.to_le_bytes());
        for &s in pcm {
            wav.extend_from_slice(&s.to_le_bytes());
        }
        wav
    }

    fn parse_response(json: &serde_json::Value) -> Option<String> {
        if let Some(t) = json.get("DisplayText").and_then(|v| v.as_str()) {
            if !t.is_empty() {
                return Some(t.to_string());
            }
        }
        json.get("NBest")
            .and_then(|v| v.get(0))
            .and_then(|b| b.get("Display").and_then(|v| v.as_str()))
            .filter(|t| !t.is_empty())
            .map(|s| s.to_string())
    }

    /// One-shot recognition (used by CLI probe).
    pub async fn recognize_pcm(&self, pcm: Vec<i16>) -> anyhow::Result<Option<String>> {
        self.recognize_chunk(pcm).await
    }

    async fn recognize_chunk(&self, pcm: Vec<i16>) -> anyhow::Result<Option<String>> {
        let url = format!(
            "https://{}.stt.speech.microsoft.com/speech/recognition/dictation/cognitiveservices/v1?language={}&format=detailed&punctuation=implicit",
            self.region, self.language
        );
        let wav = Self::wav_bytes(&pcm, 16_000);
        let resp = self
            .client
            .post(&url)
            .header("Ocp-Apim-Subscription-Key", &self.key)
            .header("Content-Type", "audio/wav; codecs=audio/pcm; samplerate=16000")
            .body(wav)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Azure Speech HTTP {status}: {body}");
        }

        let json: serde_json::Value = resp.json().await?;
        Ok(Self::parse_response(&json))
    }
}

#[async_trait]
impl SpeechRecognizer for AzureRestRecognizer {
    async fn start(&mut self) -> anyhow::Result<()> {
        *self.running.lock().await = true;
        let _ = self.tx.send(SpeechEvent {
            text: format!(
                "Azure REST ({}) — transcribing every ~4s (set AZURE_SPEECH_MODE=streaming to experiment)",
                self.language
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
                Ok(Some(text)) => {
                    let _ = this.tx.send(SpeechEvent {
                        text: text.clone(),
                        is_final: false,
                    });
                    let _ = this.tx.send(SpeechEvent {
                        text,
                        is_final: true,
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
