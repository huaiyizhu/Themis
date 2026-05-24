use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::{Mutex, RwLock};

/// Per-phrase latency breakdown (REST mode: buffer ≈ chunk length).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyBreakdown {
    /// Audio accumulated before this STT request (~4s in REST mode).
    pub buffer_ms: u32,
    /// Azure HTTP round-trip (network + recognition).
    pub azure_ms: u32,
    /// Wall time for STT step (parallel multi-lang ≈ max azure).
    pub stt_wall_ms: u32,
    /// Estimated speech-end → text-ready: buffer + azure.
    pub estimated_e2e_ms: u32,
    pub language: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyRecord {
    pub id: u64,
    pub text: String,
    pub is_final: bool,
    pub emitted_unix_ms: i64,
    pub breakdown: LatencyBreakdown,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LatencySummary {
    pub count: u32,
    pub avg_azure_ms: u32,
    pub avg_e2e_ms: u32,
    pub max_e2e_ms: u32,
    pub last_azure_ms: u32,
}

#[derive(Debug)]
pub struct LatencyDiagnostics {
    records: Mutex<VecDeque<LatencyRecord>>,
    summary: RwLock<LatencySummary>,
    next_id: Mutex<u64>,
    max_records: usize,
}

impl LatencyDiagnostics {
    pub fn new(max_records: usize) -> Self {
        Self {
            records: Mutex::new(VecDeque::new()),
            summary: RwLock::new(LatencySummary::default()),
            next_id: Mutex::new(1),
            max_records,
        }
    }

    pub fn push(
        &self,
        text: String,
        is_final: bool,
        breakdown: LatencyBreakdown,
    ) -> LatencyRecord {
        let id = {
            let mut n = self.next_id.lock().unwrap();
            let id = *n;
            *n += 1;
            id
        };
        let record = LatencyRecord {
            id,
            text: truncate(&text, 120),
            is_final,
            emitted_unix_ms: Utc::now().timestamp_millis(),
            breakdown,
        };

        {
            let mut q = self.records.lock().unwrap();
            q.push_back(record.clone());
            while q.len() > self.max_records {
                q.pop_front();
            }
        }

        if is_final {
            self.update_summary(&record.breakdown);
        }

        record
    }

    fn update_summary(&self, b: &LatencyBreakdown) {
        let mut s = self.summary.write().unwrap();
        s.count += 1;
        let n = s.count as f64;
        s.avg_azure_ms = ((s.avg_azure_ms as f64 * (n - 1.0) + b.azure_ms as f64) / n) as u32;
        s.avg_e2e_ms =
            ((s.avg_e2e_ms as f64 * (n - 1.0) + b.estimated_e2e_ms as f64) / n) as u32;
        s.max_e2e_ms = s.max_e2e_ms.max(b.estimated_e2e_ms);
        s.last_azure_ms = b.azure_ms;
    }

    pub fn snapshot(&self) -> LatencyDiagnosticsSnapshot {
        let records: Vec<LatencyRecord> = self.records.lock().unwrap().iter().cloned().collect();
        let summary = self.summary.read().unwrap().clone();
        LatencyDiagnosticsSnapshot { records, summary }
    }

    pub fn clear(&self) {
        self.records.lock().unwrap().clear();
        *self.summary.write().unwrap() = LatencySummary::default();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyDiagnosticsSnapshot {
    pub records: Vec<LatencyRecord>,
    pub summary: LatencySummary,
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    s.chars().take(max).collect::<String>() + "…"
}
