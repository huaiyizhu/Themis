//! Post-STT text fixes for common misrecognitions (e.g. 「RAG」→「Reg」).

use regex::Regex;
use std::sync::LazyLock;

static DEFAULT_CORRECTIONS: &[(&str, &str)] = &[
    ("Reg", "RAG"),
    ("REG", "RAG"),
    ("Regs", "RAG"),
    ("regs", "RAG"),
    // LLM / GPT style slips
    ("L L M", "LLM"),
    ("G P T", "GPT"),
    // Common Chinese homophones for tech terms (STT mishears)
    ("拉格", "RAG"),
    ("瑞格", "RAG"),
    ("雷格", "RAG"),
    ("艾格", "RAG"),
];

static ENV_CORRECTIONS: LazyLock<Vec<(String, String)>> = LazyLock::new(load_env_corrections);

fn load_env_corrections() -> Vec<(String, String)> {
    let raw = match std::env::var("AZURE_SPEECH_CORRECTIONS") {
        Ok(v) if !v.trim().is_empty() => v,
        _ => return Vec::new(),
    };
    let mut out = Vec::new();
    for part in raw.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if let Some((from, to)) = part.split_once(':') {
            let from = from.trim();
            let to = to.trim();
            if !from.is_empty() && !to.is_empty() {
                out.push((from.to_string(), to.to_string()));
            }
        }
    }
    out
}

/// Apply built-in + env word-boundary replacements after Azure STT.
pub fn apply_transcript_fixup(text: &str) -> String {
    let mut out = text.to_string();
    for (from, to) in DEFAULT_CORRECTIONS {
        out = replace_token(&out, from, to);
    }
    for (from, to) in ENV_CORRECTIONS.iter() {
        out = replace_token(&out, from, to);
    }
    out
}

fn replace_token(text: &str, from: &str, to: &str) -> String {
    if from.is_empty() || from == to {
        return text.to_string();
    }
    if from.is_ascii() {
        return replace_ascii_word(text, from, to);
    }
    text.replace(from, to)
}

fn replace_ascii_word(text: &str, from: &str, to: &str) -> String {
    if from.is_empty() || from == to {
        return text.to_string();
    }
    let pattern = format!(r"(?i)\b{}\b", regex::escape(from));
    let Ok(re) = Regex::new(&pattern) else {
        return text.to_string();
    };
    re.replace_all(text, to).into_owned()
}

/// Extra bias: if sentence clearly discusses AI/knowledge but says 「Reg」, fix even without word boundary issues.
pub fn apply_contextual_fixup(text: &str) -> String {
    let mut out = apply_transcript_fixup(text);
    let lower = out.to_lowercase();
    let ai_context = lower.contains("ai")
        || out.contains("人工智能")
        || out.contains("知识")
        || out.contains("资料")
        || out.contains("检索")
        || out.contains("嵌入")
        || out.contains("向量")
        || out.contains("大模型")
        || out.contains("LLM")
        || out.contains("GPT");
    if ai_context {
        // Punctuation after Reg (lookahead not available in default regex)
        for (from, to) in [
            ("Reg，", "RAG，"),
            ("Reg,", "RAG,"),
            ("Reg、", "RAG、"),
            ("Reg。", "RAG。"),
            ("Reg ", "RAG "),
        ] {
            out = out.replace(from, to);
            out = out.replace(&from.to_lowercase(), &to.to_lowercase());
        }
        if out.ends_with("Reg") {
            out.truncate(out.len() - 3);
            out.push_str("RAG");
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reg_to_rag_with_ai_context() {
        let s = apply_contextual_fixup("Reg, 让AI。");
        assert!(s.contains("RAG"), "got: {s}");
        assert!(!s.contains("Reg,"));
    }

    #[test]
    fn reg_to_rag_word_boundary() {
        assert_eq!(apply_transcript_fixup("we use Reg here"), "we use RAG here");
    }

    #[test]
    fn zh_homophone_to_rag() {
        assert_eq!(apply_transcript_fixup("我们讲一下拉格的原理"), "我们讲一下RAG的原理");
    }
}
