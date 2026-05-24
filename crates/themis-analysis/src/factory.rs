use async_trait::async_trait;
use std::sync::Arc;
use std::time::Duration;
use themis_core::{AnalysisProvider, AnalysisResult, NoopAnalysis, ThemisConfig};
use tokio::time::timeout;
use tracing::debug;

use crate::heuristic::analyze_heuristic;
use crate::llm::LlmAnalyzer;

struct CompositeAnalyzer {
    llm: Option<LlmAnalyzer>,
    llm_timeout: Duration,
}

#[async_trait]
impl AnalysisProvider for CompositeAnalyzer {
    async fn analyze(&self, transcript: &str) -> anyhow::Result<Option<AnalysisResult>> {
        let mut result = analyze_heuristic(transcript);

        if let Some(llm) = &self.llm {
            match timeout(self.llm_timeout, llm.analyze(transcript)).await {
                Ok(Ok(Some(llm_result))) => {
                    result.merge(llm_result);
                }
                Ok(Ok(None)) => {}
                Ok(Err(e)) => debug!(error = %e, "llm analyze error"),
                Err(_) => debug!("llm analyze timed out"),
            }
        }

        if result.is_empty() {
            Ok(None)
        } else {
            Ok(Some(result))
        }
    }
}

pub fn create_analyzer(config: &ThemisConfig) -> Arc<dyn AnalysisProvider> {
    if !config.analysis_enabled {
        return Arc::new(NoopAnalysis);
    }

    let llm = LlmAnalyzer::from_config(config);
    if llm.is_none() {
        debug!("analysis: heuristic only (set FOUNDRY_* for LLM glosses/answers)");
    }

    Arc::new(CompositeAnalyzer {
        llm,
        llm_timeout: Duration::from_secs(12),
    })
}
