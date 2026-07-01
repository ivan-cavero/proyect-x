//! Ollama provider — local models via REST API.
//!
//! API docs: https://github.com/ollama/ollama/blob/main/docs/api.md
//! Default endpoint: http://localhost:11434

use async_trait::async_trait;
use praxis_agent_traits::provider::*;
use praxis_shared::types::{ModelInfo, TokenUsage};
use tokio::sync::mpsc;

/// Ollama provider for local models.
pub struct OllamaProvider {
    client: reqwest::Client,
    base_url: String,
    model: String,
}

impl OllamaProvider {
    pub fn new(model: String, base_url: Option<String>) -> Result<Self, crate::ProviderInitError> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()
            .map_err(|error| crate::ProviderInitError(format!("Failed to build HTTP client: {error}")))?;

        Ok(Self {
            client,
            base_url: base_url.unwrap_or_else(|| "http://localhost:11434".to_string()),
            model,
        })
    }

    fn chat_url(&self) -> String {
        format!("{}/api/chat", self.base_url)
    }
}

#[async_trait]
impl LLMProvider for OllamaProvider {
    async fn chat(
        &self,
        messages: &[ChatMessage],
        config: &ChatConfig,
    ) -> crate::Result<ChatResponse> {
        let ollama_messages: Vec<serde_json::Value> = messages
            .iter()
            .map(|m| serde_json::json!({
                "role": match m.role {
                    ChatRole::System => "system",
                    ChatRole::User => "user",
                    ChatRole::Assistant => "assistant",
                    ChatRole::Tool => "tool",
                },
                "content": m.content,
            }))
            .collect();

        let body = serde_json::json!({
            "model": config.model,
            "messages": ollama_messages,
            "stream": false,
            "options": {
                "temperature": config.temperature,
                "num_predict": config.max_tokens,
            },
        });

        let response = self
            .client
            .post(self.chat_url())
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                praxis_shared::error::ProjectXError::ProviderError(
                    format!("Ollama request failed (is Ollama running?): {}", e),
                )
            })?;

        let value: serde_json::Value = response.json().await.map_err(|e| {
            praxis_shared::error::ProjectXError::ProviderError(format!("Ollama parse error: {}", e))
        })?;

        let content = value["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let usage = TokenUsage::new(
            value["prompt_eval_count"].as_u64().unwrap_or(0) as u32,
            value["eval_count"].as_u64().unwrap_or(0) as u32,
        );

        Ok(ChatResponse {
            content,
            finish_reason: "stop".to_string(),
            usage,
            model: value["model"].as_str().unwrap_or(&self.model).to_string(),
        })
    }

    async fn stream(
        &self,
        messages: &[ChatMessage],
        config: &ChatConfig,
    ) -> crate::Result<StreamReceiver> {
        let ollama_messages: Vec<serde_json::Value> = messages
            .iter()
            .map(|m| serde_json::json!({
                "role": match m.role {
                    ChatRole::System => "system",
                    ChatRole::User => "user",
                    ChatRole::Assistant => "assistant",
                    ChatRole::Tool => "tool",
                },
                "content": m.content,
            }))
            .collect();

        let body = serde_json::json!({
            "model": config.model,
            "messages": ollama_messages,
            "stream": true,
            "options": {
                "temperature": config.temperature,
                "num_predict": config.max_tokens,
            },
        });

        let response = self
            .client
            .post(self.chat_url())
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                praxis_shared::error::ProjectXError::ProviderError(
                    format!("Ollama stream failed: {}", e),
                )
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
                            if line.is_empty() { continue; }

                            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&line) {
                                if let Some(content) = value["message"]["content"].as_str() {
                                    if !content.is_empty() {
                                        let _ = tx.send(StreamChunk::Delta(content.to_string())).await;
                                    }
                                }
                                if value["done"].as_bool().unwrap_or(false) {
                                    let usage = TokenUsage::new(
                                        value["prompt_eval_count"].as_u64().unwrap_or(0) as u32,
                                        value["eval_count"].as_u64().unwrap_or(0) as u32,
                                    );
                                    let _ = tx.send(StreamChunk::Done(usage)).await;
                                    break;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(StreamChunk::Error(format!("Ollama stream error: {}", e))).await;
                        break;
                    }
                }
            }
        });

        Ok(rx)
    }

    async fn embed(&self, _input: &[String]) -> crate::Result<Vec<Vec<f32>>> {
        Err(praxis_shared::error::ProjectXError::ProviderError(
            "Ollama embeddings not implemented. Use OpenAI for embeddings.".to_string(),
        ))
    }

    fn count_tokens(&self, text: &str) -> usize {
        text.len() / 4
    }

    fn model_info(&self) -> ModelInfo {
        ModelInfo {
            name: self.model.clone(),
            provider: "ollama".to_string(),
            context_window: 32_768,
            hard_limit_pct: 0.50,
            max_output_tokens: 4_096,
            supports_streaming: true,
            supports_embeddings: false,
        }
    }

    fn model_cost(&self) -> ModelCost {
        ModelCost {
            per_input_token: 0.0,
            per_output_token: 0.0,
            currency: "LOCAL".to_string(),
        }
    }

    fn provider_name(&self) -> &str {
        "ollama"
    }
}
