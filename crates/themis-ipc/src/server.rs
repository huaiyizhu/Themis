use crate::proto::{
    themis_service_server::ThemisService, GetStatusRequest, GetStatusResponse, StartCaptureRequest,
    StartCaptureResponse, StopCaptureRequest, StopCaptureResponse, SubscribeTranscriptsRequest,
    TranscriptMessage,
};
use std::pin::Pin;
use std::sync::Arc;
use themis_core::{CaptureState, StateMachine, TranscriptEvent};
use tokio::sync::broadcast;
use tokio_stream::{wrappers::BroadcastStream, Stream, StreamExt};
use tonic::{Request, Response, Status};
use tracing::info;

pub struct CaptureService {
    pub state: Arc<StateMachine>,
    /// Subscribers attach here; engine publishes via the cloned sender.
    pub transcript_tx: broadcast::Sender<TranscriptEvent>,
    pub engine: Arc<dyn CaptureEngineHandle + Send + Sync>,
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
                    .set_state(CaptureState::Capturing, "capturing via gRPC");
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
        let state_str = match status.state {
            CaptureState::Idle => "idle",
            CaptureState::Capturing => "capturing",
            CaptureState::Error => "error",
        };
        Ok(Response::new(GetStatusResponse {
            state: state_str.into(),
            message: status.message,
            transcripts_received: status.transcripts_received,
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
                timestamp_unix_ms: chrono::Utc::now().timestamp_millis(),
            })),
            Err(_) => None,
        });
        Ok(Response::new(Box::pin(stream)))
    }
}
