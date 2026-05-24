//! Shared Azure Speech REST dictation helpers.

use tracing::debug;

#[derive(Debug, Clone)]
pub struct ParsedRecognition {
    pub text: String,
    pub confidence: f32,
    pub language: String,
}

pub fn wav_bytes(pcm: &[i16], sample_rate: u32) -> Vec<u8> {
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

pub fn parse_detailed(json: &serde_json::Value, language: &str) -> Option<ParsedRecognition> {
    let status = json.get("RecognitionStatus")?.as_str()?;
    if status != "Success" {
        return None;
    }

    let nbest = json.get("NBest")?.get(0)?;
    let text = nbest
        .get("Display")
        .and_then(|v| v.as_str())
        .or_else(|| json.get("DisplayText").and_then(|v| v.as_str()))
        .filter(|t| !t.is_empty())?;

    let confidence = nbest
        .get("Confidence")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.5) as f32;

    Some(ParsedRecognition {
        text: text.to_string(),
        confidence,
        language: language.to_string(),
    })
}

pub async fn recognize_pcm(
    client: &reqwest::Client,
    key: &str,
    region: &str,
    language: &str,
    pcm: &[i16],
) -> anyhow::Result<Option<ParsedRecognition>> {
    let url = format!(
        "https://{region}.stt.speech.microsoft.com/speech/recognition/dictation/cognitiveservices/v1?language={language}&format=detailed&punctuation=implicit"
    );
    let wav = wav_bytes(pcm, 16_000);
    let resp = client
        .post(&url)
        .header("Ocp-Apim-Subscription-Key", key)
        .header("Content-Type", "audio/wav; codecs=audio/pcm; samplerate=16000")
        .body(wav)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("Azure Speech HTTP {status} ({language}): {body}");
    }

    let json: serde_json::Value = resp.json().await?;
    Ok(parse_detailed(&json, language))
}

pub fn pick_best(results: Vec<Option<ParsedRecognition>>) -> Option<ParsedRecognition> {
    let best = results
        .into_iter()
        .flatten()
        .max_by(|a, b| {
            a.confidence
                .partial_cmp(&b.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    if let Some(ref b) = best {
        debug!(
            language = %b.language,
            confidence = b.confidence,
            "auto language pick"
        );
    }
    best
}
