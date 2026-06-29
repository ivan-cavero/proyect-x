//! Agent traits: LLMProvider, Tool, Memory, and Persistence backends.
//!
//! These traits define the contracts that all implementations must satisfy.

pub mod provider;
pub mod tool;
pub mod memory;
pub mod persistence;

pub mod prelude {
    pub use crate::memory::MemoryBackend;
    pub use crate::persistence::EventStore;
    pub use crate::provider::LLMProvider;
    pub use crate::tool::Tool;
}

// Re-export the unified error type from shared
pub use project_x_shared::error::ProjectXError;

/// Result type used throughout agent-traits.
pub type Result<T> = std::result::Result<T, ProjectXError>;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ─── Provider Types ────────────────────────────────────────

    #[test]
    fn test_chat_message_creation() {
        let msg = provider::ChatMessage {
            role: provider::ChatRole::User,
            content: "Hello".to_string(),
            tool_calls: None,
            tool_call_id: None,
        };
        assert_eq!(msg.content, "Hello");
    }

    #[test]
    fn test_chat_message_with_tool_calls() {
        let msg = provider::ChatMessage {
            role: provider::ChatRole::Assistant,
            content: "".to_string(),
            tool_calls: Some(vec![provider::ToolCall {
                id: "call-1".to_string(),
                name: "read_file".to_string(),
                arguments: json!({"path": "src/main.rs"}),
            }]),
            tool_call_id: None,
        };
        assert!(msg.tool_calls.is_some());
        assert_eq!(msg.tool_calls.unwrap()[0].name, "read_file");
    }

    #[test]
    fn test_chat_config_default() {
        let config = provider::ChatConfig::default();
        assert_eq!(config.temperature, 0.3);
        assert_eq!(config.max_tokens, 4096);
        assert!(config.top_p.is_none());
    }

    #[test]
    fn test_chat_response_creation() {
        let response = provider::ChatResponse {
            content: "Hello!".to_string(),
            finish_reason: "stop".to_string(),
            usage: project_x_shared::types::TokenUsage::new(10, 5),
            model: "gpt-4o".to_string(),
        };
        assert_eq!(response.content, "Hello!");
        assert_eq!(response.usage.total_tokens, 15);
    }

    #[test]
    fn test_stream_chunk_types() {
        let delta = provider::StreamChunk::Delta("hello".to_string());
        let done = provider::StreamChunk::Done(project_x_shared::types::TokenUsage::new(100, 50));
        let error = provider::StreamChunk::Error("timeout".to_string());

        match delta {
            provider::StreamChunk::Delta(text) => assert_eq!(text, "hello"),
            _ => panic!("Expected Delta"),
        }
        match done {
            provider::StreamChunk::Done(usage) => assert_eq!(usage.total_tokens, 150),
            _ => panic!("Expected Done"),
        }
        match error {
            provider::StreamChunk::Error(msg) => assert_eq!(msg, "timeout"),
            _ => panic!("Expected Error"),
        }
    }

    #[test]
    fn test_model_cost_calculation() {
        let cost = provider::ModelCost {
            per_input_token: 2.50 / 1_000_000.0,
            per_output_token: 10.00 / 1_000_000.0,
            currency: "USD".to_string(),
        };
        let input_cost = cost.per_input_token * 1000.0;
        let output_cost = cost.per_output_token * 500.0;
        assert!((input_cost - 0.0025).abs() < 0.0001);
        assert!((output_cost - 0.005).abs() < 0.0001);
    }

    #[test]
    fn test_model_tier_variants() {
        let fast = provider::ModelTier::Fast;
        let balanced = provider::ModelTier::Balanced;
        let capable = provider::ModelTier::Capable;
        assert_ne!(format!("{:?}", fast), format!("{:?}", balanced));
        assert_ne!(format!("{:?}", capable), format!("{:?}", balanced));
    }

    #[test]
    fn test_budget_profile_variants() {
        let balanced = provider::BudgetProfile::Balanced;
        let generous = provider::BudgetProfile::Generous;
        let aggressive = provider::BudgetProfile::Aggressive;
        let research = provider::BudgetProfile::Research;
        // Just verify they exist and are distinct
        assert_ne!(format!("{:?}", balanced), format!("{:?}", generous));
        assert_ne!(format!("{:?}", aggressive), format!("{:?}", research));
    }

    // ─── Persistence Types ─────────────────────────────────────

    #[test]
    fn test_stored_event_creation() {
        let event = persistence::StoredEvent {
            id: uuid::Uuid::new_v4(),
            aggregate_id: uuid::Uuid::new_v4(),
            aggregate_type: "session".to_string(),
            event_type: "session.created".to_string(),
            payload: json!({"goal": "test"}),
            metadata: json!({"source": "cli"}),
            version: 1,
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        assert_eq!(event.aggregate_type, "session");
        assert_eq!(event.version, 1);
    }

    #[test]
    fn test_stored_event_serialization() {
        let event = persistence::StoredEvent {
            id: uuid::Uuid::new_v4(),
            aggregate_id: uuid::Uuid::new_v4(),
            aggregate_type: "test".to_string(),
            event_type: "test.event".to_string(),
            payload: json!({"data": 42}),
            metadata: json!({}),
            version: 1,
            created_at: "2024-01-01T00:00:00Z".to_string(),
        };
        let json_str = serde_json::to_string(&event).unwrap();
        let parsed: persistence::StoredEvent = serde_json::from_str(&json_str).unwrap();
        assert_eq!(parsed.event_type, "test.event");
        assert_eq!(parsed.payload["data"], 42);
    }

    #[test]
    fn test_stored_snapshot_creation() {
        let snapshot = persistence::StoredSnapshot {
            aggregate_id: uuid::Uuid::new_v4(),
            aggregate_type: "session".to_string(),
            state: json!({"phase": "implementing", "iteration": 5}),
            version: 5,
            updated_at: chrono::Utc::now().to_rfc3339(),
        };
        assert_eq!(snapshot.version, 5);
        assert_eq!(snapshot.state["phase"], "implementing");
    }

    #[test]
    fn test_persistence_mode_variants() {
        let embedded = persistence::PersistenceMode::Embedded;
        let remote = persistence::PersistenceMode::Remote;
        assert_ne!(format!("{:?}", embedded), format!("{:?}", remote));
    }

    // ─── Memory Types ──────────────────────────────────────────

    #[test]
    fn test_memory_entry_creation() {
        let entry = memory::MemoryEntry {
            id: "mem-1".to_string(),
            content: "Important decision: use SQLite".to_string(),
            embedding: Some(vec![0.1, 0.2, 0.3]),
            metadata: std::collections::HashMap::from([
                ("category".to_string(), "decision".to_string()),
            ]),
            timestamp: chrono::Utc::now().to_rfc3339(),
            session_id: "s1".to_string(),
            project_id: "p1".to_string(),
        };
        assert_eq!(entry.id, "mem-1");
        assert!(entry.embedding.is_some());
    }

    #[test]
    fn test_memory_entry_serialization() {
        let entry = memory::MemoryEntry {
            id: "test".to_string(),
            content: "test content".to_string(),
            embedding: None,
            metadata: std::collections::HashMap::new(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            session_id: "s1".to_string(),
            project_id: "p1".to_string(),
        };
        let json_str = serde_json::to_string(&entry).unwrap();
        let parsed: memory::MemoryEntry = serde_json::from_str(&json_str).unwrap();
        assert_eq!(parsed.content, "test content");
    }

    #[test]
    fn test_search_result_creation() {
        let entry = memory::MemoryEntry {
            id: "mem-1".to_string(),
            content: "test".to_string(),
            embedding: None,
            metadata: std::collections::HashMap::new(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            session_id: "s1".to_string(),
            project_id: "p1".to_string(),
        };
        let result = memory::SearchResult {
            entry,
            score: 0.95,
        };
        assert!((result.score - 0.95).abs() < 0.001);
    }

    #[test]
    fn test_memory_stats_creation() {
        let stats = memory::MemoryStats {
            total_entries: 100,
            total_vectors: 80,
            disk_usage_bytes: 1024 * 1024,
        };
        assert_eq!(stats.total_entries, 100);
        assert_eq!(stats.disk_usage_bytes, 1_048_576);
    }

    // ─── Tool Types ────────────────────────────────────────────

    #[test]
    fn test_tool_cost_creation() {
        let cost = tool::ToolCost {
            estimated_tokens: 500,
            estimated_duration_ms: 100,
            requires_network: true,
        };
        assert_eq!(cost.estimated_tokens, 500);
        assert!(cost.requires_network);
    }

    #[test]
    fn test_tool_output_creation() {
        let output = tool::ToolOutput {
            success: true,
            data: json!({"content": "file contents"}),
            duration_ms: 50,
            tokens_used: 100,
        };
        assert!(output.success);
        assert_eq!(output.tokens_used, 100);
    }

    #[test]
    fn test_tool_output_failure() {
        let output = tool::ToolOutput {
            success: false,
            data: json!({"error": "file not found"}),
            duration_ms: 10,
            tokens_used: 0,
        };
        assert!(!output.success);
    }

    // ─── Error Types ───────────────────────────────────────────

    #[test]
    fn test_project_x_error_display() {
        let err = ProjectXError::ProviderError("test error".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("test error"), "Error should contain message: {}", msg);
    }

    #[test]
    fn test_project_x_error_database() {
        let err = ProjectXError::DatabaseError("connection failed".to_string());
        assert!(format!("{}", err).contains("connection failed"));
    }
}
