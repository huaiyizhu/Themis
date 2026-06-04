use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AnalysisContext {
    /// Recent finalized transcript lines before the current phrase.
    pub recent_transcript: Option<String>,
    /// Rolling full-session summary when available.
    pub session_summary: Option<String>,
    /// Prefer Chinese glosses/answers when true (see analysis-prefs.json).
    #[serde(default = "default_localize_zh")]
    pub localize_zh: bool,
}

fn default_localize_zh() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TermInsight {
    pub term: String,
    pub explanation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QuestionInsight {
    pub question: String,
    pub answer: String,
}

/// Structured output from transcript analysis (keywords, glosses, Q&A).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AnalysisResult {
    pub keywords: Vec<String>,
    pub terms: Vec<TermInsight>,
    pub questions: Vec<QuestionInsight>,
}

impl AnalysisResult {
    pub fn is_empty(&self) -> bool {
        self.keywords.is_empty() && self.terms.is_empty() && self.questions.is_empty()
    }

    pub fn merge(&mut self, other: AnalysisResult) {
        self.merge_keywords_and_terms(&other);
        for q in other.questions {
            merge_question(self, q);
        }
    }

    /// Merge LLM keywords/terms and attach LLM answers to existing transcript questions.
    /// Question text always stays verbatim from the transcript (heuristic extraction); LLM
    /// must not add or rephrase questions.
    pub fn merge_llm_supplement(&mut self, other: &AnalysisResult) {
        self.merge_keywords_and_terms(other);
        attach_llm_question_answers(&mut self.questions, &other.questions);
    }

    fn merge_keywords_and_terms(&mut self, other: &AnalysisResult) {
        for kw in &other.keywords {
            if !self
                .keywords
                .iter()
                .any(|k| k.eq_ignore_ascii_case(kw))
            {
                self.keywords.push(kw.clone());
            }
        }
        for t in &other.terms {
            if let Some(existing) = self
                .terms
                .iter_mut()
                .find(|x| x.term.eq_ignore_ascii_case(&t.term))
            {
                if should_prefer_explanation(&existing.explanation, &t.explanation) {
                    existing.explanation = t.explanation.clone();
                }
            } else {
                self.terms.push(t.clone());
            }
        }
    }

    /// One-line fallback for legacy `feedback` field.
    pub fn summary(&self) -> String {
        let mut parts = Vec::new();
        if !self.keywords.is_empty() {
            parts.push(format!("关键词: {}", self.keywords.join(" · ")));
        }
        if let Some(t) = self.terms.first() {
            parts.push(format!("{} — {}", t.term, t.explanation));
        }
        if let Some(q) = self.questions.first() {
            parts.push(format!("Q: {} A: {}", q.question, q.answer));
        }
        parts.join(" | ")
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".into())
    }

    pub fn from_json(s: &str) -> Option<Self> {
        serde_json::from_str(s).ok()
    }
}

fn merge_question(result: &mut AnalysisResult, incoming: QuestionInsight) {
    if let Some(existing) = result
        .questions
        .iter_mut()
        .find(|x| questions_match(&x.question, &incoming.question))
    {
        if should_prefer_explanation(&existing.answer, &incoming.answer) {
            existing.answer = incoming.answer;
        }
        return;
    }
    result.questions.push(incoming);
}

fn attach_llm_question_answers(
    transcript_questions: &mut [QuestionInsight],
    llm_questions: &[QuestionInsight],
) {
    for (idx, q) in transcript_questions.iter_mut().enumerate() {
        let llm_q = llm_questions
            .iter()
            .find(|x| questions_match(&x.question, &q.question))
            .or_else(|| llm_questions.get(idx));
        if let Some(llm_q) = llm_q {
            if should_prefer_explanation(&q.answer, &llm_q.answer) {
                q.answer = llm_q.answer.clone();
            }
        }
    }
}

fn normalize_question_key(q: &str) -> String {
    q.trim()
        .trim_end_matches(['?', '？', '.', '!', '！'])
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn questions_match(a: &str, b: &str) -> bool {
    let na = normalize_question_key(a);
    let nb = normalize_question_key(b);
    if na.is_empty() || nb.is_empty() {
        return false;
    }
    if na == nb {
        return true;
    }
    na.contains(&nb) || nb.contains(&na)
}

/// Collapse whitespace/punctuation for substring checks against live transcript lines.
pub fn normalize_transcript_match(s: &str) -> String {
    s.chars()
        .filter(|c| {
            !c.is_whitespace()
                && !['，', ',', '。', '.', '、', '；', ';', '：', ':', '！', '!', '？', '?']
                    .contains(c)
        })
        .flat_map(|c| c.to_lowercase())
        .collect()
}

/// True when `question` appears verbatim (modulo spacing/punctuation) inside `transcript`.
pub fn question_in_transcript(question: &str, transcript: &str) -> bool {
    let q = normalize_transcript_match(question);
    let t = normalize_transcript_match(transcript);
    if q.chars().count() < 4 || t.is_empty() {
        return false;
    }
    t.contains(&q)
}

pub fn retain_questions_in_transcript(result: &mut AnalysisResult, transcript: &str) {
    result
        .questions
        .retain(|q| question_in_transcript(&q.question, transcript));
}

pub fn is_placeholder_answer(answer: &str) -> bool {
    let a = answer.trim();
    a.contains("FOUNDRY")
        || a.contains("配置 Azure OpenAI")
        || a.contains("若已配置")
        || a.starts_with("（初步）")
        || a.contains("这是概念/定义类问题")
        || a.contains("这是因果/动机类问题")
        || a.contains("这是方法/步骤类问题")
        || a.contains("Identified as a question")
        || a.contains("Configure Azure OpenAI")
        || a.contains("Definition-style question")
        || a.contains("waiting for LLM")
        || a.contains("Why/cause-style question")
        || a.contains("How-to/process question")
        || a.contains("Preliminary question detected")
}

fn should_prefer_explanation(existing: &str, incoming: &str) -> bool {
    if incoming.trim().is_empty() {
        return false;
    }
    if existing.trim().is_empty() {
        return true;
    }
    if is_placeholder_answer(existing) && !is_placeholder_answer(incoming) {
        return true;
    }
    if !is_placeholder_answer(existing) && is_placeholder_answer(incoming) {
        return false;
    }
    incoming.len() > existing.len()
}

pub fn finalize_question_answers(
    merged: &mut AnalysisResult,
    llm: Option<&AnalysisResult>,
    llm_configured: bool,
    llm_status: &str,
    localize_zh: bool,
) {
    if !llm_configured {
        return;
    }
    for q in &mut merged.questions {
        if !is_placeholder_answer(&q.answer) {
            continue;
        }
        if let Some(llm) = llm {
            if let Some(llm_q) = llm
                .questions
                .iter()
                .find(|x| questions_match(&x.question, &q.question))
            {
                if !is_placeholder_answer(&llm_q.answer) {
                    q.answer = llm_q.answer.clone();
                    continue;
                }
            }
        }
        q.answer = configured_placeholder_message(llm_status, localize_zh);
    }
}

fn configured_placeholder_message(llm_status: &str, localize_zh: bool) -> String {
    if llm_status.starts_with("error") {
        if localize_zh {
            format!("LLM 分析失败（{llm_status}），以上为初步识别。")
        } else {
            format!("LLM analysis failed ({llm_status}); showing heuristic hint only.")
        }
    } else if localize_zh {
        "暂无更深入的技术解读（可能不是技术向问题，或 LLM 未提取到答案）。".into()
    } else {
        "No deeper technical answer yet (may be non-technical speech, or LLM returned nothing)."
            .into()
    }
}

#[cfg(test)]
mod merge_tests {
    use super::*;

    #[test]
    fn merge_replaces_placeholder_with_llm_answer() {
        let mut base = AnalysisResult {
            questions: vec![QuestionInsight {
                question: "What is RAG?".into(),
                answer: "（初步）已识别为问题句；配置 Azure OpenAI（FOUNDRY_*）可获得更完整回答。".into(),
            }],
            ..Default::default()
        };
        let llm = AnalysisResult {
            questions: vec![QuestionInsight {
                question: "What is RAG in AI?".into(),
                answer: "RAG retrieves documents before generation.".into(),
            }],
            ..Default::default()
        };
        base.merge(llm);
        assert_eq!(base.questions.len(), 1);
        assert!(base.questions[0].answer.contains("retrieves"));
    }

    #[test]
    fn merge_llm_supplement_keeps_transcript_question_text() {
        let mut base = AnalysisResult {
            questions: vec![QuestionInsight {
                question: "What is RAG?".into(),
                answer: "Definition-style question; waiting for LLM to expand.".into(),
            }],
            ..Default::default()
        };
        let llm = AnalysisResult {
            questions: vec![
                QuestionInsight {
                    question: "What is RAG in AI systems?".into(),
                    answer: "RAG retrieves documents before generation.".into(),
                },
                QuestionInsight {
                    question: "How does transformer attention work?".into(),
                    answer: "Self-attention compares all tokens in the sequence.".into(),
                },
            ],
            ..Default::default()
        };
        base.merge_llm_supplement(&llm);
        assert_eq!(base.questions.len(), 1);
        assert_eq!(base.questions[0].question, "What is RAG?");
        assert!(base.questions[0].answer.contains("retrieves"));
    }

    #[test]
    fn question_in_transcript_normalizes_spacing() {
        assert!(question_in_transcript("RAG 是什么？", "那么 RAG 是什么？"));
        assert!(!question_in_transcript(
            "什么是 Transformer？",
            "今天讲 embedding"
        ));
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisMeta {
    pub llm_configured: bool,
    /// `disabled` | `skipped` | `ok` | `empty` | `timeout` | `error`
    pub llm_status: String,
    pub heuristic_ms: u32,
    pub llm_ms: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisDetail {
    pub merged: AnalysisResult,
    pub heuristic: AnalysisResult,
    pub llm: Option<AnalysisResult>,
    pub meta: AnalysisMeta,
}

#[async_trait]
pub trait AnalysisProvider: Send + Sync {
    async fn analyze(
        &self,
        transcript: &str,
        ctx: &AnalysisContext,
    ) -> anyhow::Result<Option<AnalysisDetail>>;
}

pub struct NoopAnalysis;

#[async_trait]
impl AnalysisProvider for NoopAnalysis {
    async fn analyze(
        &self,
        _transcript: &str,
        _ctx: &AnalysisContext,
    ) -> anyhow::Result<Option<AnalysisDetail>> {
        Ok(None)
    }
}
