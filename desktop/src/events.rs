//! Tauri event types — events emitted from Rust to frontend.

use serde::Serialize;

/// Event emitted when an agent changes phase.
#[derive(Serialize)]
pub struct PhaseChangedEvent {
    pub agent_id: String,
    pub from: String,
    pub to: String,
    pub timestamp: String,
}

/// Event emitted for token usage.
#[derive(Serialize)]
pub struct TokenUsedEvent {
    pub provider: String,
    pub model: String,
    pub input: u32,
    pub output: u32,
    pub timestamp: String,
}

/// Event emitted for context pressure changes.
#[derive(Serialize)]
pub struct ContextPressureEvent {
    pub pressure: f32,
    pub agent_id: String,
    pub action: String,
    pub timestamp: String,
}

/// Event emitted for drift alerts.
#[derive(Serialize)]
pub struct DriftAlertEvent {
    pub asi_score: f32,
    pub severity: String,
    pub details: String,
    pub timestamp: String,
}