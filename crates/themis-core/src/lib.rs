mod analysis;
mod capture_diag;
mod config;
mod frame;
mod gain;
mod latency;
mod pipeline;
mod state;

pub use analysis::{
    AnalysisProvider, AnalysisResult, NoopAnalysis, QuestionInsight, TermInsight,
};
pub use capture_diag::{CaptureDiagnostics, CaptureDiagnosticsSnapshot};
pub use config::ThemisConfig;
pub use frame::{AudioFrame, SampleFormat};
pub use gain::normalize_pcm16;
pub use latency::{
    LatencyBreakdown, LatencyDiagnostics, LatencyDiagnosticsSnapshot, LatencyRecord, LatencySummary,
};
pub use pipeline::TranscriptEvent;
pub use state::{CaptureState, ServiceStatus, StateMachine};
