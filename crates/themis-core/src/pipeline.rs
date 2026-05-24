use crate::latency::LatencyBreakdown;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptEvent {
    pub text: String,
    pub is_final: bool,
    pub feedback: Option<String>,
    pub emitted_unix_ms: i64,
    pub latency: Option<LatencyBreakdown>,
}
