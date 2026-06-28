//! Role-based agent implementations.

pub mod base;
pub mod architect;
pub mod coder;
pub mod reviewer;
pub mod security;
pub mod tester;
pub mod git;
pub mod researcher;
pub mod memory_keeper;
pub mod drift_guard;
pub mod summarizer;

pub use base::{BaseAgent, AgentFactory};