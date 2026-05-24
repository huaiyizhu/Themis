use crate::proto::{
    themis_service_server::ThemisService, GetDiagnosticsRequest, GetDiagnosticsResponse,
    GetStatusRequest, GetStatusResponse, LatencyBreakdown as ProtoLatencyBreakdown,
    LatencyRecord as ProtoLatencyRecord, LatencySummary as ProtoLatencySummary,
    StartCaptureRequest, StartCaptureResponse, StopCaptureRequest, StopCaptureResponse,
    SubscribeTranscriptsRequest, TranscriptMessage,
};
use std::pin::Pin;
use std::sync::Arc;
use themis_core::{
    CaptureDiagnostics, CaptureState, LatencyBreakdown, LatencyDiagnostics, StateMachine,
    TranscriptEvent,
};
use tokio::sync::broadcast;
use tokio_stream::{wrappers::BroadcastStream, Stream, StreamExt};
use tonic::{Request, Response, Status};
use tracing::info;

pub struct CaptureService {
    pub state: Arc<StateMachine>,
    /// Subscribers attach here; engine publishes via the cloned sender.
    pub transcript_tx: broadcast::Sender<TranscriptEvent>,
    pub capture_diag: Arc<CaptureDiagnostics>,
    pub latency_diag: Arc<LatencyDiagnostics>,
    pub engine: Arc<dyn CaptureEngineHandle + Send + Sync>,
}

fn breakdown_to_proto(b: &LatencyBreakdown) -> ProtoLatencyBreakdown {
    ProtoLatencyBreakdown {
        buffer_ms: b.buffer_ms,
        azure_ms: b.azure_ms,
        stt_wall_ms: b.stt_wall_ms,
        estimated_e2e_ms: b.estimated_e2e_ms,
        language: b.language.clone(),
    }
}

#[async_trait::async_trait]
pub trait CaptureEngineHandle: Send + Sync {
    async fn start(&self) -> anyhow::Result<()>;
    async fn stop(&self) -> anyhow::Result<()>;
}

pub struct ThemisGrpcServer {
    service: Arc<CaptureService>,
}

impl ThemisGrpcServer {
    pub fn new(service: CaptureService) -> Self {
        Self {
            service: Arc::new(service),
        }
    }
}

fn format_capture_status(base: &str, diag: &themis_core::CaptureDiagnosticsSnapshot) -> String {
    let signal = if diag.peak >= 2000 {
        "strong"
    } else if diag.peak >= 200 {
        "ok"
    } else if diag.frames > 0 {
        "quiet"
    } else {
        "silent"
    };
    format!(
        "{base} | capture={} sessions={} peak={} frames={} signal={}",
        diag.mode, diag.sessions, diag.peak, diag.frames, signal
    )
}

#[tonic::async_trait]
impl ThemisService for ThemisGrpcServer {
    async fn start_capture(
        &self,
        _request: Request<StartCaptureRequest>,
    ) -> Result<Response<StartCaptureResponse>, Status> {
        match self.service.engine.start().await {
            Ok(()) => {
                self.service
                    .state
                    .set_state(CaptureState::Capturing, "listening for audio");
                info!("StartCapture RPC");
                Ok(Response::new(StartCaptureResponse {
                    ok: true,
                    message: "capture started".into(),
                }))
            }
            Err(e) => Ok(Response::new(StartCaptureResponse {
                ok: false,
                message: e.to_string(),
            })),
        }
    }

    async fn stop_capture(
        &self,
        _request: Request<StopCaptureRequest>,
    ) -> Result<Response<StopCaptureResponse>, Status> {
        match self.service.engine.stop().await {
            Ok(()) => {
                self.service
                    .state
                    .set_state(CaptureState::Idle, "stopped via gRPC");
                info!("StopCapture RPC");
                Ok(Response::new(StopCaptureResponse {
                    ok: true,
                    message: "capture stopped".into(),
                }))
            }
            Err(e) => Ok(Response::new(StopCaptureResponse {
                ok: false,
                message: e.to_string(),
            })),
        }
    }

    async fn get_status(
        &self,
        _request: Request<GetStatusRequest>,
    ) -> Result<Response<GetStatusResponse>, Status> {
        let status = self.service.state.status();
        let diag = self.service.capture_diag.snapshot();
        let state_str = match status.state {
            CaptureState::Idle => "idle",
            CaptureState::Capturing => "capturing",
            CaptureState::Error => "error",
        };
        let message = if status.state == CaptureState::Capturing {
            format_capture_status(&status.message, &diag)
        } else {
            status.message
        };
        Ok(Response::new(GetStatusResponse {
            state: state_str.into(),
            message,
            transcripts_received: status.transcripts_received,
            audio_peak: diag.peak,
            audio_frames: diag.frames,
            capture_mode: diag.mode,
            audio_sessions: diag.sessions,
            capture_detail: diag.detail,
        }))
    }

    async fn get_diagnostics(
        &self,
        _request: Request<GetDiagnosticsRequest>,
    ) -> Result<Response<GetDiagnosticsResponse>, Status> {
        let snap = self.service.latency_diag.snapshot();
        let records = snap
            .records
            .into_iter()
            .map(|r| ProtoLatencyRecord {
                id: r.id,
                text: r.text,
                is_final: r.is_final,
                emitted_unix_ms: r.emitted_unix_ms,
                breakdown: Some(breakdown_to_proto(&r.breakdown)),
            })
            .collect();
        let summary = snap.summary;
        Ok(Response::new(GetDiagnosticsResponse {
            records,
            summary: Some(ProtoLatencySummary {
                count: summary.count,
                avg_azure_ms: summary.avg_azure_ms,
                avg_e2e_ms: summary.avg_e2e_ms,
                max_e2e_ms: summary.max_e2e_ms,
                last_azure_ms: summary.last_azure_ms,
            }),
        }))
    }

    type SubscribeTranscriptsStream =
        Pin<Box<dyn Stream<Item = Result<TranscriptMessage, Status>> + Send>>;

    async fn subscribe_transcripts(
        &self,
        _request: Request<SubscribeTranscriptsRequest>,
    ) -> Result<Response<Self::SubscribeTranscriptsStream>, Status> {
        let rx = self.service.transcript_tx.subscribe();
        let stream = BroadcastStream::new(rx).filter_map(|item| match item {
            Ok(ev) => Some(Ok(TranscriptMessage {
                text: ev.text,
                is_final: ev.is_final,
                feedback: ev.feedback.unwrap_or_default(),
                timestamp_unix_ms: ev.emitted_unix_ms,
                latency: ev.latency.as_ref().map(breakdown_to_proto),
            })),
            Err(_) => None,
        });
        Ok(Response::new(Box::pin(stream)))
    }
}
