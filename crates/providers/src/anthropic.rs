//! Anthropic Claude provider — Messages API.
//!
//! Supports Claude 3.5 Sonnet, Claude 3 Opus, Claude 3 Haiku.
//! API docs: https://docs.anthropic.com/en/api/messages

use async_trait::async_trait;
use praxis_agent_traits::provider::*;
use praxis_shared::types::{ModelInfo, TokenUsage};
use tokio::sync::mpsc;

/// Anthropic Claude provider.
pub struct AnthropicProvider {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
    model: String,
    max_retries: u32,
}

impl AnthropicProvider {
    /// Create a new Anthropic provider.
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
            base_url: base_url.unwrap_or_else(|| "https://api.anthropic.com".to_string()),
            api_key,
            model,
            max_retries: max_retries.unwrap_or(3),
        })
    }

    fn messages_url(&self) -> String {
        format!("{}/v1/messages", self.base_url)
    }

    async fn send_with_retry(
        &self,
        request: reqwest::RequestBuilder,
    ) -> Result<reqwest::Response, praxis_shared::error::ProjectXError> {
        let mut attempts = 0;
        loop {
            let req = request
                .try_clone()
                .expect("Request must be cloneable for retry");
            match req.send().await {
                Ok(resp) if resp.status().is_success() => return Ok(resp),
                Ok(resp) if resp.status().as_u16() == 429 => {
                    let retry_after = resp
                        .headers()
                        .get("retry-after")
                        .and_then(|v| v.to_str().ok())
                        .and_then(|s| s.parse::<u64>().ok())
                        .unwrap_or(5);
                    tracing::warn!("Anthropic rate limited, retrying in {}s", retry_after);
                    tokio::time::sleep(std::time::Duration::from_secs(retry_after)).await;
                    attempts += 1;
                }
                Ok(resp) => {
                    let status = resp.status();
                    let body = resp.text().await.unwrap_or_default();
                    tracing::error!("Anthropic HTTP {}: {}", status, body);
                    return Err(praxis_shared::error::ProjectXError::ProviderError(
                        format!("Anthropic HTTP {}: {}", status, body),
                    ));
                }
                Err(e) if e.is_timeout() || e.is_connect() => {
                    attempts += 1;
                    if attempts > self.max_retries {
                        return Err(praxis_shared::error::ProjectXError::ProviderError(
                            format!("Anthropic request failed after {} retries: {}", self.max_retries, e),
                        ));
                    }
                    let backoff = std::time::Duration::from_millis(100 * 2u64.pow(attempts - 1));
                    tracing::warn!("Anthropic request failed: {}, retrying in {:?}", e, backoff);
                    tokio::time::sleep(backoff).await;
                }
                Err(e) => return Err(praxis_shared::error::ProjectXError::ProviderError(
                    format!("Anthropic request error: {}", e),
                )),
            }
        }
    }
}

#[async_trait]
impl LLMProvider for AnthropicProvider {
    async fn chat(
        &self,
        messages: &[ChatMessage],
        config: &ChatConfig,
    ) -> crate::Result<ChatResponse> {
        // Anthropic uses separate system message
        let mut system_prompt = String::new();
        let anthropic_messages: Vec<serde_json::Value> = messages
            .iter()
            .filter_map(|m| {
                match m.role {
                    ChatRole::System => {
                        system_prompt = m.content.clone();
                        None
                    }
                    ChatRole::User => Some(serde_json::json!({
                        "role": "user",
                        "content": m.content,
                    })),
                    ChatRole::Assistant => Some(serde_json::json!({
                        "role": "assistant",
                        "content": m.content,
                    })),
                    ChatRole::Tool => Some(serde_json::json!({
                        "role": "user",
                        "content": format!("[Tool result]: {}", m.content),
                    })),
                }
            })
            .collect();

        let mut body = serde_json::json!({
            "model": config.model,
            "max_tokens": config.max_tokens,
            "messages": anthropic_messages,
            "temperature": config.temperature,
        });

        if !system_prompt.is_empty() {
            body["system"] = serde_json::json!(system_prompt);
        }

        if let Some(ref stops) = config.stop_sequences {
            body["stop_sequences"] = serde_json::json!(stops);
        }

        let request = self
            .client
            .post(self.messages_url())
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&body);

        let response = self.send_with_retry(request).await.map_err(|e| {
            praxis_shared::error::ProjectXError::ProviderError(format!("Anthropic API error: {}", e))
        })?;

        let value: serde_json::Value = response.json().await.map_err(|e| {
            praxis_shared::error::ProjectXError::ProviderError(format!("Failed to parse Anthropic response: {}", e))
        })?;

        // Parse Anthropic response format
        let content = value["content"]
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|block| block["text"].as_str())
            .unwrap_or("")
            .to_string();

        let finish_reason = value["stop_reason"]
            .as_str()
            .unwrap_or("end_turn")
            .to_string();

        let usage = TokenUsage::new(
            value["usage"]["input_tokens"].as_u64().unwrap_or(0) as u32,
            value["usage"]["output_tokens"].as_u64().unwrap_or(0) as u32,
        );

        let model = value["model"]
            .as_str()
            .unwrap_or(&self.model)
            .to_string();

        Ok(ChatResponse {
            content,
            finish_reason,
            usage,
            model,
        })
    }

    async fn stream(
        &self,
        messages: &[ChatMessage],
        config: &ChatConfig,
    ) -> crate::Result<StreamReceiver> {
        let mut system_prompt = String::new();
        let anthropic_messages: Vec<serde_json::Value> = messages
            .iter()
            .filter_map(|m| {
                match m.role {
                    ChatRole::System => {
                        system_prompt = m.content.clone();
                        None
                    }
                    ChatRole::User => Some(serde_json::json!({
                        "role": "user",
                        "content": m.content,
                    })),
                    ChatRole::Assistant => Some(serde_json::json!({
                        "role": "assistant",
                        "content": m.content,
                    })),
                    ChatRole::Tool => Some(serde_json::json!({
                        "role": "user",
                        "content": format!("[Tool result]: {}", m.content),
                    })),
                }
            })
            .collect();

        let mut body = serde_json::json!({
            "model": config.model,
            "max_tokens": config.max_tokens,
            "messages": anthropic_messages,
            "temperature": config.temperature,
            "stream": true,
        });

        if !system_prompt.is_empty() {
            body["system"] = serde_json::json!(system_prompt);
        }

        let request = self
            .client
            .post(self.messages_url())
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&body);

        let response = self.send_with_retry(request).await.map_err(|e| {
            praxis_shared::error::ProjectXError::ProviderError(format!("Anthropic stream error: {}", e))
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

                            if line.is_empty() {
                                continue;
                            }

                            if let Some(data) = line.strip_prefix("data: ") {
                                if let Ok(value) = serde_json::from_str::<serde_json::Value>(data) {
                                    let event_type = value["type"].as_str().unwrap_or("");

                                    match event_type {
                                        "content_block_delta" => {
                                            if let Some(text) = value["delta"]["text"].as_str() {
                                                if !text.is_empty() {
                                                    let _ = tx.send(StreamChunk::Delta(text.to_string())).await;
                                                }
                                            }
                                        }
                                        "message_stop" => {
                                            let tokens = TokenUsage::new(0, 0);
                                            let _ = tx.send(StreamChunk::Done(tokens)).await;
                                            break;
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(StreamChunk::Error(format!("Anthropic stream error: {}", e))).await;
                        break;
                    }
                }
            }
        });

        Ok(rx)
    }

    async fn embed(&self, _input: &[String]) -> crate::Result<Vec<Vec<f32>>> {
        // Anthropic doesn't have a native embeddings API
        Err(praxis_shared::error::ProjectXError::ProviderError(
            "Anthropic does not support embeddings. Use OpenAI or a dedicated embedding model.".to_string(),
        ))
    }

    fn count_tokens(&self, text: &str) -> usize {
        // Claude uses a similar tokenizer to GPT-4
        text.len() / 4
    }

    fn model_info(&self) -> ModelInfo {
        let (context_window, max_output) = match self.model.as_str() {
            "claude-sonnet-4-20250514" | "claude-3-5-sonnet-20241022" => (200_000, 8_192),
            "claude-3-opus-20240229" => (200_000, 4_096),
            "claude-3-haiku-20240307" => (200_000, 4_096),
            _ => (200_000, 4_096),
        };

        ModelInfo {
            name: self.model.clone(),
            provider: "anthropic".to_string(),
            context_window,
            hard_limit_pct: 0.75,
            max_output_tokens: max_output,
            supports_streaming: true,
            supports_embeddings: false,
        }
    }

    fn model_cost(&self) -> ModelCost {
        let (input, output) = match self.model.as_str() {
            "claude-sonnet-4-20250514" | "claude-3-5-sonnet-20241022" => (3.00 / 1_000_000.0, 15.00 / 1_000_000.0),
            "claude-3-opus-20240229" => (15.00 / 1_000_000.0, 75.00 / 1_000_000.0),
            "claude-3-haiku-20240307" => (0.25 / 1_000_000.0, 1.25 / 1_000_000.0),
            _ => (3.00 / 1_000_000.0, 15.00 / 1_000_000.0),
        };

        ModelCost {
            per_input_token: input,
            per_output_token: output,
            currency: "USD".to_string(),
        }
    }

    fn provider_name(&self) -> &str {
        "anthropic"
    }
}
