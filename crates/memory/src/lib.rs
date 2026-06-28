//! Memory subsystem — three-tier memory architecture.
//!
//! - Hot memory: in-process DashMap + moka cache (session state, context window)
//! - Episodic memory: Qdrant embedded (vector search, cross-session recall)
//! - Consolidated memory: compressed summaries (long-term retention)

pub mod hot;
pub mod episodic;
pub mod consolidated;
pub mod cache;

pub use hot::{HotMemory, Interaction, SessionState, SessionStatus, SlidingWindow, HotMemoryStats};
pub use cache::{LLMCache, CachedResponse, CacheStats};