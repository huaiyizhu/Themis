use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::RwLock;

/// Live capture health metrics (updated by `themis-audio`, read by service/UI).
#[derive(Debug)]
pub struct CaptureDiagnostics {
    peak: AtomicU32,
    frames: AtomicU64,
    sessions: AtomicU32,
    mode: RwLock<String>,
    detail: RwLock<String>,
}

impl CaptureDiagnostics {
    pub fn new() -> Self {
        Self {
            peak: AtomicU32::new(0),
            frames: AtomicU64::new(0),
            sessions: AtomicU32::new(0),
            mode: RwLock::new("idle".into()),
            detail: RwLock::new(String::new()),
        }
    }

    pub fn record_frame(&self, raw_peak: u32) {
        self.frames.fetch_add(1, Ordering::Relaxed);
        self.peak.fetch_max(raw_peak, Ordering::Relaxed);
    }

    pub fn set_mode(&self, mode: impl Into<String>) {
        if let Ok(mut m) = self.mode.write() {
            *m = mode.into();
        }
    }

    pub fn set_detail(&self, detail: impl Into<String>) {
        if let Ok(mut d) = self.detail.write() {
            *d = detail.into();
        }
    }

    pub fn set_sessions(&self, count: u32) {
        self.sessions.store(count, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> CaptureDiagnosticsSnapshot {
        CaptureDiagnosticsSnapshot {
            peak: self.peak.load(Ordering::Relaxed),
            frames: self.frames.load(Ordering::Relaxed),
            sessions: self.sessions.load(Ordering::Relaxed),
            mode: self.mode.read().map(|s| s.clone()).unwrap_or_default(),
            detail: self.detail.read().map(|s| s.clone()).unwrap_or_default(),
        }
    }

    pub fn reset_session_peak(&self) {
        self.peak.store(0, Ordering::Relaxed);
    }
}

impl Default for CaptureDiagnostics {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct CaptureDiagnosticsSnapshot {
    pub peak: u32,
    pub frames: u64,
    pub sessions: u32,
    pub mode: String,
    pub detail: String,
}

impl CaptureDiagnosticsSnapshot {
    pub fn is_healthy(&self) -> bool {
        self.frames > 0 && self.peak >= 200
    }
}
