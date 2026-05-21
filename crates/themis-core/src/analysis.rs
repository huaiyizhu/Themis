use async_trait::async_trait;

#[async_trait]
pub trait AnalysisProvider: Send + Sync {
    async fn analyze(&self, transcript: &str) -> anyhow::Result<Option<String>>;
}

pub struct NoopAnalysis;

#[async_trait]
impl AnalysisProvider for NoopAnalysis {
    async fn analyze(&self, _transcript: &str) -> anyhow::Result<Option<String>> {
        Ok(None)
    }
}
