//! # praxis Shared Types
//!
//! Core type definitions, protocol messages, configuration structs
//! shared across all crates in the workspace.

pub mod types;
pub mod protocol;
pub mod config;
pub mod error;

/// Prelude module: re-exports the most commonly used types.
pub mod prelude {
    pub use crate::config::*;
    pub use crate::error::*;
    pub use crate::protocol::*;
    pub use crate::types::*;
}