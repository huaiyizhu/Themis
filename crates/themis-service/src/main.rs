mod engine;

use engine::CaptureEngine;
use std::sync::Arc;
use themis_core::{StateMachine, ThemisConfig};
use themis_ipc::server::{CaptureEngineHandle, CaptureService, ThemisGrpcServer};
use themis_ipc::ThemisServiceServer;
use tonic::transport::Server;
use tracing::info;
use tracing_subscriber::EnvFilter;

struct EngineHandle(Arc<CaptureEngine>);

#[async_trait::async_trait]
impl CaptureEngineHandle for EngineHandle {
    async fn start(&self) -> anyhow::Result<()> {
        self.0.start().await
    }
    async fn stop(&self) -> anyhow::Result<()> {
        self.0.stop().await
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = ThemisConfig::from_env();
    init_logging(&config)?;

    let state = Arc::new(StateMachine::new());
    let engine = Arc::new(CaptureEngine::new(config.clone(), Arc::clone(&state)));

    let service = CaptureService {
        state: Arc::clone(&state),
        transcript_tx: engine.transcript_sender(),
        engine: Arc::new(EngineHandle(Arc::clone(&engine))),
    };

    let addr = format!("127.0.0.1:{}", config.grpc_port).parse()?;
    let svc = ThemisGrpcServer::new(service);

    info!(%addr, "themis-service listening");
    Server::builder()
        .add_service(ThemisServiceServer::new(svc))
        .serve(addr)
        .await?;

    Ok(())
}

fn init_logging(config: &ThemisConfig) -> anyhow::Result<()> {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&config.log_level));

    let log_dir = ThemisConfig::log_dir();
    std::fs::create_dir_all(&log_dir)?;
    let file_appender = tracing_appender::rolling::daily(&log_dir, "themis.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(non_blocking)
        .with_ansi(true)
        .init();

    Ok(())
}
