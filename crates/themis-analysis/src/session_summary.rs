//! Rolling full-session transcript summary (LLM, interval-based updates).

use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::llm::LlmAnalyzer;
use themis_core::ThemisConfig;

const LLM_MIN_CHARS: usize = 48;
/// How many prior transcript lines to pass as LLM analysis context.
pub const ANALYSIS_CONTEXT_LINES: usize = 10;

pub struct SessionSummarizer {
    lines: Mutex<Vec<String>>,
    summary: Mutex<Option<String>>,
    last_llm_at: Mutex<Option<Instant>>,
    llm: Option<LlmAnalyzer>,
    interval: Duration,
}

impl SessionSummarizer {
    pub fn from_config(config: &ThemisConfig) -> Self {
        Self {
            lines: Mutex::new(Vec::new()),
            summary: Mutex::new(None),
            last_llm_at: Mutex::new(None),
            llm: LlmAnalyzer::from_config(config),
            interval: Duration::from_secs(config.session_summary_interval_secs as u64),
        }
    }

    pub fn reset(&self) {
        *self.lines.lock().unwrap() = Vec::new();
        *self.summary.lock().unwrap() = None;
        *self.last_llm_at.lock().unwrap() = None;
    }

    pub fn append_line(&self, line: &str) {
        let t = line.trim();
        if t.is_empty() {
            return;
        }
        let mut lines = self.lines.lock().unwrap();
        if lines.last().map(String::as_str) == Some(t) {
            return;
        }
        lines.push(t.to_string());
    }

    pub fn current_summary(&self) -> Option<String> {
        self.summary.lock().unwrap().clone()
    }

    pub fn full_text(&self) -> String {
        self.lines.lock().unwrap().join("\n")
    }

    /// Prior finalized lines (excludes the latest) for per-phrase LLM context.
    pub fn prior_context(&self, max_lines: usize) -> String {
        let lines = self.lines.lock().unwrap();
        if lines.len() <= 1 {
            return String::new();
        }
        let end = lines.len() - 1;
        let start = end.saturating_sub(max_lines);
        lines[start..end].join("\n")
    }

    fn should_call_llm(&self, text_len: usize) -> bool {
        if text_len < LLM_MIN_CHARS {
            return false;
        }
        let last = self.last_llm_at.lock().unwrap();
        last.map(|t| t.elapsed() >= self.interval)
            .unwrap_or(true)
    }

    /// Periodic LLM refresh. Returns `Some` only when a new stable summary is produced.
    pub async fn refresh_if_due(&self) -> Option<String> {
        let text = self.full_text();
        if text.is_empty() || !self.should_call_llm(text.len()) {
            return None;
        }

        let llm = self.llm.as_ref()?;

        let trimmed = match llm.summarize_transcript(&text).await {
            Ok(Some(s)) if !s.trim().is_empty() => s.trim().to_string(),
            _ => return None,
        };

        let unchanged = self
            .summary
            .lock()
            .unwrap()
            .as_ref()
            .map(|prev| prev.trim() == trimmed)
            .unwrap_or(false);
        if unchanged {
            *self.last_llm_at.lock().unwrap() = Some(Instant::now());
            return None;
        }

        *self.last_llm_at.lock().unwrap() = Some(Instant::now());
        *self.summary.lock().unwrap() = Some(trimmed.clone());
        Some(trimmed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use themis_core::ThemisConfig;

    #[test]
    fn prior_context_excludes_latest_line() {
        let s = SessionSummarizer::from_config(&ThemisConfig::default());
        s.append_line("line one");
        s.append_line("line two");
        s.append_line("line three");
        assert_eq!(s.prior_context(10), "line one\nline two");
        assert_eq!(s.prior_context(1), "line two");
    }
}
