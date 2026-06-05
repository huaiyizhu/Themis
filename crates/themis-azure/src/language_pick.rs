//! Pick the best STT candidate when multiple Azure language models compete.

use crate::recognition::ParsedRecognition;

fn is_cjk(ch: char) -> bool {
    matches!(
        ch,
        '\u{4e00}'..='\u{9fff}'
            | '\u{3400}'..='\u{4dbf}'
            | '\u{3000}'..='\u{303f}'
            | '\u{ff00}'..='\u{ffef}'
    )
}

fn script_ratios(text: &str) -> (f32, f32) {
    let mut cjk = 0usize;
    let mut latin = 0usize;
    let mut meaningful = 0usize;
    for ch in text.chars() {
        if ch.is_whitespace() || ch.is_ascii_punctuation() {
            continue;
        }
        meaningful += 1;
        if is_cjk(ch) {
            cjk += 1;
        } else if ch.is_ascii_alphabetic() {
            latin += 1;
        }
    }
    if meaningful == 0 {
        return (0.0, 0.0);
    }
    (
        cjk as f32 / meaningful as f32,
        latin as f32 / meaningful as f32,
    )
}

fn is_chinese_language(lang: &str) -> bool {
    lang.trim().to_ascii_lowercase().starts_with("zh")
}

/// Score a candidate using Azure confidence plus script/language alignment.
fn candidate_score(rec: &ParsedRecognition) -> f32 {
    let (cjk, latin) = script_ratios(&rec.text);
    let mut score = rec.confidence;

    if is_chinese_language(&rec.language) {
        score += cjk * 0.45;
        score -= latin * 0.35;
        // Prefer zh when output is clearly Chinese even if confidence is slightly lower.
        if cjk >= 0.35 {
            score += 0.08;
        }
    } else {
        score += latin * 0.25;
        score -= cjk * 0.40;
        // Penalize English model output with no Latin letters (often garbage on Chinese audio).
        if latin < 0.15 && cjk < 0.10 {
            score -= 0.15;
        }
    }

    score.clamp(0.0, 1.5)
}

/// Pick the best recognition among competing language models for one audio chunk.
pub fn pick_best_recognition(
    candidates: Vec<(ParsedRecognition, u32)>,
) -> Option<(ParsedRecognition, u32)> {
    if candidates.is_empty() {
        return None;
    }
    if candidates.len() == 1 {
        return Some(candidates.into_iter().next().expect("one candidate"));
    }

    let best_zh = candidates
        .iter()
        .filter(|(p, _)| is_chinese_language(&p.language))
        .max_by(|a, b| {
            candidate_score(&a.0)
                .partial_cmp(&candidate_score(&b.0))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    let best_en = candidates
        .iter()
        .filter(|(p, _)| !is_chinese_language(&p.language))
        .max_by(|a, b| {
            candidate_score(&a.0)
                .partial_cmp(&candidate_score(&b.0))
                .unwrap_or(std::cmp::Ordering::Equal)
        });

    if let (Some(zh), Some(en)) = (best_zh, best_en) {
        let (zh_cjk, zh_latin) = script_ratios(&zh.0.text);
        let (_, en_latin) = script_ratios(&en.0.text);
        if zh_cjk >= 0.30 && en_latin >= 0.50 {
            let zh_pure = zh_cjk >= 0.92 && zh_latin < 0.08;
            let en_pure = en_latin >= 0.92;
            if zh_pure && en_pure {
                return Some(if en.0.confidence >= zh.0.confidence {
                    en.clone()
                } else {
                    zh.clone()
                });
            }
            if en.0.confidence > zh.0.confidence + 0.15 {
                return Some(en.clone());
            }
            return Some(zh.clone());
        }
    }

    candidates.into_iter().max_by(|a, b| {
        candidate_score(&a.0)
            .partial_cmp(&candidate_score(&b.0))
            .unwrap_or(std::cmp::Ordering::Equal)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cand(lang: &str, text: &str, confidence: f32) -> (ParsedRecognition, u32) {
        (
            ParsedRecognition {
                text: text.into(),
                confidence,
                language: lang.into(),
            },
            100,
        )
    }

    #[test]
    fn prefers_chinese_when_output_is_chinese() {
        let picked = pick_best_recognition(vec![
            cand("en-US", "we need to use rag for this", 0.82),
            cand("zh-CN", "我们需要使用 RAG 来处理这个问题", 0.78),
        ])
        .expect("pick");
        assert_eq!(picked.0.language, "zh-CN");
    }

    #[test]
    fn prefers_english_for_english_speech() {
        let picked = pick_best_recognition(vec![
            cand("en-US", "what is retrieval augmented generation", 0.85),
            cand("zh-CN", "什么是检索增强生成", 0.70),
        ])
        .expect("pick");
        assert_eq!(picked.0.language, "en-US");
    }

    #[test]
    fn rejects_english_gibberish_on_chinese_audio() {
        let picked = pick_best_recognition(vec![
            cand("en-US", "the need to use rag", 0.88),
            cand("zh-CN", "那么 RAG 是什么", 0.75),
        ])
        .expect("pick");
        assert_eq!(picked.0.language, "zh-CN");
    }
}
