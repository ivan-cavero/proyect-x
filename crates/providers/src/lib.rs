//! LLM provider implementations.
//!
//! Each provider implements the `LLMProvider` trait from agent-traits.

pub mod openai;
pub mod anthropic;
pub mod gemini;
pub mod ollama;
pub mod openai_compat;
pub mod mock;
pub mod router;

pub use mock::MockProvider;
pub use openai::OpenAIProvider;
pub use router::ProviderRouter;

// Re-export key types from agent-traits for convenience
pub use project_x_agent_traits::provider::{
    BudgetProfile, ChatConfig, ChatMessage, ChatResponse, ChatRole, LLMProvider, ModelCost,
    ModelTier, StreamChunk, StreamReceiver, ToolCall,
};

/// Result type for provider operations.
pub type Result<T> = std::result::Result<T, project_x_shared::error::ProjectXError>;