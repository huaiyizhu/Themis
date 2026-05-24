use std::sync::OnceLock;
use themis_core::AudioFrame;
use tokio::sync::mpsc;
use tracing::warn;
use wasapi::SampleType;

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

pub fn bytes_to_pcm16(
    bytes: &[u8],
    sample_type: SampleType,
    bits_per_sample: u16,
    channels: u16,
) -> Vec<i16> {
    let mut out = Vec::new();
    match (sample_type, bits_per_sample) {
        (SampleType::Float, 32) => {
            for frame in bytes.chunks_exact(4) {
                let f = f32::from_le_bytes([frame[0], frame[1], frame[2], frame[3]]);
                let s = (f.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
                out.push(s);
            }
        }
        (SampleType::Int, 16) => {
            for frame in bytes.chunks_exact(2) {
                out.push(i16::from_le_bytes([frame[0], frame[1]]));
            }
        }
        _ => {
            warn!(?sample_type, bits_per_sample, "unsupported sample format");
        }
    }

    if channels > 1 && !out.is_empty() {
        out = out
            .chunks(channels as usize)
            .map(|c| c.iter().map(|&s| s as i32).sum::<i32>() / c.len() as i32)
            .map(|s| s as i16)
            .collect();
    }

    out
}

pub fn send_frame(tx: &mpsc::Sender<AudioFrame>, samples: Vec<i16>, rate: u32) {
    let frame = AudioFrame::new(samples, rate, 1);
    let tx = tx.clone();
    runtime().spawn(async move {
        let _ = tx.send(frame).await;
    });
}
