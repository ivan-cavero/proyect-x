//! Phase transitions and conditions.

use crate::machine::phase::Phase;

/// A transition rule between phases.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Transition {
    pub from: Phase,
    pub to: Phase,
    pub gate: Option<String>,
    pub condition: TransitionCondition,
}

/// Condition that must be met for a transition.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum TransitionCondition {
    Automatic,
    AllAgentsComplete,
    GatePassed(String),
    MaxIterationsReached,
}

impl Default for TransitionCondition {
    fn default() -> Self {
        Self::Automatic
    }
}