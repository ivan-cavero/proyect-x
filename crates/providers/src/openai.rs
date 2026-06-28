//! OpenAI / API-compatible provider.
//!
//! Supports: chat completions (non-streaming + streaming), embeddings.
//! Uses reqwest for HTTP, handles retry + timeout.

use async_trait::async_trait;
use project_x_agent_traits::provider::*;
use project_x_shared::types::{ModelInfo, TokenUsage};
use tokio::sync::mpsc;

/// OpenAI / API-compatible provider.
pub struct OpenAIProvider {
    /// HTTP client (shared connection pool).
    client: reqwest::Client,
    /// API base URL (e.g., "https://api.openai.com/v1").
    base_url: String,
    /// API key.
    api_key: String,
    /// Default model name.
    model: String,
    /// Max retries on transient errors.
    max_retries: u32,
}

impl OpenAIProvider {
    /// Create a new OpenAI provider.
    pub fn new(
        api_key: String,
        model: String,
        base_url: Option<String>,
        timeout: Option<std::time::Duration>,
        max_retries: Option<u32>,
    ) -> Self {
        let client = reqwest::Client::builder()
            .timeout(timeout.unwrap_or(std::time::Duration::from_secs(120)))
            .build()
            .expect("Failed to build HTTP client");

        Self {
            client,
            base_url: base_url.unwrap_or_else(|| "https://api.openai.com/v1".to_string()),
            api_key,
            model,
            max_retries: max_retries.unwrap_or(3),
        }
    }

    /// Build the chat completions URL.
    fn chat_url(&self) -> String {
        format!("{}/chat/completions", self.base_url)
    }

    /// Build the embeddings URL.
    fn embed_url(&self) -> String {
        format!("{}/embeddings", self.base_url)
    }

    /// Send a request with retry on transient errors.
    async fn send_with_retry(
        &self,
        request: reqwest::RequestBuilder,
    ) -> Result<reqwest::Response, project_x_shared::error::ProjectXError> {
        let mut attempts = 0;
        loop {
            let req = request
                .try_clone()
                .expect("Request must be cloneable for retry");
            match req.send().await {
                Ok(resp) if resp.status().is_success() => return Ok(resp),
                Ok(resp) if resp.status().as_u16() == 429 => {
                    // Rate limited — extract Retry-After
                    let retry_after = resp
                        .headers()
                        .get("retry-after")
                        .and_then(|v| v.to_str().ok())
                        .and_then(|s| s.parse::<u64>().ok())
                        .unwrap_or(5);
                    tracing::warn!("Rate limited, retrying in {}s", retry_after);
                    tokio::time::sleep(std::time::Duration::from_secs(retry_after)).await;
                    attempts += 1;
                }
                Ok(resp) => {
                    // Non-retryable error
                    let status = resp.status();
                    let body = resp.text().await.unwrap_or_default();
                    tracing::error!("HTTP {}: {}", status, body);
                    return Err(project_x_shared::error::ProjectXError::ProviderError(
                        format!("HTTP {}: {}", status, body),
                    ));
                }
                Err(e) if e.is_timeout() || e.is_connect() => {
                    attempts += 1;
                    if attempts > self.max_retries {
                        return Err(project_x_shared::error::ProjectXError::ProviderError(
                            format!("Request failed after {} retries: {}", self.max_retries, e),
                        ));
                    }
                    let backoff = std::time::Duration::from_millis(100 * 2u64.pow(attempts - 1));
                    tracing::warn!("Request failed: {}, retrying in {:?}", e, backoff);
                    tokio::time::sleep(backoff).await;
                }
                Err(e) => return Err(project_x_shared::error::ProjectXError::ProviderError(
                    format!("Request error: {}", e),
                )),
            }
        }
    }

    /// Parse the OpenAI chat completions response.
    fn parse_chat_response(
        &self,
        value: serde_json::Value,
    ) -> Result<ChatResponse, String> {
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
}

#[async_trait]
impl LLMProvider for OpenAIProvider {
    /// Send a non-streaming chat completion request.
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
            "top_p": config.top_p,
            "stop": config.stop_sequences,
            "presence_penalty": config.presence_penalty,
            "frequency_penalty": config.frequency_penalty,
        });

        let request = self
            .client
            .post(self.chat_url())
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body);

        let response = self.send_with_retry(request).await.map_err(|e| {
            project_x_shared::error::ProjectXError::ProviderError(format!("OpenAI API error: {}", e))
        })?;

        let value: serde_json::Value = response.json().await.map_err(|e| {
            project_x_shared::error::ProjectXError::ProviderError(format!("Failed to parse response: {}", e))
        })?;

        self.parse_chat_response(value).map_err(|e| {
            project_x_shared::error::ProjectXError::ProviderError(e)
        })
    }

    /// Send a streaming chat completion request.
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
            project_x_shared::error::ProjectXError::ProviderError(format!("OpenAI stream error: {}", e))
        })?;

        let (tx, rx) = mpsc::channel::<StreamChunk>(256);

        // Spawn a task to read SSE lines and forward chunks
        tokio::spawn(async move {
            let mut bytes_stream = response.bytes_stream();
            let mut buffer = String::new();

            use futures_util::StreamExt;

            while let Some(chunk_result) = bytes_stream.next().await {
                match chunk_result {
                    Ok(chunk) => {
                        buffer.push_str(&String::from_utf8_lossy(&chunk));

                        // Process complete lines
                        while let Some(line_end) = buffer.find('\n') {
                            let line = buffer[..line_end].trim().to_string();
                            buffer = buffer[line_end + 1..].to_string();

                            if line.is_empty() || line == "data: [DONE]" {
                                continue;
                            }

                            if let Some(data) = line.strip_prefix("data: ") {
                                if let Ok(value) = serde_json::from_str::<serde_json::Value>(data) {
                                    let delta = &value["choices"][0]["delta"];

                                    if let Some(content) = delta["content"].as_str() {
                                        if !content.is_empty() {
                                            let _ = tx.send(StreamChunk::Delta(content.to_string())).await;
                                        }
                                    }

                                    if let Some(usage) = value.get("usage") {
                                        let tokens = TokenUsage::new(
                                            usage["prompt_tokens"].as_u64().unwrap_or(0) as u32,
                                            usage["completion_tokens"].as_u64().unwrap_or(0) as u32,
                                        );
                                        let _ = tx.send(StreamChunk::Done(tokens)).await;
                                    }

                                    if let Some(finish) = value["choices"][0]["finish_reason"].as_str() {
                                        if finish == "stop" {
                                            // Send empty delta to signal completion
                                            let _ = tx.send(StreamChunk::Delta(String::new())).await;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(StreamChunk::Error(format!("Stream error: {}", e))).await;
                        break;
                    }
                }
            }
        });

        Ok(rx)
    }

    /// Generate embeddings for the given input strings.
    async fn embed(&self, input: &[String]) -> crate::Result<Vec<Vec<f32>>> {
        let body = serde_json::json!({
            "model": "text-embedding-3-small",
            "input": input,
        });

        let request = self
            .client
            .post(self.embed_url())
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body);

        let response = self.send_with_retry(request).await.map_err(|e| {
            project_x_shared::error::ProjectXError::ProviderError(format!("Embedding error: {}", e))
        })?;

        let value: serde_json::Value = response.json().await.map_err(|e| {
            project_x_shared::error::ProjectXError::ProviderError(format!("Failed to parse embedding response: {}", e))
        })?;

        let embeddings: Vec<Vec<f32>> = value["data"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|item| {
                item["embedding"]
                    .as_array()
                    .map(|arr| arr.iter().filter_map(|v| v.as_f64().map(|f| f as f32)).collect())
            })
            .collect();

        Ok(embeddings)
    }

    /// Count tokens using tiktoken-rs.
    fn count_tokens(&self, text: &str) -> usize {
        match tiktoken_rs::cl100k_base() {
            Ok(encoding) => {
                if let Ok((tokens, _size)) = encoding.encode(text, &std::collections::HashSet::new()) {
                    tokens.len()
                } else {
                    text.len() / 4
                }
            }
            Err(_) => text.len() / 4,
        }
    }

    /// Return information about the currently configured model.
    fn model_info(&self) -> ModelInfo {
        let (context_window, max_output, supports_embeddings) = match self.model.as_str() {
            "gpt-5" => (128_000, 16_384, false),
            "gpt-4o" => (128_000, 16_384, false),
            "gpt-4o-mini" => (128_000, 16_384, false),
            "gpt-4-turbo" => (128_000, 4_096, false),
            "gpt-4" => (8_192, 4_096, false),
            "gpt-3.5-turbo" => (16_385, 4_096, false),
            "text-embedding-3-large" => (8_191, 0, true),
            "text-embedding-3-small" => (8_191, 0, true),
            "text-embedding-ada-002" => (8_191, 0, true),
            _ => (128_000, 4_096, false), // Default
        };

        ModelInfo {
            name: self.model.clone(),
            provider: "openai".to_string(),
            context_window,
            hard_limit_pct: 0.7,
            max_output_tokens: max_output,
            supports_streaming: true,
            supports_embeddings,
        }
    }

    /// Return cost information for the configured model.
    fn model_cost(&self) -> ModelCost {
        let (input, output) = match self.model.as_str() {
            "gpt-5" => (2.50 / 1_000_000.0, 10.00 / 1_000_000.0),
            "gpt-4o" => (2.50 / 1_000_000.0, 10.00 / 1_000_000.0),
            "gpt-4o-mini" => (0.15 / 1_000_000.0, 0.60 / 1_000_000.0),
            "gpt-4-turbo" => (10.00 / 1_000_000.0, 30.00 / 1_000_000.0),
            "gpt-4" => (30.00 / 1_000_000.0, 60.00 / 1_000_000.0),
            "text-embedding-3-small" => (0.02 / 1_000_000.0, 0.0),
            "text-embedding-3-large" => (0.13 / 1_000_000.0, 0.0),
            _ => (0.50 / 1_000_000.0, 1.50 / 1_000_000.0), // Default
        };

        ModelCost {
            per_input_token: input,
            per_output_token: output,
            currency: "USD".to_string(),
        }
    }

    /// Return the provider name.
    fn provider_name(&self) -> &str {
        "openai"
    }
}