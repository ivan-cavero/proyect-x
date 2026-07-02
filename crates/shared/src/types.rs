//! Shared type aliases and core enums for the praxis system.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ─── ID Type Aliases ───────────────────────────────────────────

pub type AgentId = String;
pub type SessionId = Uuid;
pub type ProjectId = Uuid;
pub type GoalId = String;
pub type TaskId = Uuid;
pub type PhaseId = String;
pub type ConversationId = Uuid;
pub type EventId = Uuid;

// ─── Priority ──────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Priority {
    Low,
    Normal,
    High,
    Critical,
}

impl Priority {
    pub fn ordering(&self) -> u8 {
        match self {
            Priority::Low => 0,
            Priority::Normal => 1,
            Priority::High => 2,
            Priority::Critical => 3,
        }
    }
}

// ─── Phases ────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Phase {
    Idle,
    Planning,
    Researching,
    Designing,
    Implementing,
    Reviewing,
    Fixing,
    Testing,
    SecurityScan,
    Finalizing,
    Completed,
    Failed,
    Cancelled,
}

impl std::fmt::Display for Phase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

impl Phase {
    /// Get the index of this phase (for ordering).
    pub fn index(&self) -> u8 {
        match self {
            Phase::Idle => 0,
            Phase::Planning => 1,
            Phase::Researching => 2,
            Phase::Designing => 3,
            Phase::Implementing => 4,
            Phase::Reviewing => 5,
            Phase::Fixing => 6,
            Phase::Testing => 7,
            Phase::SecurityScan => 8,
            Phase::Finalizing => 9,
            Phase::Completed => 10,
            Phase::Failed => 11,
            Phase::Cancelled => 12,
        }
    }

    /// Check if this is a terminal phase (no more transitions).
    pub fn is_terminal(&self) -> bool {
        matches!(self, Phase::Completed | Phase::Failed | Phase::Cancelled)
    }

    /// Get the default transitions from this phase.
    pub fn default_transitions(&self) -> Vec<Phase> {
        match self {
            Phase::Idle => vec![Phase::Planning, Phase::Cancelled],
            Phase::Planning => vec![Phase::Researching, Phase::Designing, Phase::Implementing, Phase::Cancelled],
            Phase::Researching => vec![Phase::Designing, Phase::Cancelled],
            Phase::Designing => vec![Phase::Implementing, Phase::Cancelled],
            Phase::Implementing => vec![Phase::Reviewing, Phase::Testing, Phase::Cancelled],
            Phase::Reviewing => vec![Phase::Fixing, Phase::Testing, Phase::Implementing, Phase::Completed, Phase::Cancelled],
            Phase::Fixing => vec![Phase::Reviewing, Phase::Implementing, Phase::Cancelled],
            Phase::Testing => vec![Phase::SecurityScan, Phase::Finalizing, Phase::Reviewing, Phase::Cancelled],
            Phase::SecurityScan => vec![Phase::Fixing, Phase::Finalizing, Phase::Cancelled],
            Phase::Finalizing => vec![Phase::Completed, Phase::Cancelled],
            Phase::Completed => vec![],
            Phase::Failed => vec![],
            Phase::Cancelled => vec![],
        }
    }

    /// Display name for the phase.
    pub fn display_name(&self) -> &str {
        match self {
            Phase::Idle => "Idle",
            Phase::Planning => "Planning",
            Phase::Researching => "Researching",
            Phase::Designing => "Designing",
            Phase::Implementing => "Implementing",
            Phase::Reviewing => "Reviewing",
            Phase::Fixing => "Fixing",
            Phase::Testing => "Testing",
            Phase::SecurityScan => "Security Scan",
            Phase::Finalizing => "Finalizing",
            Phase::Completed => "Completed",
            Phase::Failed => "Failed",
            Phase::Cancelled => "Cancelled",
        }
    }
}

// ─── Transition ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transition {
    pub from: Phase,
    pub to: Phase,
    pub gate: Option<String>,
    pub condition: TransitionCondition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransitionCondition {
    Automatic,
    AllAgentsComplete,
    GatePassed(String),
    UserApproval,
    MaxIterationsReached,
}

// ─── Gate ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateResult {
    pub gate_name: String,
    pub passed: bool,
    pub details: String,
    pub evaluator: String,
}

// ─── Agent Status ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    pub id: AgentId,
    pub role: String,
    pub model: String,
    pub status: AgentStatus,
    pub asi_score: f32,
    pub current_action: String,
    pub context_pressure: f32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentStatus {
    Idle,
    Running,
    Blocked,
    Degraded,
    Failed,
}

// ─── Model Information ─────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub name: String,
    pub provider: String,
    pub context_window: usize,
    pub hard_limit_pct: f32,
    pub max_output_tokens: u32,
    pub supports_streaming: bool,
    pub supports_embeddings: bool,
}

// ─── Token Usage ───────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub total_tokens: u32,
}

impl TokenUsage {
    pub fn new(input: u32, output: u32) -> Self {
        Self {
            input_tokens: input,
            output_tokens: output,
            total_tokens: input + output,
        }
    }
}

// ─── Session State ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionState {
    pub id: SessionId,
    pub project_id: ProjectId,
    pub status: SessionStatus,
    pub goal: String,
    pub current_phase: Phase,
    pub iteration: u32,
    pub started_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionStatus {
    Active,
    Paused,
    Completed,
    Failed,
    Cancelled,
}
