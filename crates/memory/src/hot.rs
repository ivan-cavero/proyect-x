//! Hot Memory — DashMap-based session state and sliding window.
//!
//! In-memory state for active sessions. Fast, concurrent, ephemeral.
//! Used for: session state, context windows, per-agent variables.

use dashmap::DashMap;
use std::collections::VecDeque;
use std::sync::Arc;

// ─── Interaction ──────────────────────────────────────────────

/// A single interaction in an agent's context window.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Interaction {
    pub role: String,
    pub content: String,
    pub token_count: u32,
    pub timestamp: String,
}

// ─── Session State ────────────────────────────────────────────

/// State for an active session.
#[derive(Debug, Clone)]
pub struct SessionState {
    pub id: String,
    pub project_id: String,
    pub status: SessionStatus,
    pub goal: String,
    pub current_phase: String,
    pub iteration: u32,
    pub started_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionStatus {
    Active,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

// ─── Sliding Window ───────────────────────────────────────────

/// Configurable sliding window for context management.
#[derive(Debug, Clone)]
pub struct SlidingWindow {
    max_interactions: usize,
    max_tokens: u32,
    window: VecDeque<Interaction>,
    total_tokens: u32,
}

impl SlidingWindow {
    /// Create a new sliding window.
    pub fn new(max_interactions: usize, max_tokens: u32) -> Self {
        Self {
            max_interactions,
            max_tokens,
            window: VecDeque::with_capacity(max_interactions),
            total_tokens: 0,
        }
    }

    /// Push an interaction. Evicts oldest if over limit.
    pub fn push(&mut self, interaction: Interaction) -> Option<Interaction> {
        self.total_tokens += interaction.token_count;
        self.window.push_back(interaction);

        let mut evicted = None;

        // Evict by count
        while self.window.len() > self.max_interactions {
            if let Some(old) = self.window.pop_front() {
                self.total_tokens -= old.token_count;
                evicted = Some(old);
            }
        }

        // Evict by token budget (keep under limit, not just at it)
        while self.total_tokens >= self.max_tokens && !self.window.is_empty() {
            if let Some(old) = self.window.pop_front() {
                self.total_tokens -= old.token_count;
                evicted = Some(old);
            }
        }

        evicted
    }

    /// Get all interactions in the window.
    pub fn interactions(&self) -> &VecDeque<Interaction> {
        &self.window
    }

    /// Current number of interactions.
    pub fn len(&self) -> usize {
        self.window.len()
    }

    /// Current total tokens.
    pub fn tokens(&self) -> u32 {
        self.total_tokens
    }

    /// Maximum interactions allowed.
    pub fn max_interactions(&self) -> usize {
        self.max_interactions
    }

    /// Maximum tokens allowed.
    pub fn max_tokens(&self) -> u32 {
        self.max_tokens
    }

    /// Clear the window.
    pub fn clear(&mut self) {
        self.window.clear();
        self.total_tokens = 0;
    }

    /// Check if window is at capacity.
    pub fn is_full(&self) -> bool {
        self.window.len() >= self.max_interactions || self.total_tokens >= self.max_tokens
    }

    /// Calculate pressure (0.0–1.0) based on token usage.
    pub fn pressure(&self) -> f32 {
        if self.max_tokens == 0 {
            return 0.0;
        }
        (self.total_tokens as f32 / self.max_tokens as f32).min(1.0)
    }

    /// Serialize to JSON for context injection.
    pub fn to_json(&self) -> serde_json::Value {
        let messages: Vec<serde_json::Value> = self.window.iter().map(|i| {
            serde_json::json!({
                "role": i.role,
                "content": i.content,
            })
        }).collect();

        serde_json::json!({
            "messages": messages,
            "token_count": self.total_tokens,
            "max_tokens": self.max_tokens,
            "pressure": self.pressure(),
        })
    }
}

// ─── Hot Memory ───────────────────────────────────────────────

/// In-memory state for active sessions.
pub struct HotMemory {
    /// Session states.
    sessions: DashMap<String, SessionState>,
    /// Context windows per (session_id, agent_id).
    contexts: DashMap<(String, String), SlidingWindow>,
    /// Per-session key-value variables.
    variables: DashMap<(String, String), String>,
    /// Global counters.
    total_sessions: Arc<std::sync::atomic::AtomicU64>,
    total_interactions: Arc<std::sync::atomic::AtomicU64>,
}

impl HotMemory {
    /// Create a new hot memory instance.
    pub fn new() -> Self {
        Self {
            sessions: DashMap::new(),
            contexts: DashMap::new(),
            variables: DashMap::new(),
            total_sessions: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            total_interactions: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    // ─── Session Management ─────────────────────────────────

    /// Create a new session.
    pub fn create_session(&self, session_id: &str, project_id: &str, goal: &str) -> SessionState {
        let state = SessionState {
            id: session_id.to_string(),
            project_id: project_id.to_string(),
            status: SessionStatus::Active,
            goal: goal.to_string(),
            current_phase: "idle".to_string(),
            iteration: 0,
            started_at: chrono::Utc::now().to_rfc3339(),
        };

        self.sessions.insert(session_id.to_string(), state.clone());
        self.total_sessions.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        tracing::info!("Session created: {}", session_id);
        state
    }

    /// Get session state.
    pub fn get_session(&self, session_id: &str) -> Option<SessionState> {
        self.sessions.get(session_id).map(|s| s.clone())
    }

    /// Update session state.
    pub fn update_session<F>(&self, session_id: &str, f: F)
    where
        F: FnOnce(&mut SessionState),
    {
        if let Some(mut state) = self.sessions.get_mut(session_id) {
            f(&mut state);
        }
    }

    /// List all active sessions.
    pub fn active_sessions(&self) -> Vec<SessionState> {
        self.sessions.iter()
            .filter(|s| s.status == SessionStatus::Active)
            .map(|s| s.clone())
            .collect()
    }

    // ─── Context Window Management ──────────────────────────

    /// Push an interaction to an agent's context window.
    pub fn push_interaction(&self, session_id: &str, agent_id: &str, interaction: Interaction) {
        let key = (session_id.to_string(), agent_id.to_string());
        self.contexts.entry(key).or_insert_with(|| {
            SlidingWindow::new(50, 62_720) // 70% of 89,600 (GPT-5)
        }).push(interaction);

        self.total_interactions.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }

    /// Get an agent's context window.
    pub fn get_context(&self, session_id: &str, agent_id: &str) -> Option<SlidingWindow> {
        self.contexts.get(&(session_id.to_string(), agent_id.to_string()))
            .map(|w| w.clone())
    }

    /// Clear an agent's context window.
    pub fn clear_context(&self, session_id: &str, agent_id: &str) {
        if let Some(mut window) = self.contexts.get_mut(&(session_id.to_string(), agent_id.to_string())) {
            window.clear();
        }
    }

    /// Get context pressure for an agent.
    pub fn context_pressure(&self, session_id: &str, agent_id: &str) -> f32 {
        self.contexts
            .get(&(session_id.to_string(), agent_id.to_string()))
            .map(|w| w.pressure())
            .unwrap_or(0.0)
    }

    // ─── Variable Management ────────────────────────────────

    /// Set a session variable.
    pub fn set_variable(&self, session_id: &str, key: &str, value: &str) {
        self.variables.insert(
            (session_id.to_string(), key.to_string()),
            value.to_string(),
        );
    }

    /// Get a session variable.
    pub fn get_variable(&self, session_id: &str, key: &str) -> Option<String> {
        self.variables.get(&(session_id.to_string(), key.to_string()))
            .map(|v| v.clone())
    }

    /// Remove a session variable.
    pub fn remove_variable(&self, session_id: &str, key: &str) {
        self.variables.remove(&(session_id.to_string(), key.to_string()));
    }

    // ─── Statistics ─────────────────────────────────────────

    /// Get total sessions created.
    pub fn total_sessions(&self) -> u64 {
        self.total_sessions.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Get total interactions recorded.
    pub fn total_interactions(&self) -> u64 {
        self.total_interactions.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Get memory statistics.
    pub fn stats(&self) -> HotMemoryStats {
        HotMemoryStats {
            active_sessions: self.sessions.len(),
            total_sessions: self.total_sessions(),
            total_interactions: self.total_interactions(),
            total_variables: self.variables.len(),
            total_context_windows: self.contexts.len(),
        }
    }
}

impl Default for HotMemory {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Stats ────────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HotMemoryStats {
    pub active_sessions: usize,
    pub total_sessions: u64,
    pub total_interactions: u64,
    pub total_variables: usize,
    pub total_context_windows: usize,
}

// ─── Tests ────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sliding_window_push() {
        let mut window = SlidingWindow::new(3, 1000);

        for i in 0..5 {
            window.push(Interaction {
                role: "user".to_string(),
                content: format!("Message {}", i),
                token_count: 10,
                timestamp: chrono::Utc::now().to_rfc3339(),
            });
        }

        assert_eq!(window.len(), 3);
        assert_eq!(window.total_tokens, 30);
    }

    #[test]
    fn test_sliding_window_token_eviction() {
        let mut window = SlidingWindow::new(10, 50);

        window.push(Interaction {
            role: "user".to_string(),
            content: "Hello".to_string(),
            token_count: 20,
            timestamp: chrono::Utc::now().to_rfc3339(),
        });

        window.push(Interaction {
            role: "assistant".to_string(),
            content: "Hi".to_string(),
            token_count: 20,
            timestamp: chrono::Utc::now().to_rfc3339(),
        });

        assert_eq!(window.len(), 2);

        window.push(Interaction {
            role: "user".to_string(),
            content: "How are you?".to_string(),
            token_count: 30,
            timestamp: chrono::Utc::now().to_rfc3339(),
        });

        // Should evict first two (20+20=40 > 50-30=20)
        assert_eq!(window.len(), 1);
    }

    #[test]
    fn test_sliding_window_pressure() {
        let mut window = SlidingWindow::new(100, 100);

        window.push(Interaction {
            role: "user".to_string(),
            content: "test".to_string(),
            token_count: 50,
            timestamp: chrono::Utc::now().to_rfc3339(),
        });

        assert!((window.pressure() - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_hot_memory_session() {
        let mem = HotMemory::new();

        let session = mem.create_session("s1", "p1", "test goal");
        assert_eq!(session.status, SessionStatus::Active);

        let retrieved = mem.get_session("s1").unwrap();
        assert_eq!(retrieved.goal, "test goal");

        mem.update_session("s1", |s| {
            s.current_phase = "planning".to_string();
        });

        let updated = mem.get_session("s1").unwrap();
        assert_eq!(updated.current_phase, "planning");
    }

    #[test]
    fn test_hot_memory_context() {
        let mem = HotMemory::new();

        mem.push_interaction("s1", "coder", Interaction {
            role: "user".to_string(),
            content: "Hello".to_string(),
            token_count: 5,
            timestamp: chrono::Utc::now().to_rfc3339(),
        });

        let ctx = mem.get_context("s1", "coder").unwrap();
        assert_eq!(ctx.len(), 1);
        assert!(mem.context_pressure("s1", "coder") > 0.0);
    }

    #[test]
    fn test_hot_memory_variables() {
        let mem = HotMemory::new();

        mem.set_variable("s1", "phase", "planning");
        assert_eq!(mem.get_variable("s1", "phase"), Some("planning".to_string()));

        mem.remove_variable("s1", "phase");
        assert_eq!(mem.get_variable("s1", "phase"), None);
    }

    #[test]
    fn test_hot_memory_stats() {
        let mem = HotMemory::new();
        mem.create_session("s1", "p1", "goal");
        mem.create_session("s2", "p1", "goal");

        let stats = mem.stats();
        assert_eq!(stats.active_sessions, 2);
        assert_eq!(stats.total_sessions, 2);
    }
}