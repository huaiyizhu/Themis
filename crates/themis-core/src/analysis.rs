use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AnalysisContext {
    /// Recent finalized transcript lines before the current phrase.
    pub recent_transcript: Option<String>,
    /// Rolling full-session summary when available.
    pub session_summary: Option<String>,
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
        for kw in other.keywords {
            if !self.keywords.iter().any(|k| k.eq_ignore_ascii_case(&kw)) {
                self.keywords.push(kw);
            }
        }
        for t in other.terms {
            if !self
                .terms
                .iter()
                .any(|x| x.term.eq_ignore_ascii_case(&t.term))
            {
                self.terms.push(t);
            }
        }
        for q in other.questions {
            if !self
                .questions
                .iter()
                .any(|x| x.question == q.question)
            {
                self.questions.push(q);
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
