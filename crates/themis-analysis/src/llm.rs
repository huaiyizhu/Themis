//! Azure OpenAI / Foundry chat completion for structured insights.

use reqwest::Client;
use serde::{Deserialize, Serialize};
use themis_core::{AnalysisResult, QuestionInsight, TermInsight, ThemisConfig, AnalysisContext};
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

    pub async fn analyze(
        &self,
        transcript: &str,
        ctx: &AnalysisContext,
    ) -> anyhow::Result<Option<AnalysisResult>> {
        let text = transcript.trim();
        if text.len() < 6 {
            return Ok(None);
        }

        let url = format!(
            "{}/openai/deployments/{}/chat/completions?api-version=2024-08-01-preview",
            self.endpoint, self.deployment
        );

        let system = "You extract TECH-focused keywords, specialized terminology, and substantively \
challenging questions from live speech transcripts. \
Respond ONLY with valid JSON: \
{\"keywords\":[\"...\"],\"terms\":[{\"term\":\"...\",\"explanation\":\"...\"}],\
\"questions\":[{\"question\":\"...\",\"answer\":\"...\"}]}. \
\
CONTEXT: You receive session summary and/or recent prior lines to infer domain (e.g. AI/ML vs hardware). \
Use that context to disambiguate acronyms — e.g. MCP in an AI/Agent/RAG discussion means Model Context Protocol, \
not chip packaging. Answers must match the session domain. \
\
KEYWORDS (max 8): Prioritize AI/ML, software engineering, cloud/infra, data, security, and adjacent \
technical domains. Include acronyms and product/framework names when domain-specific (RAG, LLM, CUDA, \
Kubernetes). EXCLUDE generic daily words, sports/entertainment, people's names, places, and obvious \
business buzzwords without technical meaning. \
\
TERMS (max 6): Only jargon that a general audience would NOT already understand well — e.g. RLHF, MoE, \
KV cache, embedding space, retrieval reranking. \
Each explanation MUST be in Chinese (简体中文), 1-3 short sentences, covering when possible: \
主要用途、核心作用、常见用法/场景、一个简短例子. Be fast to read — no English-first phrasing, \
no long definitions. English term name may appear in parentheses only if needed. \
EXCLUDE trivial abbreviations everyone knows (API, CPU, WiFi) unless used in a non-obvious technical sense. \
\
QUESTIONS (max 3): Only REAL technical or conceptual questions that need expertise — mechanisms, \
tradeoffs, comparisons, failure modes, architecture, edge cases. Answers: 2-4 sentences in Chinese, factual. \
EXCLUDE rhetorical questions (对吧/是不是/right?), yes/no confirmations, small talk, and trivial \
one-liner definitions. \
If nothing meets this bar, return empty arrays.";

        let user_content = build_analysis_user_content(text, ctx);

        let body = ChatRequest {
            messages: vec![
                ChatMessage {
                    role: "system".into(),
                    content: system.into(),
                },
                ChatMessage {
                    role: "user".into(),
                    content: user_content,
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

    pub async fn summarize_transcript(&self, full_text: &str) -> anyhow::Result<Option<String>> {
        let text = full_text.trim();
        if text.len() < 12 {
            return Ok(None);
        }

        let url = format!(
            "{}/openai/deployments/{}/chat/completions?api-version=2024-08-01-preview",
            self.endpoint, self.deployment
        );

        let system = "你是实时听写助手，负责用简体中文总结目前已知的全部字幕内容。\
输出 3-6 句概括性摘要，覆盖：主题、关键论点、技术要点、结论或待办（若有）。\
要求：总结全文已知信息，不要引用或复述最新一两句话的原话，不要逐句罗列，不要反问，不要 Markdown 标题。\
只输出总结正文，不要前缀。";

        let body = SummarizeChatRequest {
            messages: vec![
                ChatMessage {
                    role: "system".into(),
                    content: system.into(),
                },
                ChatMessage {
                    role: "user".into(),
                    content: format!("Full transcript so far:\n{text}"),
                },
            ],
            temperature: 0.25,
            max_tokens: 420,
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
            warn!(%status, body = %err, "session summary llm failed");
            return Ok(None);
        }

        let parsed: ChatResponse = resp.json().await?;
        let content = parsed
            .choices
            .first()
            .map(|c| c.message.content.trim().to_string())
            .unwrap_or_default();
        if content.is_empty() {
            Ok(None)
        } else {
            Ok(Some(content))
        }
    }

    /// Longer Chinese explanation for a term or question (on demand).
    pub async fn expand_insight_detail(
        &self,
        kind: &str,
        subject: &str,
        brief: &str,
        session_context: &str,
    ) -> anyhow::Result<Option<String>> {
        let subject = subject.trim();
        let brief = brief.trim();
        if subject.is_empty() {
            return Ok(None);
        }

        let (system, user) = match kind {
            "term" => (
                "你是技术听写助手，负责用简体中文给出术语的深入解释。\
输出 4-8 句，覆盖：定义、原理或机制、典型用途、注意事项、1 个简短例子。\
结合会话上下文判断领域（如 AI/软件）；缩写按当前语境解释（如 MCP = Model Context Protocol）。\
不要 Markdown 标题，不要反问，只输出正文。",
                format!(
                    "Session context (if any):\n{}\n\nTerm: {subject}\nBrief explanation already shown:\n{brief}\n\nGive a fuller explanation:",
                    session_context.trim()
                ),
            ),
            "question" => (
                "你是技术听写助手，负责用简体中文深入回答一个问题。\
输出 4-8 句，覆盖：直接回答、关键原理、实践要点或对比、必要时举简短例子。\
结合会话上下文；不要 Markdown 标题，不要反问，只输出正文。",
                format!(
                    "Session context (if any):\n{}\n\nQuestion: {subject}\nBrief answer already shown:\n{brief}\n\nGive a fuller answer:",
                    session_context.trim()
                ),
            ),
            _ => return Ok(None),
        };

        self.chat_plain(system, &user, 680, 0.25).await
    }

    async fn chat_plain(
        &self,
        system: &str,
        user: &str,
        max_tokens: u32,
        temperature: f32,
    ) -> anyhow::Result<Option<String>> {
        let url = format!(
            "{}/openai/deployments/{}/chat/completions?api-version=2024-08-01-preview",
            self.endpoint, self.deployment
        );
        let body = SummarizeChatRequest {
            messages: vec![
                ChatMessage {
                    role: "system".into(),
                    content: system.into(),
                },
                ChatMessage {
                    role: "user".into(),
                    content: user.into(),
                },
            ],
            temperature,
            max_tokens,
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
            warn!(%status, body = %err, "llm expand failed");
            return Ok(None);
        }
        let parsed: ChatResponse = resp.json().await?;
        let content = parsed
            .choices
            .first()
            .map(|c| c.message.content.trim().to_string())
            .unwrap_or_default();
        if content.is_empty() {
            Ok(None)
        } else {
            Ok(Some(content))
        }
    }
}

fn build_analysis_user_content(phrase: &str, ctx: &AnalysisContext) -> String {
    let mut parts = Vec::new();
    if let Some(summary) = ctx
        .session_summary
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        parts.push(format!("Session summary so far:\n{summary}"));
    }
    if let Some(recent) = ctx
        .recent_transcript
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        parts.push(format!("Recent prior transcript:\n{recent}"));
    }
    parts.push(format!("Latest phrase (extract insights primarily from this):\n{phrase}"));
    parts.join("\n\n")
}

#[derive(Serialize)]
struct ChatRequest {
    messages: Vec<ChatMessage>,
    temperature: f32,
    max_tokens: u32,
    response_format: ResponseFormat,
}

#[derive(Serialize)]
struct SummarizeChatRequest {
    messages: Vec<ChatMessage>,
    temperature: f32,
    max_tokens: u32,
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
