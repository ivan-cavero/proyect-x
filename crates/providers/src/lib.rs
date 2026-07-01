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

#[cfg(test)]
mod tests;

pub use mock::MockProvider;
pub use openai::OpenAIProvider;
pub use anthropic::AnthropicProvider;
pub use gemini::GeminiProvider;
pub use ollama::OllamaProvider;
pub use router::ProviderRouter;

// Re-export key types from agent-traits for convenience
pub use praxis_agent_traits::provider::{
    BudgetProfile, ChatConfig, ChatMessage, ChatResponse, ChatRole, LLMProvider, ModelCost,
    ModelTier, StreamChunk, StreamReceiver, ToolCall,
};

/// Result type for provider operations.
pub type Result<T> = std::result::Result<T, praxis_shared::error::ProjectXError>;

/// Errors from provider initialization.
#[derive(Debug, Clone)]
pub struct ProviderInitError(pub String);

impl std::fmt::Display for ProviderInitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Provider init error: {}", self.0)
    }
}

impl std::error::Error for ProviderInitError {}