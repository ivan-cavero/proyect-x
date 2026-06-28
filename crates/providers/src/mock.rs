//! Mock provider — deterministic responses for testing.

use async_trait::async_trait;
use project_x_agent_traits::provider::*;
use project_x_shared::types::{ModelInfo, TokenUsage};
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::mpsc;

/// Mock LLM provider for testing.
///
/// Returns configurable responses without hitting any API.
/// Tracks call count, total tokens, and response patterns.
pub struct MockProvider {
    /// Response to return for chat().
    chat_response: String,
    /// Delay before responding (simulates latency).
    delay: std::time::Duration,
    /// Counter for all requests.
    call_count: AtomicUsize,
    /// Total tokens consumed.
    total_tokens: AtomicUsize,
}

impl MockProvider {
    /// Create a mock that returns the given response immediately.
    pub fn simple(response: &str) -> Self {
        Self {
            chat_response: response.to_string(),
            delay: std::time::Duration::ZERO,
            call_count: AtomicUsize::new(0),
            total_tokens: AtomicUsize::new(0),
        }
    }

    /// Create a mock with simulated latency.
    pub fn with_delay(response: &str, delay: std::time::Duration) -> Self {
        Self {
            chat_response: response.to_string(),
            delay,
            call_count: AtomicUsize::new(0),
            total_tokens: AtomicUsize::new(0),
        }
    }

    /// Get total number of requests made.
    pub fn call_count(&self) -> usize {
        self.call_count.load(Ordering::SeqCst)
    }

    /// Get total tokens consumed across all calls.
    pub fn total_tokens(&self) -> usize {
        self.total_tokens.load(Ordering::SeqCst)
    }

    /// Reset all counters.
    pub fn reset(&self) {
        self.call_count.store(0, Ordering::SeqCst);
        self.total_tokens.store(0, Ordering::SeqCst);
    }
}

#[async_trait]
impl LLMProvider for MockProvider {
    async fn chat(
        &self,
        _messages: &[ChatMessage],
        config: &ChatConfig,
    ) -> crate::Result<ChatResponse> {
        self.call_count.fetch_add(1, Ordering::SeqCst);

        if !self.delay.is_zero() {
            tokio::time::sleep(self.delay).await;
        }

        // Estimate tokens: ~4 chars per token
        let input_tokens = _messages
            .iter()
            .map(|m| m.content.len() / 4)
            .sum::<usize>() as u32;
        let output_tokens = (self.chat_response.len() / 4) as u32;

        self.total_tokens
            .fetch_add((input_tokens + output_tokens) as usize, Ordering::SeqCst);

        Ok(ChatResponse {
            content: self.chat_response.clone(),
            finish_reason: "stop".to_string(),
            usage: TokenUsage::new(input_tokens, output_tokens),
            model: config.model.clone(),
        })
    }

    async fn stream(
        &self,
        _messages: &[ChatMessage],
        _config: &ChatConfig,
    ) -> crate::Result<StreamReceiver> {
        self.call_count.fetch_add(1, Ordering::SeqCst);

        if !self.delay.is_zero() {
            tokio::time::sleep(self.delay).await;
        }

        let (tx, rx) = mpsc::channel(256);
        let response = self.chat_response.clone();

        tokio::spawn(async move {
            // Split response into word-sized chunks
            let words: Vec<&str> = response.split_whitespace().collect();
            let mut full_response = String::new();

            for word in &words {
                let chunk = format!("{} ", word);
                full_response.push_str(&chunk);
                let _ = tx.send(StreamChunk::Delta(chunk)).await;
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }

            // Send completion with usage
            let input_tokens = 0; // Mock doesn't track this precisely
            let output_tokens = (full_response.len() / 4) as u32;
            let _ = tx
                .send(StreamChunk::Done(TokenUsage::new(input_tokens, output_tokens)))
                .await;
        });

        Ok(rx)
    }

    async fn embed(&self, input: &[String]) -> crate::Result<Vec<Vec<f32>>> {
        self.call_count.fetch_add(1, Ordering::SeqCst);

        // Return random-ish embeddings (deterministic based on input)
        Ok(input
            .iter()
            .enumerate()
            .map(|(i, _)| {
                (0..1536)
                    .map(|j| ((i * 1536 + j) as f32 / 1536.0).sin())
                    .collect()
            })
            .collect())
    }

    fn count_tokens(&self, text: &str) -> usize {
        // Simple estimation: ~4 chars per token
        text.len() / 4
    }

    fn model_info(&self) -> ModelInfo {
        ModelInfo {
            name: "mock-model".to_string(),
            provider: "mock".to_string(),
            context_window: 128_000,
            hard_limit_pct: 0.7,
            max_output_tokens: 4_096,
            supports_streaming: true,
            supports_embeddings: true,
        }
    }

    fn model_cost(&self) -> ModelCost {
        ModelCost {
            per_input_token: 0.0,
            per_output_token: 0.0,
            currency: "USD".to_string(),
        }
    }

    fn provider_name(&self) -> &str {
        "mock"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_chat() {
        let mock = MockProvider::simple("Hello, world!");
        let config = ChatConfig::default();
        let messages = vec![ChatMessage {
            role: ChatRole::User,
            content: "Say hello".to_string(),
            tool_calls: None,
            tool_call_id: None,
        }];

        let response = mock.chat(&messages, &config).await.unwrap();
        assert_eq!(response.content, "Hello, world!");
        assert_eq!(response.finish_reason, "stop");
        assert!(response.usage.total_tokens > 0);
        assert_eq!(mock.call_count(), 1);
    }

    #[tokio::test]
    async fn test_mock_stream() {
        let mock = MockProvider::simple("Hello world from mock");
        let config = ChatConfig::default();
        let messages = vec![ChatMessage {
            role: ChatRole::User,
            content: "Say hello".to_string(),
            tool_calls: None,
            tool_call_id: None,
        }];

        let mut rx = mock.stream(&messages, &config).await.unwrap();
        let mut full = String::new();

        while let Some(chunk) = rx.recv().await {
            match chunk {
                StreamChunk::Delta(text) => full.push_str(&text),
                StreamChunk::Done(_) => break,
                StreamChunk::Error(e) => panic!("Stream error: {}", e),
            }
        }

        assert!(full.contains("Hello"));
        assert_eq!(mock.call_count(), 1);
    }

    #[tokio::test]
    async fn test_mock_embed() {
        let mock = MockProvider::simple("test");
        let inputs = vec!["hello".to_string(), "world".to_string()];
        let embeddings = mock.embed(&inputs).await.unwrap();

        assert_eq!(embeddings.len(), 2);
        assert_eq!(embeddings[0].len(), 1536);
    }

    #[tokio::test]
    async fn test_mock_token_count() {
        let mock = MockProvider::simple("test");
        let count = mock.count_tokens("Hello, this is a test message");
        assert!(count > 0);
        assert!(count < 20); // ~7 words = ~7-10 tokens
    }

    #[tokio::test]
    async fn test_mock_model_info() {
        let mock = MockProvider::simple("test");
        let info = mock.model_info();
        assert_eq!(info.name, "mock-model");
        assert_eq!(info.provider, "mock");
        assert!(info.supports_streaming);
    }

    #[tokio::test]
    async fn test_mock_reset() {
        let mock = MockProvider::simple("test");
        let config = ChatConfig::default();
        let messages = vec![];

        let _ = mock.chat(&messages, &config).await;
        let _ = mock.chat(&messages, &config).await;
        assert_eq!(mock.call_count(), 2);

        mock.reset();
        assert_eq!(mock.call_count(), 0);
        assert_eq!(mock.total_tokens(), 0);
    }
}