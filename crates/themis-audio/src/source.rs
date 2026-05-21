use themis_core::AudioFrame;
use tokio::sync::mpsc;

pub trait AudioSource: Send {
    fn start(&mut self, tx: mpsc::Sender<AudioFrame>) -> anyhow::Result<()>;
    fn stop(&mut self) -> anyhow::Result<()>;
}

pub type LoopbackSource = Box<dyn AudioSource + Send>;
