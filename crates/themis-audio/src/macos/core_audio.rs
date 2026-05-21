//! macOS system audio capture.
//!
//! Native output tap requires additional entitlements. This implementation uses
//! cpal with the default input device. For full system-audio loopback, install
//! [BlackHole](https://existential.audio/blackhole/) and select it as input — see
//! `docs/platform-notes.md`.

use crate::stub::StubAudioSource;
use crate::AudioSource;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, SampleRate, Stream};
use std::sync::OnceLock;
use themis_core::AudioFrame;
use tokio::sync::mpsc;
use tracing::{info, warn};

static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();

fn runtime() -> &'static tokio::runtime::Runtime {
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .worker_threads(1)
            .build()
            .expect("tokio runtime")
    })
}

fn send_frame(tx: &mpsc::Sender<AudioFrame>, samples: Vec<i16>, rate: u32, channels: u16) {
    let frame = AudioFrame::new(samples, rate, channels);
    let tx = tx.clone();
    runtime().spawn(async move {
        let _ = tx.send(frame).await;
    });
}

pub struct CoreAudioLoopback {
    sample_rate: u32,
    channels: u16,
    stream: Option<Stream>,
    use_stub: bool,
    stub: Option<StubAudioSource>,
}

impl CoreAudioLoopback {
    pub fn new(sample_rate: u32, channels: u16) -> anyhow::Result<Self> {
        Ok(Self {
            sample_rate,
            channels,
            stream: None,
            use_stub: false,
            stub: None,
        })
    }
}

impl AudioSource for CoreAudioLoopback {
    fn start(&mut self, tx: mpsc::Sender<AudioFrame>) -> anyhow::Result<()> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .or_else(|| {
                warn!("no input device; falling back to stub audio source");
                None
            });

        let Some(device) = device else {
            self.use_stub = true;
            let mut stub = crate::stub::StubAudioSource::new(self.sample_rate, self.channels);
            stub.start(tx)?;
            self.stub = Some(stub);
            return Ok(());
        };

        let config = device.default_input_config()?;
        let sample_format = config.sample_format();
        let stream_config = cpal::StreamConfig {
            channels: config.channels(),
            sample_rate: SampleRate(self.sample_rate),
            buffer_size: cpal::BufferSize::Default,
        };

        let target_rate = self.sample_rate;
        let target_channels = self.channels;
        let err_fn = |e| tracing::error!(error = %e, "cpal macOS stream error");

        let stream = match sample_format {
            SampleFormat::F32 => device.build_input_stream(
                &stream_config,
                move |data: &[f32], _| {
                    let samples: Vec<i16> = data
                        .iter()
                        .map(|&s| (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)
                        .collect();
                    send_frame(&tx, samples, target_rate, target_channels);
                },
                err_fn,
                None,
            )?,
            SampleFormat::I16 => device.build_input_stream(
                &stream_config,
                move |data: &[i16], _| {
                    send_frame(&tx, data.to_vec(), target_rate, target_channels);
                },
                err_fn,
                None,
            )?,
            _ => anyhow::bail!("unsupported sample format"),
        };

        stream.play()?;
        self.stream = Some(stream);
        info!("macOS cpal input capture started");
        Ok(())
    }

    fn stop(&mut self) -> anyhow::Result<()> {
        self.stream = None;
        if let Some(mut stub) = self.stub.take() {
            stub.stop()?;
        }
        Ok(())
    }
}
