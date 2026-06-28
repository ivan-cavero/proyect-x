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