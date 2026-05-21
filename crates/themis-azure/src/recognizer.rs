use async_trait::async_trait;
use tokio::sync::broadcast;

#[derive(Debug, Clone)]
pub struct SpeechEvent {
    pub text: String,
    pub is_final: bool,
}

#[async_trait]
pub trait SpeechRecognizer: Send + Sync {
    async fn start(&mut self) -> anyhow::Result<()>;
    async fn stop(&mut self) -> anyhow::Result<()>;
    async fn push_audio(&mut self, pcm16: &[i16]) -> anyhow::Result<()>;
    fn subscribe(&self) -> broadcast::Receiver<SpeechEvent>;
}
