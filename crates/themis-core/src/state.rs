use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU8, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum CaptureState {
    Idle = 0,
    Capturing = 1,
    Error = 2,
}

impl From<u8> for CaptureState {
    fn from(v: u8) -> Self {
        match v {
            1 => CaptureState::Capturing,
            2 => CaptureState::Error,
            _ => CaptureState::Idle,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceStatus {
    pub state: CaptureState,
    pub message: String,
    pub transcripts_received: u64,
}

pub struct StateMachine {
    inner: AtomicU8,
    message: std::sync::RwLock<String>,
    transcript_count: std::sync::atomic::AtomicU64,
}

impl StateMachine {
    pub fn new() -> Self {
        Self {
            inner: AtomicU8::new(CaptureState::Idle as u8),
            message: std::sync::RwLock::new(String::new()),
            transcript_count: std::sync::atomic::AtomicU64::new(0),
        }
    }

    pub fn state(&self) -> CaptureState {
        CaptureState::from(self.inner.load(Ordering::SeqCst))
    }

    pub fn set_state(&self, state: CaptureState, message: impl Into<String>) {
        self.inner.store(state as u8, Ordering::SeqCst);
        if let Ok(mut msg) = self.message.write() {
            *msg = message.into();
        }
    }

    pub fn status(&self) -> ServiceStatus {
        ServiceStatus {
            state: self.state(),
            message: self.message.read().map(|m| m.clone()).unwrap_or_default(),
            transcripts_received: self.transcript_count.load(Ordering::SeqCst),
        }
    }

    pub fn record_transcript(&self) {
        self.transcript_count.fetch_add(1, Ordering::SeqCst);
    }
}

impl Default for StateMachine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_transitions() {
        let sm = StateMachine::new();
        assert_eq!(sm.state(), CaptureState::Idle);
        sm.set_state(CaptureState::Capturing, "started");
        assert_eq!(sm.state(), CaptureState::Capturing);
        sm.record_transcript();
        assert_eq!(sm.status().transcripts_received, 1);
    }
}
