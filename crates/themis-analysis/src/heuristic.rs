//! Rule-based extraction: glossary, tech-shaped English tokens, substantive questions.

use crate::glossary;
use regex::Regex;
use std::sync::LazyLock;
use themis_core::{AnalysisResult, QuestionInsight, TermInsight};

static ACRONYM_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b[A-Z]{2,8}\b").expect("acronym regex"));
static EN_WORD_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b[A-Za-z][A-Za-z0-9_-]{2,}\b").expect("en word"));
static QUESTION_EN_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[.!?]\s*([^.!?]*\?)").expect("question en"));
static QUESTION_ZH_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[^。！？]*[？?]").expect("question zh"));
/// 「embedding 是什么」「RAG是啥」— 口语常不带问号
static ZH_X_IS_WHAT_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)([A-Za-z][A-Za-z0-9_-]*|[\u4e00-\u9fff]{2,12})\s*(是什么|是啥|什么意思)")
        .expect("zh x是什么")
});
static ZH_WHAT_IS_X_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(什么是|啥是|何谓)\s*([A-Za-z][A-Za-z0-9_-]*|[\u4e00-\u9fff]{2,12})")
        .expect("zh 什么是x")
});
static EN_WHAT_IS_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)what\s+is\s+(?:an?\s+)?([A-Za-z][A-Za-z0-9_-]+)").expect("what is x")
});

/// Common English words that should not become keywords on their own.
static COMMON_WORDS: &[&str] = &[
    "the", "and", "for", "are", "but", "not", "you", "all", "can", "had", "her", "was", "one", "our",
    "out", "day", "get", "has", "him", "his", "how", "its", "may", "new", "now", "old", "see", "two",
    "way", "who", "did", "she", "use", "many", "some", "time", "very", "when", "come", "here", "just",
    "like", "long", "make", "much", "over", "such", "take", "than", "them", "well", "were", "what",
    "with", "your", "from", "have", "this", "that", "will", "they", "been", "each", "which", "their",
    "said", "also", "into", "only", "other", "about", "after", "before", "being", "between", "both",
    "could", "first", "more", "most", "should", "these", "those", "through", "under", "where", "while",
    "would", "there", "then", "than", "because", "during", "without", "within", "against", "again",
    "game", "games", "team", "teams", "tonight", "today", "tomorrow", "yesterday", "week", "month",
    "year", "people", "person", "thing", "things", "good", "great", "best", "bad", "nice",
    "really", "very", "think", "know", "want", "need", "look", "looking", "talk", "talking", "said",
    "say", "says", "tell", "told", "going", "went", "come", "came", "right", "left", "back", "next",
];

pub fn analyze_heuristic(transcript: &str, localize_zh: bool) -> AnalysisResult {
    let text = transcript.trim();
    if text.is_empty() {
        return AnalysisResult::default();
    }

    let mut result = AnalysisResult::default();

    for cap in ACRONYM_RE.captures_iter(text) {
        if let Some(m) = cap.get(0) {
            add_glossary_hit(&mut result, m.as_str(), localize_zh);
        }
    }

    for cap in EN_WORD_RE.captures_iter(text) {
        if let Some(m) = cap.get(0) {
            let word = m.as_str();
            add_glossary_hit(&mut result, word, localize_zh);
            if is_tech_shaped(word) && !is_common_word(word) {
                push_keyword(&mut result, word);
            }
        }
    }

    for q in extract_questions(text) {
        if is_substantive_question(&q) {
            attach_question(&mut result, &q, localize_zh);
        }
    }

    result.keywords.truncate(10);
    result.terms.truncate(8);
    result.questions.truncate(4);
    result
}

fn is_common_word(word: &str) -> bool {
    let lower = word.to_lowercase();
    COMMON_WORDS.iter().any(|w| *w == lower.as_str())
}

/// English token looks like a technical identifier (not plain dictionary prose).
fn is_tech_shaped(word: &str) -> bool {
    if word.len() < 3 {
        return false;
    }
    if word.contains('-') || word.contains('_') {
        return true;
    }
    if word.chars().any(|c| c.is_ascii_digit()) {
        return true;
    }
    let has_lower = word.chars().any(|c| c.is_ascii_lowercase());
    let has_upper = word.chars().any(|c| c.is_ascii_uppercase());
    // CamelCase / PascalCase, e.g. ChatGPT, PyTorch
    has_lower && has_upper
}

fn add_glossary_hit(result: &mut AnalysisResult, raw: &str, localize_zh: bool) {
    let Some((display, explanation)) = glossary::lookup(raw) else {
        return;
    };
    let explanation = if localize_zh {
        explanation.to_string()
    } else {
        format!("Technical term: {display}.")
    };
    if !result
        .terms
        .iter()
        .any(|t| t.term.eq_ignore_ascii_case(display))
    {
        result.terms.push(TermInsight {
            term: display.to_string(),
            explanation,
        });
    }
    push_keyword(result, display);
}

fn push_keyword(result: &mut AnalysisResult, word: &str) {
    let w = word.trim();
    if w.len() < 2 {
        return;
    }
    if !result.keywords.iter().any(|k| k.eq_ignore_ascii_case(w)) {
        result.keywords.push(w.to_string());
    }
}

fn attach_question(result: &mut AnalysisResult, question: &str, localize_zh: bool) {
    let q = question.trim().to_string();
    if q.len() < 4 {
        return;
    }
    if result.questions.iter().any(|x| x.question == q) {
        return;
    }

    let answer = answer_for_question(&q, localize_zh);
    result.questions.push(QuestionInsight {
        question: q.clone(),
        answer,
    });

    // Pull subject term from 「X是什么」/ what is X
    if let Some(subject) = subject_from_question(&q) {
        add_glossary_hit(result, &subject, localize_zh);
    }
}

fn subject_from_question(q: &str) -> Option<String> {
    if let Some(cap) = ZH_X_IS_WHAT_RE.captures(q) {
        return cap.get(1).map(|m| m.as_str().to_string());
    }
    if let Some(cap) = ZH_WHAT_IS_X_RE.captures(q) {
        return cap.get(2).map(|m| m.as_str().to_string());
    }
    if let Some(cap) = EN_WHAT_IS_RE.captures(q) {
        return cap.get(1).map(|m| m.as_str().to_string());
    }
    None
}

fn contains_technical_signal(text: &str) -> bool {
    if is_tech_shaped(text) {
        return true;
    }
    for cap in EN_WORD_RE.captures_iter(text) {
        if let Some(m) = cap.get(0) {
            let w = m.as_str();
            if glossary::lookup(w).is_some() {
                return true;
            }
            if is_tech_shaped(w) && !is_common_word(w) {
                return true;
            }
        }
    }
    for cap in ACRONYM_RE.captures_iter(text) {
        if let Some(m) = cap.get(0) {
            if glossary::lookup(m.as_str()).is_some() {
                return true;
            }
        }
    }
    false
}

fn is_substantive_question(q: &str) -> bool {
    let trimmed = q.trim();
    let q_lower = q.to_lowercase();
    let char_count = trimmed.chars().count();

    if char_count < 6 {
        return false;
    }

    const RHETORICAL: &[&str] = &[
        "对吧",
        "是不是",
        "好吗",
        "行吗",
        "可以吗",
        "有问题吗",
        "你知道吗",
        "你知道吧",
        "明白吗",
        "懂吗",
        "听懂了吗",
        "清楚了吗",
        "明白了吗",
        "是吗",
        "真的吗",
        "对不对",
        "没问题吧",
        "可以理解吗",
        "听懂没有",
        "isn't it",
        "don't you",
        "you know",
        "does that make sense",
        "can you hear",
        "are you there",
        "right?",
        "correct?",
        "ok?",
        "okay?",
    ];
    if RHETORICAL.iter().any(|p| q_lower.contains(p)) {
        return false;
    }

    let has_question_marker = trimmed.contains('?')
        || trimmed.contains('？')
        || q_lower.contains("what is")
        || q_lower.contains("what's")
        || q_lower.contains("how does")
        || q_lower.contains("how do")
        || q_lower.contains("how to")
        || q_lower.contains("why ")
        || trimmed.contains("为什么")
        || trimmed.contains("如何")
        || trimmed.contains("怎么")
        || trimmed.contains("什么")
        || trimmed.contains("为何")
        || trimmed.contains("怎样");

    if !has_question_marker {
        return false;
    }

    // 「X是什么」/ what is X with a technical subject always counts
    if let Some(subject) = subject_from_question(trimmed) {
        if glossary::lookup(&subject).is_some() || is_tech_shaped(&subject) {
            return true;
        }
    }

    if contains_technical_signal(trimmed) {
        return true;
    }

    // Deeper why/how questions without explicit glossary hits
    let is_deep_why_how = trimmed.contains("为什么")
        || trimmed.contains("为何")
        || trimmed.contains("如何")
        || trimmed.contains("怎么")
        || trimmed.contains("怎样")
        || q_lower.contains("why ")
        || q_lower.contains("how does")
        || q_lower.contains("how do");
    if is_deep_why_how && char_count >= 12 {
        return true;
    }

    // Comparison / tradeoff / mechanism phrasing
    const DEEP_MARKERS: &[&str] = &[
        "区别",
        "差异",
        "对比",
        "比较",
        "原理",
        "机制",
        "tradeoff",
        "trade-off",
        " versus ",
        " vs ",
        "相比",
        "优缺点",
        "局限",
        "瓶颈",
        "失败",
        "幻觉",
        "延迟",
        "吞吐",
        "成本",
        "扩展",
        "架构",
        "实现",
        "优化",
    ];
    if DEEP_MARKERS.iter().any(|m| q_lower.contains(m)) && char_count >= 10 {
        return true;
    }

    false
}

fn extract_questions(text: &str) -> Vec<String> {
    let mut out = Vec::new();

    for cap in ZH_X_IS_WHAT_RE.captures_iter(text) {
        if let Some(m) = cap.get(0) {
            push_unique(&mut out, m.as_str());
        }
    }
    for cap in ZH_WHAT_IS_X_RE.captures_iter(text) {
        if let Some(m) = cap.get(0) {
            push_unique(&mut out, m.as_str());
        }
    }
    for cap in EN_WHAT_IS_RE.captures_iter(text) {
        if let Some(m) = cap.get(0) {
            push_unique(&mut out, m.as_str());
        }
    }
    for cap in QUESTION_ZH_RE.find_iter(text) {
        push_unique(&mut out, cap.as_str());
    }
    for cap in QUESTION_EN_RE.captures_iter(text) {
        if let Some(m) = cap.get(1) {
            push_unique(&mut out, m.as_str());
        }
    }
    let trimmed = text.trim();
    if (trimmed.ends_with('?') || trimmed.ends_with('？')) && trimmed.len() >= 4 {
        push_unique(&mut out, trimmed);
    }
    out.truncate(4);
    out
}

fn push_unique(out: &mut Vec<String>, s: &str) {
    let q = s.trim().to_string();
    if q.len() >= 4 && !out.contains(&q) {
        out.push(q);
    }
}

fn answer_for_question(question: &str, localize_zh: bool) -> String {
    let q = question.to_lowercase();

    if let Some((display, exp)) = glossary::lookup(question) {
        return if localize_zh {
            exp.to_string()
        } else {
            format!("Technical term: {display}.")
        };
    }
    if let Some(subject) = subject_from_question(question) {
        if let Some((display, exp)) = glossary::lookup(&subject) {
            return if localize_zh {
                exp.to_string()
            } else {
                format!("Technical term: {display}.")
            };
        }
    }

    if q.contains("rag") {
        return if localize_zh {
            "检索增强生成：先查知识库再让模型回答。作用：减少幻觉、接入私有资料。例：企业 wiki 问答先搜文档再生成。".into()
        } else {
            "Retrieval-augmented generation: retrieve documents first, then generate an answer. Reduces hallucinations and grounds answers in private data.".into()
        };
    }
    if q.contains("embedding") {
        return if localize_zh {
            "嵌入向量：把文本映射为数值向量。用途：语义搜索与 RAG 检索。例：「机器学习」与「ML」向量相近。".into()
        } else {
            "Embeddings map text to dense vectors for semantic search and RAG retrieval.".into()
        };
    }
    if question.contains("什么") || q.contains("what is") || q.contains("what's") {
        return if localize_zh {
            "这是概念/定义类问题，等待 LLM 补充更完整解释。".into()
        } else {
            "Definition-style question; waiting for LLM to expand.".into()
        };
    }
    if question.contains("为什么") || q.contains("why") {
        return if localize_zh {
            "这是因果/动机类问题，需结合前后文与领域背景分析。".into()
        } else {
            "Why/cause-style question; needs surrounding context to answer well.".into()
        };
    }
    if question.contains("如何")
        || question.contains("怎么")
        || q.contains("how to")
        || q.contains("how do")
    {
        return if localize_zh {
            "这是方法/步骤类问题，可拆解为流程或关键条件来回答。".into()
        } else {
            "How-to/process question; break into steps or prerequisites.".into()
        };
    }
    if localize_zh {
        "（初步）已识别为问题句，等待 LLM 补充答案。".into()
    } else {
        "Preliminary question detected; waiting for LLM answer.".into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_rag_not_generic_words() {
        let r = analyze_heuristic("What is RAG in AI? NBA games tonight.", true);
        assert!(r.terms.iter().any(|t| t.term == "RAG"));
        assert!(
            !r.keywords.iter().any(|k| k.eq_ignore_ascii_case("games")),
            "keywords: {:?}",
            r.keywords
        );
        assert!(
            !r.keywords.iter().any(|k| k.eq_ignore_ascii_case("nba")),
            "keywords: {:?}",
            r.keywords
        );
    }

    #[test]
    fn embedding_what_is_zh_without_question_mark() {
        let r = analyze_heuristic("embedding 是什么", true);
        assert!(
            r.terms.iter().any(|t| t.term.eq_ignore_ascii_case("embedding")),
            "terms: {:?}",
            r.terms
        );
        assert!(
            r.questions.iter().any(|q| q.question.contains("embedding")),
            "questions: {:?}",
            r.questions
        );
        assert!(
            r.questions[0].answer.to_lowercase().contains("向量")
                || r.questions[0].answer.to_lowercase().contains("embedding"),
            "answer: {}",
            r.questions[0].answer
        );
    }

    #[test]
    fn filters_rhetorical_questions() {
        let r = analyze_heuristic("这个功能对吧？你明白了吗？", true);
        assert!(
            r.questions.is_empty(),
            "expected no rhetorical questions, got {:?}",
            r.questions
        );
    }

    #[test]
    fn keeps_deep_technical_question() {
        let r = analyze_heuristic("为什么 RAG 比纯 LLM 更不容易产生幻觉？", true);
        assert!(
            !r.questions.is_empty(),
            "expected substantive question, got {:?}",
            r.questions
        );
    }
}
