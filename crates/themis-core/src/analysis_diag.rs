use crate::analysis::{AnalysisDetail, AnalysisResult};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisInsightRecord {
    pub id: u64,
    pub text: String,
    pub emitted_unix_ms: i64,
    pub heuristic: AnalysisResult,
    pub llm: Option<AnalysisResult>,
    pub merged: AnalysisResult,
    pub llm_configured: bool,
    pub llm_status: String,
    pub heuristic_ms: u32,
    pub llm_ms: Option<u32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AnalysisDiagnosticsSummary {
    pub count: u32,
    pub llm_configured: bool,
    pub last_llm_status: String,
}

#[derive(Debug)]
pub struct AnalysisDiagnostics {
    records: Mutex<VecDeque<AnalysisInsightRecord>>,
    summary: Mutex<AnalysisDiagnosticsSummary>,
    next_id: Mutex<u64>,
    max_records: usize,
}

impl AnalysisDiagnostics {
    pub fn new(max_records: usize) -> Self {
        Self {
            records: Mutex::new(VecDeque::new()),
            summary: Mutex::new(AnalysisDiagnosticsSummary::default()),
            next_id: Mutex::new(1),
            max_records,
        }
    }

    pub fn push(&self, text: String, detail: &AnalysisDetail) -> AnalysisInsightRecord {
        let id = {
            let mut n = self.next_id.lock().unwrap();
            let id = *n;
            *n += 1;
            id
        };
        let record = AnalysisInsightRecord {
            id,
            text: truncate(&text, 120),
            emitted_unix_ms: chrono::Utc::now().timestamp_millis(),
            heuristic: detail.heuristic.clone(),
            llm: detail.llm.clone(),
            merged: detail.merged.clone(),
            llm_configured: detail.meta.llm_configured,
            llm_status: detail.meta.llm_status.clone(),
            heuristic_ms: detail.meta.heuristic_ms,
            llm_ms: detail.meta.llm_ms,
        };
        {
            let mut q = self.records.lock().unwrap();
            q.push_back(record.clone());
            while q.len() > self.max_records {
                q.pop_front();
            }
        }
        {
            let mut s = self.summary.lock().unwrap();
            s.count += 1;
            s.llm_configured = detail.meta.llm_configured;
            s.last_llm_status = detail.meta.llm_status.clone();
        }
        record
    }

    pub fn snapshot(&self) -> AnalysisDiagnosticsSnapshot {
        AnalysisDiagnosticsSnapshot {
            records: self.records.lock().unwrap().iter().cloned().collect(),
            summary: self.summary.lock().unwrap().clone(),
        }
    }

    pub fn clear(&self) {
        self.records.lock().unwrap().clear();
        *self.summary.lock().unwrap() = AnalysisDiagnosticsSummary::default();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisDiagnosticsSnapshot {
    pub records: Vec<AnalysisInsightRecord>,
    pub summary: AnalysisDiagnosticsSummary,
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    s.chars().take(max).collect::<String>() + "…"
}
