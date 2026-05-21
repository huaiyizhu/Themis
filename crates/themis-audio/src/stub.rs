use super::AudioSource;
use themis_core::AudioFrame;
use tokio::sync::mpsc;
use tracing::info;

pub struct StubAudioSource {
    sample_rate: u32,
    channels: u16,
    running: bool,
}

impl StubAudioSource {
    pub fn new(sample_rate: u32, channels: u16) -> Self {
        Self {
            sample_rate,
            channels,
            running: false,
        }
    }
}

impl AudioSource for StubAudioSource {
    fn start(&mut self, tx: mpsc::Sender<AudioFrame>) -> anyhow::Result<()> {
        if self.running {
            return Ok(());
        }
        self.running = true;
        let sample_rate = self.sample_rate;
        let channels = self.channels;

        tokio::spawn(async move {
            let mut phase = 0.0f32;
            loop {
                let samples: Vec<i16> = (0..sample_rate as usize / 10)
                    .map(|i| {
                        let v = (phase + i as f32 * 0.01).sin() * 3000.0;
                        v as i16
                    })
                    .collect();
                phase += 1.0;
                if tx
                    .send(AudioFrame::new(samples, sample_rate, channels))
                    .await
                    .is_err()
                {
                    break;
                }
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        });

        info!("stub audio source started");
        Ok(())
    }

    fn stop(&mut self) -> anyhow::Result<()> {
        self.running = false;
        Ok(())
    }
}
