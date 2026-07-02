//! Unified error types for the praxis system.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProjectXError {
    // ─── Provider errors ───────────────────────────────────
    #[error("LLM provider error: {0}")]
    ProviderError(String),

    #[error("Provider not found: {0}")]
    ProviderNotFound(String),

    #[error("Model not found: {0}")]
    ModelNotFound(String),

    #[error("API rate limited: retry after {0}s")]
    RateLimited(u64),

    // ─── Actor errors ──────────────────────────────────────
    #[error("Agent not found: {0}")]
    AgentNotFound(String),

    #[error("Agent crashed: {0}")]
    AgentCrashed(String),

    #[error("Actor communication timeout")]
    ActorTimeout,

    // ─── State machine errors ──────────────────────────────
    #[error("Invalid phase transition: {from} → {to}")]
    InvalidTransition { from: String, to: String },

    #[error("Gate failed: {0}")]
    GateFailed(String),

    // ─── MCP errors ────────────────────────────────────────
    #[error("MCP server error: {0}")]
    McpError(String),

    #[error("MCP server not connected: {0}")]
    McpNotConnected(String),

    #[error("Tool execution failed: {0}")]
    ToolExecutionError(String),

    // ─── Persistence errors ────────────────────────────────
    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Event not found")]
    EventNotFound,

    #[error("Migration error: {0}")]
    MigrationError(String),

    // ─── Context errors ────────────────────────────────────
    #[error("Context budget exceeded: {pressure:.1}% > {limit:.1}%")]
    ContextBudgetExceeded { pressure: f32, limit: f32 },

    #[error("Compression failed: {0}")]
    CompressionFailed(String),

    // ─── Config errors ─────────────────────────────────────
    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Missing required config: {0}")]
    MissingConfig(String),

    // ─── Vault errors ──────────────────────────────────────
    #[error("Credential error: {0}")]
    CredentialError(String),

    #[error("API key not found: {0}")]
    ApiKeyNotFound(String),

    // ─── IO ────────────────────────────────────────────────
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    // ─── Internal ──────────────────────────────────────────
    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Not implemented: {0}")]
    NotImplemented(String),
}

impl From<serde_json::Error> for ProjectXError {
    fn from(e: serde_json::Error) -> Self {
        ProjectXError::Internal(format!("JSON error: {}", e))
    }
}

impl From<toml::de::Error> for ProjectXError {
    fn from(e: toml::de::Error) -> Self {
        ProjectXError::ConfigError(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, ProjectXError>;
