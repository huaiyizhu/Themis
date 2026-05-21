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

pub struct WasapiLoopback {
    sample_rate: u32,
    channels: u16,
    stream: Option<Stream>,
    fallback: Option<StubAudioSource>,
}

impl WasapiLoopback {
    pub fn new(sample_rate: u32, channels: u16) -> anyhow::Result<Self> {
        Ok(Self {
            sample_rate,
            channels,
            stream: None,
            fallback: None,
        })
    }
}

impl AudioSource for WasapiLoopback {
    fn start(&mut self, tx: mpsc::Sender<AudioFrame>) -> anyhow::Result<()> {
        match try_start_loopback(self.sample_rate, self.channels, tx.clone()) {
            Ok(stream) => {
                self.stream = Some(stream);
                info!("WASAPI loopback capture started");
                Ok(())
            }
            Err(e) => {
                warn!(error = %e, "loopback unavailable; using stub source");
                let mut stub = StubAudioSource::new(self.sample_rate, self.channels);
                stub.start(tx)?;
                self.fallback = Some(stub);
                Ok(())
            }
        }
    }

    fn stop(&mut self) -> anyhow::Result<()> {
        self.stream = None;
        if let Some(mut stub) = self.fallback.take() {
            stub.stop()?;
        }
        Ok(())
    }
}

fn try_start_loopback(
    target_rate: u32,
    target_channels: u16,
    tx: mpsc::Sender<AudioFrame>,
) -> anyhow::Result<Stream> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or_else(|| anyhow::anyhow!("no default output device"))?;

    let output = device.default_output_config()?;
    let sample_format = output.sample_format();
    let stream_config = cpal::StreamConfig {
        channels: output.channels(),
        sample_rate: SampleRate(target_rate.min(output.sample_rate().0)),
        buffer_size: cpal::BufferSize::Default,
    };

    let err_fn = |e| tracing::error!(error = %e, "cpal stream error");

    let stream = match sample_format {
        SampleFormat::F32 => device.build_input_stream(
            &stream_config,
            move |data: &[f32], _| {
                send_frame(&tx, f32_to_i16(data), target_rate, target_channels);
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
        other => anyhow::bail!("unsupported sample format {other:?}"),
    };

    stream.play()?;
    Ok(stream)
}

fn f32_to_i16(data: &[f32]) -> Vec<i16> {
    data.iter()
        .map(|&s| (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)
        .collect()
}

fn send_frame(tx: &mpsc::Sender<AudioFrame>, samples: Vec<i16>, rate: u32, channels: u16) {
    let frame = AudioFrame::new(samples, rate, channels);
    let tx = tx.clone();
    runtime().spawn(async move {
        let _ = tx.send(frame).await;
    });
}
