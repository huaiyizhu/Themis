//! Rule-based extraction: glossary, English tech words, Chinese/English questions.

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

pub fn analyze_heuristic(transcript: &str) -> AnalysisResult {
    let text = transcript.trim();
    if text.is_empty() {
        return AnalysisResult::default();
    }

    let mut result = AnalysisResult::default();

    for cap in ACRONYM_RE.captures_iter(text) {
        if let Some(m) = cap.get(0) {
            add_glossary_hit(&mut result, m.as_str());
        }
    }

    for cap in EN_WORD_RE.captures_iter(text) {
        if let Some(m) = cap.get(0) {
            let word = m.as_str();
            add_glossary_hit(&mut result, word);
            push_keyword(&mut result, word);
        }
    }

    for q in extract_questions(text) {
        attach_question(&mut result, &q);
    }

    result.keywords.truncate(10);
    result.terms.truncate(8);
    result.questions.truncate(4);
    result
}

fn add_glossary_hit(result: &mut AnalysisResult, raw: &str) {
    let Some((display, explanation)) = glossary::lookup(raw) else {
        return;
    };
    if !result
        .terms
        .iter()
        .any(|t| t.term.eq_ignore_ascii_case(display))
    {
        result.terms.push(TermInsight {
            term: display.to_string(),
            explanation: explanation.to_string(),
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

fn attach_question(result: &mut AnalysisResult, question: &str) {
    let q = question.trim().to_string();
    if q.len() < 4 {
        return;
    }
    if result.questions.iter().any(|x| x.question == q) {
        return;
    }

    let answer = answer_for_question(&q);
    result.questions.push(QuestionInsight {
        question: q.clone(),
        answer,
    });

    // Pull subject term from 「X是什么」/ what is X
    if let Some(subject) = subject_from_question(&q) {
        add_glossary_hit(result, &subject);
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

fn answer_for_question(question: &str) -> String {
    let q = question.to_lowercase();

    if let Some((_, exp)) = glossary::lookup(question) {
        return exp.to_string();
    }
    if let Some(subject) = subject_from_question(question) {
        if let Some((_, exp)) = glossary::lookup(&subject) {
            return exp.to_string();
        }
    }

    if q.contains("rag") {
        return "RAG 通过检索外部知识再生成回答，常用于减少幻觉、接入私有文档。".into();
    }
    if q.contains("embedding") {
        return "Embedding 把文本映射为向量，用于语义相似度搜索、聚类，是 RAG 与推荐系统的常见基础。"
            .into();
    }
    if question.contains("什么") || q.contains("what is") || q.contains("what's") {
        return "这是概念/定义类问题。若已配置 FOUNDRY_*，LLM 会给出更完整的解释。".into();
    }
    if question.contains("为什么") || q.contains("why") {
        return "这是因果/动机类问题，需结合前后文与领域背景分析。".into();
    }
    if question.contains("如何")
        || question.contains("怎么")
        || q.contains("how to")
        || q.contains("how do")
    {
        return "这是方法/步骤类问题，可拆解为流程或关键条件来回答。".into();
    }
    "（初步）已识别为问题句；配置 Azure OpenAI（FOUNDRY_*）可获得更完整回答。".into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_rag_and_nba() {
        let r = analyze_heuristic("What is RAG in AI? NBA games tonight.");
        assert!(r.terms.iter().any(|t| t.term == "RAG"));
        assert!(r.terms.iter().any(|t| t.term == "NBA"));
    }

    #[test]
    fn embedding_what_is_zh_without_question_mark() {
        let r = analyze_heuristic("embedding 是什么");
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
}
