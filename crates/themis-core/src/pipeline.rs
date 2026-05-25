use crate::analysis::AnalysisResult;
use crate::latency::LatencyBreakdown;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptEvent {
    pub text: String,
    pub is_final: bool,
    /// Legacy one-line summary; derived from insights when present.
    pub feedback: Option<String>,
    pub insights: Option<AnalysisResult>,
    pub emitted_unix_ms: i64,
    pub latency: Option<LatencyBreakdown>,
    /// Rolling full-session summary in Chinese (updated as transcript grows).
    pub session_summary: Option<String>,
}
