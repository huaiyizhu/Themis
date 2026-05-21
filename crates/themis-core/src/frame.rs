use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SampleFormat {
    Pcm16Le,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioFrame {
    pub samples: Vec<i16>,
    pub sample_rate: u32,
    pub channels: u16,
    pub format: SampleFormat,
    pub timestamp: DateTime<Utc>,
}

impl AudioFrame {
    pub fn new(samples: Vec<i16>, sample_rate: u32, channels: u16) -> Self {
        Self {
            samples,
            sample_rate,
            channels,
            format: SampleFormat::Pcm16Le,
            timestamp: Utc::now(),
        }
    }

    pub fn to_mono_pcm16(&self, target_rate: u32) -> Vec<i16> {
        let mono: Vec<i16> = if self.channels <= 1 {
            self.samples.clone()
        } else {
            self.samples
                .chunks(self.channels as usize)
                .map(|c| c.iter().map(|&s| s as i32).sum::<i32>() / c.len() as i32)
                .map(|s| s as i16)
                .collect()
        };

        if self.sample_rate == target_rate {
            mono
        } else {
            resample_linear(&mono, self.sample_rate, target_rate)
        }
    }

    pub fn to_mono_pcm16_bytes(&self, target_rate: u32) -> Vec<u8> {
        self.to_mono_pcm16(target_rate)
            .iter()
            .flat_map(|s| s.to_le_bytes())
            .collect()
    }
}

fn resample_linear(input: &[i16], from_rate: u32, to_rate: u32) -> Vec<i16> {
    if input.is_empty() || from_rate == 0 || to_rate == 0 {
        return Vec::new();
    }
    let ratio = from_rate as f64 / to_rate as f64;
    let out_len = ((input.len() as f64) / ratio).ceil() as usize;
    (0..out_len)
        .map(|i| {
            let src_pos = i as f64 * ratio;
            let idx = src_pos.floor() as usize;
            let frac = src_pos - idx as f64;
            let a = input.get(idx).copied().unwrap_or(0) as f64;
            let b = input.get(idx + 1).copied().unwrap_or(*input.last().unwrap()) as f64;
            (a + (b - a) * frac).round() as i16
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mono_conversion_preserves_length_for_mono_input() {
        let frame = AudioFrame::new(vec![100, -100, 200, -200], 16_000, 1);
        let out = frame.to_mono_pcm16(16_000);
        assert_eq!(out.len(), 4);
    }
}
