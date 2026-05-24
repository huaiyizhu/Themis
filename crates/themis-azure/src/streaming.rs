//! Azure Speech continuous recognition over WebSocket (conversation API).

use crate::{SpeechEvent, SpeechRecognizer};
use async_trait::async_trait;
use futures::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, Mutex};
use tokio_tungstenite::{
    connect_async,
    tungstenite::{client::IntoClientRequest, http::HeaderValue, Message},
};
use tracing::{debug, error, info, warn};

const STT_SAMPLE_RATE: u32 = 16_000;

pub struct AzureStreamingRecognizer {
    key: String,
    region: String,
    language: String,
    tx: broadcast::Sender<SpeechEvent>,
    audio_tx: Arc<Mutex<Option<mpsc::Sender<Vec<i16>>>>>,
    running: Arc<Mutex<bool>>,
}

impl AzureStreamingRecognizer {
    pub fn new(key: String, region: String, language: String) -> Self {
        let (tx, _) = broadcast::channel(256);
        Self {
            key,
            region,
            language,
            tx,
            audio_tx: Arc::new(Mutex::new(None)),
            running: Arc::new(Mutex::new(false)),
        }
    }

    fn ws_url(region: &str, language: &str) -> String {
        format!(
            "wss://{}.stt.speech.microsoft.com/speech/recognition/conversation/cognitiveservices/v1?language={}&format=detailed",
            region, language
        )
    }

    fn pcm_to_bytes(pcm: &[i16]) -> Vec<u8> {
        pcm.iter().flat_map(|s| s.to_le_bytes()).collect()
    }

    fn parse_ws_payload(data: &[u8]) -> Option<String> {
        if data.is_empty() {
            return None;
        }
        // Text JSON frame
        if data[0] == b'{' {
            return String::from_utf8(data.to_vec()).ok();
        }
        // Binary frame: 2-byte BE header length + headers + JSON body
        if data.len() >= 2 {
            let header_len = u16::from_be_bytes([data[0], data[1]]) as usize;
            if data.len() > 2 + header_len {
                let body = &data[2 + header_len..];
                if body.first() == Some(&b'{') {
                    return String::from_utf8(body.to_vec()).ok();
                }
            }
        }
        String::from_utf8(data.to_vec()).ok()
    }

    fn emit_from_payload(text: &str, tx: &broadcast::Sender<SpeechEvent>) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(text) {
            if let Some(status) = json.get("RecognitionStatus").and_then(|v| v.as_str()) {
                match status {
                    "InitialSilenceTimeout" | "NoMatch" | "BabbleTimeout" => return,
                    other => debug!(status = other, "azure recognition status"),
                }
            }

            let phrase = json
                .get("DisplayText")
                .and_then(|v| v.as_str())
                .or_else(|| {
                    json.get("NBest")
                        .and_then(|v| v.get(0))
                        .and_then(|b| b.get("Display").and_then(|v| v.as_str()))
                })
                .or_else(|| json.get("Text").and_then(|v| v.as_str()))
                .filter(|t| !t.is_empty());

            if let Some(phrase) = phrase {
                let is_final = json
                    .get("RecognitionStatus")
                    .and_then(|v| v.as_str())
                    .map(|s| s == "Success")
                    .unwrap_or_else(|| json.get("Text").is_none());
                let _ = tx.send(SpeechEvent {
                    text: phrase.to_string(),
                    is_final,
                    latency: None,
                });
            }
        }
    }

    fn handle_ws_message(msg: Message, tx: &broadcast::Sender<SpeechEvent>) {
        match msg {
            Message::Text(text) => {
                debug!(%text, "azure ws text");
                Self::emit_from_payload(&text, tx);
            }
            Message::Binary(bin) => {
                if let Some(text) = Self::parse_ws_payload(&bin) {
                    debug!(%text, "azure ws binary-as-json");
                    Self::emit_from_payload(&text, tx);
                }
            }
            Message::Close(frame) => {
                warn!(?frame, "azure ws closed");
            }
            _ => {}
        }
    }
}

#[async_trait]
impl SpeechRecognizer for AzureStreamingRecognizer {
    async fn start(&mut self) -> anyhow::Result<()> {
        *self.running.lock().await = true;
        let (audio_tx, audio_rx) = mpsc::channel::<Vec<i16>>(512);
        *self.audio_tx.lock().await = Some(audio_tx);

        let key = self.key.clone();
        let region = self.region.clone();
        let language = self.language.clone();
        let tx = self.tx.clone();
        let running = Arc::clone(&self.running);

        let _ = self.tx.send(SpeechEvent {
            text: format!("Azure streaming ({language}) connected…"),
            is_final: false,
            latency: None,
        });

        tokio::spawn(async move {
            if let Err(e) =
                run_ws_session(key, region, language, tx.clone(), audio_rx, running).await
            {
                error!(error = %e, "azure streaming session ended");
                let _ = tx.send(SpeechEvent {
                    text: format!(
                        "Streaming failed ({e}). Try AZURE_SPEECH_MODE=rest in .env and restart."
                    ),
                    is_final: true,
                    latency: None,
                });
            }
        });

        Ok(())
    }

    async fn stop(&mut self) -> anyhow::Result<()> {
        *self.running.lock().await = false;
        *self.audio_tx.lock().await = None;
        Ok(())
    }

    async fn push_audio(&mut self, pcm16: &[i16]) -> anyhow::Result<()> {
        if !*self.running.lock().await || pcm16.is_empty() {
            return Ok(());
        }
        if let Some(tx) = self.audio_tx.lock().await.as_ref() {
            let _ = tx.send(pcm16.to_vec()).await;
        }
        Ok(())
    }

    fn subscribe(&self) -> broadcast::Receiver<SpeechEvent> {
        self.tx.subscribe()
    }
}

async fn run_ws_session(
    key: String,
    region: String,
    language: String,
    tx: broadcast::Sender<SpeechEvent>,
    mut audio_rx: mpsc::Receiver<Vec<i16>>,
    running: Arc<Mutex<bool>>,
) -> anyhow::Result<()> {
    let url = AzureStreamingRecognizer::ws_url(&region, &language);
    info!(%url, "connecting azure speech websocket");
    let mut request = url.clone().into_client_request()?;
    request
        .headers_mut()
        .insert("Ocp-Apim-Subscription-Key", HeaderValue::from_str(&key)?);

    let (ws, _) = connect_async(request).await?;
    info!("azure speech websocket connected");
    let (mut write, mut read) = ws.split();

    const FRAME_SAMPLES: usize = (STT_SAMPLE_RATE as usize) / 5;

    let reader_tx = tx.clone();
    let reader = tokio::spawn(async move {
        while let Some(msg) = read.next().await {
            match msg {
                Ok(m) => AzureStreamingRecognizer::handle_ws_message(m, &reader_tx),
                Err(e) => {
                    warn!(error = %e, "websocket read error");
                    break;
                }
            }
        }
    });

    while *running.lock().await {
        match tokio::time::timeout(std::time::Duration::from_millis(250), audio_rx.recv()).await {
            Ok(Some(pcm)) => {
                for chunk in pcm.chunks(FRAME_SAMPLES) {
                    let bytes = AzureStreamingRecognizer::pcm_to_bytes(chunk);
                    // Conversation endpoint: raw PCM16 LE frames (no SDK header wrapper).
                    if write.send(Message::Binary(bytes.into())).await.is_err() {
                        break;
                    }
                }
            }
            Ok(None) => break,
            Err(_) => continue,
        }
    }

    let _ = write.send(Message::Close(None)).await;
    reader.abort();
    Ok(())
}
