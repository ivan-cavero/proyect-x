//! Google Gemini provider — OpenAI-compatible endpoint.
//!
//! Uses Gemini's OpenAI-compatible API: https://ai.google.dev/gemini-api/docs/openai
//! Endpoint: https://generativelanguage.googleapis.com/v1beta/openai/

use async_trait::async_trait;
use praxis_agent_traits::provider::*;
use praxis_shared::types::{ModelInfo, TokenUsage};
use tokio::sync::mpsc;

/// Google Gemini provider (via OpenAI-compatible endpoint).
pub struct GeminiProvider {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
    model: String,
    max_retries: u32,
}

impl GeminiProvider {
    pub fn new(
        api_key: String,
        model: String,
        base_url: Option<String>,
        timeout: Option<std::time::Duration>,
        max_retries: Option<u32>,
    ) -> Result<Self, crate::ProviderInitError> {
        let client = reqwest::Client::builder()
            .timeout(timeout.unwrap_or(std::time::Duration::from_secs(120)))
            .build()
            .map_err(|error| crate::ProviderInitError(format!("Failed to build HTTP client: {error}")))?;

        Ok(Self {
            client,
            base_url: base_url.unwrap_or_else(|| {
                "https://generativelanguage.googleapis.com/v1beta/openai".to_string()
            }),
            api_key,
            model,
            max_retries: max_retries.unwrap_or(3),
        })
    }

    fn chat_url(&self) -> String {
        format!("{}/chat/completions", self.base_url)
    }

    async fn send_with_retry(
        &self,
        request: reqwest::RequestBuilder,
    ) -> Result<reqwest::Response, praxis_shared::error::ProjectXError> {
        let mut attempts = 0;
        loop {
            let req = request.try_clone()
                .ok_or_else(|| praxis_shared::error::ProjectXError::Internal(
                    "Request body exceeded maximum size for retry cloning".to_string()
                ))?;
            match req.send().await {
                Ok(resp) if resp.status().is_success() => return Ok(resp),
                Ok(resp) if resp.status().as_u16() == 429 => {
                    let retry_after = resp
                        .headers()
                        .get("retry-after")
                        .and_then(|v| v.to_str().ok())
                        .and_then(|s| s.parse::<u64>().ok())
                        .unwrap_or(5);
                    tracing::warn!("Gemini rate limited, retrying in {}s", retry_after);
                    tokio::time::sleep(std::time::Duration::from_secs(retry_after)).await;
                    attempts += 1;
                }
                Ok(resp) => {
                    let status = resp.status();
                    let body = resp.text().await.unwrap_or_default();
                    tracing::error!("Gemini HTTP {}: {}", status, body);
                    return Err(praxis_shared::error::ProjectXError::ProviderError(
                        format!("Gemini HTTP {}: {}", status, body),
                    ));
                }
                Err(e) if e.is_timeout() || e.is_connect() => {
                    attempts += 1;
                    if attempts > self.max_retries {
                        return Err(praxis_shared::error::ProjectXError::ProviderError(
                            format!("Gemini failed after {} retries: {}", self.max_retries, e),
                        ));
                    }
                    let backoff = std::time::Duration::from_millis(100 * 2u64.pow(attempts - 1));
                    tokio::time::sleep(backoff).await;
                }
                Err(e) => return Err(praxis_shared::error::ProjectXError::ProviderError(
                    format!("Gemini request error: {}", e),
                )),
            }
        }
    }
}

#[async_trait]
impl LLMProvider for GeminiProvider {
    async fn chat(
        &self,
        messages: &[ChatMessage],
        config: &ChatConfig,
    ) -> crate::Result<ChatResponse> {
        let body = serde_json::json!({
            "model": config.model,
            "messages": messages.iter().map(|m| serde_json::json!({
                "role": match m.role {
                    ChatRole::System => "system",
                    ChatRole::User => "user",
                    ChatRole::Assistant => "assistant",
                    ChatRole::Tool => "tool",
                },
                "content": m.content,
            })).collect::<Vec<_>>(),
            "temperature": config.temperature,
            "max_tokens": config.max_tokens,
        });

        let request = self
            .client
            .post(self.chat_url())
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body);

        let response = self.send_with_retry(request).await.map_err(|e| {
            praxis_shared::error::ProjectXError::ProviderError(format!("Gemini API error: {}", e))
        })?;

        let value: serde_json::Value = response.json().await.map_err(|e| {
            praxis_shared::error::ProjectXError::ProviderError(format!("Gemini parse error: {}", e))
        })?;

        let content = value["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let finish_reason = value["choices"][0]["finish_reason"]
            .as_str()
            .unwrap_or("stop")
            .to_string();

        let usage = TokenUsage::new(
            value["usage"]["prompt_tokens"].as_u64().unwrap_or(0) as u32,
            value["usage"]["completion_tokens"].as_u64().unwrap_or(0) as u32,
        );

        Ok(ChatResponse {
            content,
            finish_reason,
            usage,
            model: value["model"].as_str().unwrap_or(&self.model).to_string(),
        })
    }

    async fn stream(
        &self,
        messages: &[ChatMessage],
        config: &ChatConfig,
    ) -> crate::Result<StreamReceiver> {
        let body = serde_json::json!({
            "model": config.model,
            "messages": messages.iter().map(|m| serde_json::json!({
                "role": match m.role {
                    ChatRole::System => "system",
                    ChatRole::User => "user",
                    ChatRole::Assistant => "assistant",
                    ChatRole::Tool => "tool",
                },
                "content": m.content,
            })).collect::<Vec<_>>(),
            "temperature": config.temperature,
            "max_tokens": config.max_tokens,
            "stream": true,
        });

        let request = self
            .client
            .post(self.chat_url())
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body);

        let response = self.send_with_retry(request).await.map_err(|e| {
            praxis_shared::error::ProjectXError::ProviderError(format!("Gemini stream error: {}", e))
        })?;

        let (tx, rx) = mpsc::channel::<StreamChunk>(256);

        tokio::spawn(async move {
            let mut bytes_stream = response.bytes_stream();
            let mut buffer = String::new();
            use futures_util::StreamExt;

            while let Some(chunk_result) = bytes_stream.next().await {
                match chunk_result {
                    Ok(chunk) => {
                        buffer.push_str(&String::from_utf8_lossy(&chunk));
                        while let Some(line_end) = buffer.find('\n') {
                            let line = buffer[..line_end].trim().to_string();
                            buffer = buffer[line_end + 1..].to_string();
                            if line.is_empty() || line == "data: [DONE]" { continue; }
                            if let Some(data) = line.strip_prefix("data: ") {
                                if let Ok(value) = serde_json::from_str::<serde_json::Value>(data) {
                                    if let Some(content) = value["choices"][0]["delta"]["content"].as_str() {
                                        if !content.is_empty() {
                                            let _ = tx.send(StreamChunk::Delta(content.to_string())).await;
                                        }
                                    }
                                    if let Some(finish) = value["choices"][0]["finish_reason"].as_str() {
                                        if finish == "stop" {
                                            let _ = tx.send(StreamChunk::Done(TokenUsage::new(0, 0))).await;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(StreamChunk::Error(format!("Gemini stream error: {}", e))).await;
                        break;
                    }
                }
            }
        });

        Ok(rx)
    }

    async fn embed(&self, _input: &[String]) -> crate::Result<Vec<Vec<f32>>> {
        Err(praxis_shared::error::ProjectXError::ProviderError(
            "Gemini embeddings not implemented. Use OpenAI for embeddings.".to_string(),
        ))
    }

    fn count_tokens(&self, text: &str) -> usize {
        text.len() / 4
    }

    fn model_info(&self) -> ModelInfo {
        ModelInfo {
            name: self.model.clone(),
            provider: "gemini".to_string(),
            context_window: 1_000_000,
            hard_limit_pct: 0.60,
            max_output_tokens: 8_192,
            supports_streaming: true,
            supports_embeddings: false,
        }
    }

    fn model_cost(&self) -> ModelCost {
        let (input, output) = match self.model.as_str() {
            "gemini-2.5-pro" => (1.25 / 1_000_000.0, 10.00 / 1_000_000.0),
            "gemini-2.5-flash" => (0.15 / 1_000_000.0, 0.60 / 1_000_000.0),
            "gemini-1.5-pro" => (1.25 / 1_000_000.0, 5.00 / 1_000_000.0),
            _ => (0.50 / 1_000_000.0, 1.50 / 1_000_000.0),
        };
        ModelCost { per_input_token: input, per_output_token: output, currency: "USD".to_string() }
    }

    fn provider_name(&self) -> &str {
        "gemini"
    }
}
