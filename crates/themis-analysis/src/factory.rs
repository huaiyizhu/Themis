use async_trait::async_trait;
use std::sync::Arc;
use std::time::{Duration, Instant};
use themis_core::{
    AnalysisDetail, AnalysisMeta, AnalysisProvider, AnalysisResult, NoopAnalysis, ThemisConfig,
};
use tokio::time::timeout;
use tracing::debug;

use crate::heuristic::analyze_heuristic;
use crate::llm::LlmAnalyzer;

struct CompositeAnalyzer {
    llm: Option<LlmAnalyzer>,
    llm_configured: bool,
    llm_timeout: Duration,
}

#[async_trait]
impl AnalysisProvider for CompositeAnalyzer {
    async fn analyze(&self, transcript: &str) -> anyhow::Result<Option<AnalysisDetail>> {
        let h0 = Instant::now();
        let heuristic = analyze_heuristic(transcript);
        let heuristic_ms = h0.elapsed().as_millis() as u32;

        let mut merged = heuristic.clone();
        let mut llm_out: Option<AnalysisResult> = None;
        let mut llm_status = if self.llm_configured {
            "pending".to_string()
        } else {
            "disabled".to_string()
        };
        let mut llm_ms = None;

        if let Some(llm) = &self.llm {
            let l0 = Instant::now();
            match timeout(self.llm_timeout, llm.analyze(transcript)).await {
                Ok(Ok(Some(llm_result))) => {
                    llm_ms = Some(l0.elapsed().as_millis() as u32);
                    llm_status = if llm_result.is_empty() {
                        "empty".into()
                    } else {
                        "ok".into()
                    };
                    llm_out = Some(llm_result.clone());
                    merged.merge(llm_result);
                }
                Ok(Ok(None)) => {
                    llm_ms = Some(l0.elapsed().as_millis() as u32);
                    llm_status = "empty".into();
                }
                Ok(Err(e)) => {
                    llm_ms = Some(l0.elapsed().as_millis() as u32);
                    llm_status = format!("error: {e}");
                    debug!(error = %e, "llm analyze error");
                }
                Err(_) => {
                    llm_ms = Some(l0.elapsed().as_millis() as u32);
                    llm_status = "timeout".into();
                    debug!("llm analyze timed out");
                }
            }
        }

        if merged.is_empty() {
            return Ok(None);
        }

        Ok(Some(AnalysisDetail {
            merged,
            heuristic,
            llm: llm_out,
            meta: AnalysisMeta {
                llm_configured: self.llm_configured,
                llm_status,
                heuristic_ms,
                llm_ms,
            },
        }))
    }
}

pub fn create_analyzer(config: &ThemisConfig) -> Arc<dyn AnalysisProvider> {
    if !config.analysis_enabled {
        return Arc::new(NoopAnalysis);
    }

    let llm = LlmAnalyzer::from_config(config);
    let llm_configured = llm.is_some();
    if !llm_configured {
        debug!("analysis: heuristic only (set FOUNDRY_* for LLM glosses/answers)");
    }

    Arc::new(CompositeAnalyzer {
        llm,
        llm_configured,
        llm_timeout: Duration::from_secs(12),
    })
}
