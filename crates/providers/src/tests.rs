//! Tests for provider response parsing and model detection.

#[cfg(test)]
mod anthropic_tests {
    use serde_json::json;

    #[test]
    fn test_anthropic_response_parsing() {
        let response = json!({
            "id": "msg_123",
            "type": "message",
            "role": "assistant",
            "content": [
                {
                    "type": "text",
                    "text": "Hello! How can I help you?"
                }
            ],
            "model": "claude-sonnet-4-20250514",
            "stop_reason": "end_turn",
            "usage": {
                "input_tokens": 10,
                "output_tokens": 20
            }
        });

        // Parse content from Anthropic format
        let content = response["content"]
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|block| block["text"].as_str())
            .unwrap_or("")
            .to_string();

        let stop_reason = response["stop_reason"]
            .as_str()
            .unwrap_or("end_turn")
            .to_string();

        let input_tokens = response["usage"]["input_tokens"].as_u64().unwrap_or(0) as u32;
        let output_tokens = response["usage"]["output_tokens"].as_u64().unwrap_or(0) as u32;

        assert_eq!(content, "Hello! How can I help you?");
        assert_eq!(stop_reason, "end_turn");
        assert_eq!(input_tokens, 10);
        assert_eq!(output_tokens, 20);
    }

    #[test]
    fn test_anthropic_empty_content() {
        let response = json!({
            "content": [],
            "stop_reason": "max_tokens",
            "usage": { "input_tokens": 5, "output_tokens": 0 }
        });

        let content = response["content"]
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|block| block["text"].as_str())
            .unwrap_or("");

        assert!(content.is_empty());
    }

    #[test]
    fn test_anthropic_model_detection() {
        assert!(crate::ProviderRouter::detect_provider("claude-sonnet-4-20250514").is_some());
        assert!(crate::ProviderRouter::detect_provider("claude-3-opus-20240229").is_some());
        assert!(crate::ProviderRouter::detect_provider("gpt-4o").is_some());
    }
}

#[cfg(test)]
mod gemini_tests {
    use serde_json::json;

    #[test]
    fn test_gemini_openai_compatible_response() {
        // Gemini uses OpenAI-compatible format
        let response = json!({
            "choices": [{
                "message": {
                    "content": "Gemini response here",
                    "role": "assistant"
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 15,
                "completion_tokens": 25
            },
            "model": "gemini-2.5-pro"
        });

        let content = response["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let finish = response["choices"][0]["finish_reason"]
            .as_str()
            .unwrap_or("stop")
            .to_string();

        let tokens_in = response["usage"]["prompt_tokens"].as_u64().unwrap_or(0) as u32;
        let tokens_out = response["usage"]["completion_tokens"].as_u64().unwrap_or(0) as u32;

        assert_eq!(content, "Gemini response here");
        assert_eq!(finish, "stop");
        assert_eq!(tokens_in, 15);
        assert_eq!(tokens_out, 25);
    }

    #[test]
    fn test_gemini_model_detection() {
        assert!(crate::ProviderRouter::detect_provider("gemini-2.5-pro").is_some());
        assert!(crate::ProviderRouter::detect_provider("gemini-2.5-flash").is_some());
    }
}

#[cfg(test)]
mod ollama_tests {
    use serde_json::json;

    #[test]
    fn test_ollama_response_parsing() {
        let response = json!({
            "model": "llama3.2",
            "message": {
                "role": "assistant",
                "content": "Ollama local response"
            },
            "done": true,
            "total_duration": 123456789,
            "load_duration": 123456,
            "prompt_eval_count": 10,
            "prompt_eval_duration": 100000,
            "eval_count": 20,
            "eval_duration": 200000
        });

        let content = response["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let done = response["done"].as_bool().unwrap_or(false);
        let prompt_tokens = response["prompt_eval_count"].as_u64().unwrap_or(0) as u32;
        let output_tokens = response["eval_count"].as_u64().unwrap_or(0) as u32;

        assert_eq!(content, "Ollama local response");
        assert!(done);
        assert_eq!(prompt_tokens, 10);
        assert_eq!(output_tokens, 20);
    }

    #[test]
    fn test_ollama_streaming_chunk() {
        let chunk = json!({
            "model": "llama3.2",
            "message": {
                "role": "assistant",
                "content": "Hello"
            },
            "done": false
        });

        let content = chunk["message"]["content"].as_str().unwrap_or("");
        let done = chunk["done"].as_bool().unwrap_or(true);

        assert_eq!(content, "Hello");
        assert!(!done);
    }

    #[test]
    fn test_ollama_model_detection() {
        assert!(crate::ProviderRouter::detect_provider("llama-3.2").is_some());
        assert!(crate::ProviderRouter::detect_provider("codellama-34b").is_some());
        assert!(crate::ProviderRouter::detect_provider("mistral-7b").is_some());
    }
}

#[cfg(test)]
mod router_tests {
    use crate::{ProviderRouter, MockProvider};
    use project_x_agent_traits::provider::ModelTier;
    use std::sync::Arc;

    #[test]
    fn test_router_model_detection_comprehensive() {
        // OpenAI
        assert_eq!(crate::ProviderRouter::detect_provider("gpt-4o"), Some("openai".to_string()));
        assert_eq!(crate::ProviderRouter::detect_provider("gpt-4o-mini"), Some("openai".to_string()));
        assert_eq!(crate::ProviderRouter::detect_provider("text-embedding-3-small"), Some("openai".to_string()));

        // Anthropic
        assert_eq!(crate::ProviderRouter::detect_provider("claude-sonnet-4-20250514"), Some("anthropic".to_string()));
        assert_eq!(crate::ProviderRouter::detect_provider("claude-3-opus-20240229"), Some("anthropic".to_string()));

        // Gemini
        assert_eq!(crate::ProviderRouter::detect_provider("gemini-2.5-pro"), Some("gemini".to_string()));

        // Ollama
        assert_eq!(crate::ProviderRouter::detect_provider("llama-3.2"), Some("ollama".to_string()));
        assert_eq!(crate::ProviderRouter::detect_provider("mistral-7b"), Some("ollama".to_string()));

        // Unknown
        assert_eq!(crate::ProviderRouter::detect_provider("unknown-model"), None);
    }

    #[tokio::test]
    async fn test_router_tier_detection() {
        let mut router = ProviderRouter::new();
        let openai: Arc<dyn project_x_agent_traits::provider::LLMProvider> =
            Arc::new(MockProvider::simple("test"));
        let anthropic: Arc<dyn project_x_agent_traits::provider::LLMProvider> =
            Arc::new(MockProvider::simple("test"));

        router.register("openai", openai, ModelTier::Balanced);
        router.register("anthropic", anthropic, ModelTier::Capable);

        assert_eq!(router.tier_for("gpt-4o"), Some(ModelTier::Balanced));
        assert_eq!(router.tier_for("claude-sonnet-4-20250514"), Some(ModelTier::Capable));
        assert_eq!(router.tier_for("unknown"), None);
    }

    #[tokio::test]
    async fn test_router_multiple_providers() {
        let mut router = ProviderRouter::new();
        let p1: Arc<dyn project_x_agent_traits::provider::LLMProvider> =
            Arc::new(MockProvider::simple("openai"));
        let p2: Arc<dyn project_x_agent_traits::provider::LLMProvider> =
            Arc::new(MockProvider::simple("anthropic"));
        let p3: Arc<dyn project_x_agent_traits::provider::LLMProvider> =
            Arc::new(MockProvider::simple("gemini"));

        router.register("openai", p1, ModelTier::Balanced);
        router.register("anthropic", p2, ModelTier::Capable);
        router.register("gemini", p3, ModelTier::Fast);

        let list = router.list();
        assert_eq!(list.len(), 3);
    }
}
