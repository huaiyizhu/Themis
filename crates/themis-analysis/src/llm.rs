//! Azure OpenAI / Foundry chat completion for structured insights.

use reqwest::Client;
use serde::{Deserialize, Serialize};
use themis_core::{AnalysisResult, QuestionInsight, TermInsight, ThemisConfig};
use tracing::{debug, warn};

#[derive(Clone)]
pub struct LlmAnalyzer {
    client: Client,
    endpoint: String,
    api_key: String,
    deployment: String,
}

impl LlmAnalyzer {
    pub fn from_config(config: &ThemisConfig) -> Option<Self> {
        let endpoint = config.foundry_endpoint.as_ref()?.trim_end_matches('/').to_string();
        let api_key = config.foundry_api_key.as_ref()?.clone();
        let deployment = config
            .foundry_deployment
            .clone()
            .unwrap_or_else(|| "gpt-4o-mini".into());
        if endpoint.is_empty() || api_key.is_empty() {
            return None;
        }
        Some(Self {
            client: Client::new(),
            endpoint,
            api_key,
            deployment,
        })
    }

    pub async fn analyze(&self, transcript: &str) -> anyhow::Result<Option<AnalysisResult>> {
        let text = transcript.trim();
        if text.len() < 6 {
            return Ok(None);
        }

        let url = format!(
            "{}/openai/deployments/{}/chat/completions?api-version=2024-08-01-preview",
            self.endpoint, self.deployment
        );

        let system = "You extract keywords, technical terms, and questions from live speech transcripts. \
Respond ONLY with valid JSON matching this schema: \
{\"keywords\":[\"...\"],\"terms\":[{\"term\":\"...\",\"explanation\":\"...\"}],\
\"questions\":[{\"question\":\"...\",\"answer\":\"...\"}]}. \
Rules: keywords max 8; terms max 6 with concise bilingual explanations when helpful; \
questions max 3 with brief factual answers (2-3 sentences). \
If nothing notable, return empty arrays.";

        let body = ChatRequest {
            messages: vec![
                ChatMessage {
                    role: "system".into(),
                    content: system.into(),
                },
                ChatMessage {
                    role: "user".into(),
                    content: format!("Transcript:\n{text}"),
                },
            ],
            temperature: 0.2,
            max_tokens: 800,
            response_format: ResponseFormat {
                r#type: "json_object".into(),
            },
        };

        let resp = self
            .client
            .post(&url)
            .header("api-key", &self.api_key)
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let err = resp.text().await.unwrap_or_default();
            warn!(%status, body = %err, "llm analysis failed");
            return Ok(None);
        }

        let parsed: ChatResponse = resp.json().await?;
        let content = parsed
            .choices
            .first()
            .map(|c| c.message.content.as_str())
            .unwrap_or("");
        debug!(len = content.len(), "llm analysis response");

        let raw: LlmJson = serde_json::from_str(content).unwrap_or_default();
        let mut result = AnalysisResult::default();
        result.keywords = raw.keywords.into_iter().take(8).collect();
        result.terms = raw
            .terms
            .into_iter()
            .take(6)
            .map(|t| TermInsight {
                term: t.term,
                explanation: t.explanation,
            })
            .collect();
        result.questions = raw
            .questions
            .into_iter()
            .take(3)
            .map(|q| QuestionInsight {
                question: q.question,
                answer: q.answer,
            })
            .collect();

        if result.is_empty() {
            Ok(None)
        } else {
            Ok(Some(result))
        }
    }
}

#[derive(Serialize)]
struct ChatRequest {
    messages: Vec<ChatMessage>,
    temperature: f32,
    max_tokens: u32,
    response_format: ResponseFormat,
}

#[derive(Serialize)]
struct ResponseFormat {
    r#type: String,
}

#[derive(Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatMessage,
}

#[derive(Default, Deserialize)]
struct LlmJson {
    #[serde(default)]
    keywords: Vec<String>,
    #[serde(default)]
    terms: Vec<LlmTerm>,
    #[serde(default)]
    questions: Vec<LlmQuestion>,
}

#[derive(Deserialize)]
struct LlmTerm {
    term: String,
    explanation: String,
}

#[derive(Deserialize)]
struct LlmQuestion {
    question: String,
    answer: String,
}
