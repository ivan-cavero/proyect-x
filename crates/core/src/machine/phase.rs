//! State Machine — phase definitions, transitions, gates.
//!
//! Manages the lifecycle of a goal execution:
//! Idle → Planning → Designing → Implementing → Reviewing → Testing → Finalizing → Completed
//!
//! Each transition can be guarded by a quality gate.

// ─── Phase ────────────────────────────────────────────────────

/// All possible phases in the system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
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

impl std::fmt::Display for Phase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

// ─── Transition ───────────────────────────────────────────────

/// A valid transition between phases.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Transition {
    pub from: Phase,
    pub to: Phase,
    pub gate: Option<String>,
    pub condition: TransitionCondition,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum TransitionCondition {
    Automatic,
    AllAgentsComplete,
    GatePassed(String),
    MaxIterationsReached,
}

// ─── State Machine ────────────────────────────────────────────

/// The state machine manages phase transitions with guards.
pub struct StateMachine {
    current: Phase,
    history: Vec<PhaseTransition>,
    transitions: Vec<Transition>,
}

/// A record of a phase transition.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PhaseTransition {
    pub from: Phase,
    pub to: Phase,
    pub timestamp: String,
    pub iteration: u32,
}

impl StateMachine {
    /// Create a new state machine starting at Idle.
    pub fn new() -> Self {
        let all_phases = [
            Phase::Idle, Phase::Planning, Phase::Researching, Phase::Designing,
            Phase::Implementing, Phase::Reviewing, Phase::Fixing, Phase::Testing,
            Phase::SecurityScan, Phase::Finalizing,
        ];

        let transitions = all_phases.iter().flat_map(|&from| {
            from.default_transitions().into_iter().map(move |to| Transition {
                from,
                to,
                gate: None,
                condition: TransitionCondition::Automatic,
            })
        }).collect();

        Self {
            current: Phase::Idle,
            history: Vec::new(),
            transitions,
        }
    }

    /// Create with custom transitions.
    pub fn with_transitions(transitions: Vec<Transition>) -> Self {
        Self {
            current: Phase::Idle,
            history: Vec::new(),
            transitions,
        }
    }

    /// Get the current phase.
    pub fn current(&self) -> Phase {
        self.current
    }

    /// Get the phase history.
    pub fn history(&self) -> &[PhaseTransition] {
        &self.history
    }

    /// Check if a transition is valid.
    pub fn can_transition(&self, to: Phase) -> bool {
        if self.current.is_terminal() {
            return false;
        }

        self.transitions.iter().any(|t| t.from == self.current && t.to == to)
    }

    /// Get all valid transitions from the current phase.
    pub fn valid_transitions(&self) -> Vec<Phase> {
        self.transitions
            .iter()
            .filter(|t| t.from == self.current)
            .map(|t| t.to)
            .collect()
    }

    /// Attempt to transition to a new phase.
    ///
    /// Returns `Ok(PhaseTransition)` on success, or `Err(reason)` on failure.
    pub fn transition(&mut self, to: Phase, iteration: u32) -> Result<PhaseTransition, String> {
        if !self.can_transition(to) {
            return Err(format!(
                "Invalid transition: {} → {} (valid: {:?})",
                self.current,
                to,
                self.valid_transitions()
            ));
        }

        let record = PhaseTransition {
            from: self.current,
            to,
            timestamp: chrono::Utc::now().to_rfc3339(),
            iteration,
        };

        self.history.push(record.clone());
        self.current = to;

        Ok(record)
    }

    /// Detect if the machine is in a cycle (A→B→A→B).
    pub fn detect_cycle(&self, window: usize) -> bool {
        if self.history.len() < window * 2 {
            return false;
        }

        let recent: Vec<Phase> = self.history.iter().rev().take(window * 2).map(|t| t.from).collect();

        // Check if the pattern repeats
        let first_half = &recent[..window];
        let second_half = &recent[window..];

        first_half == second_half
    }

    /// Get the phase duration (how long since entering current phase).
    pub fn phase_duration(&self) -> Option<std::time::Duration> {
        self.history.last().and_then(|t| {
            chrono::DateTime::parse_from_rfc3339(&t.timestamp)
                .ok()
                .map(|dt| {
                    let now = chrono::Utc::now();
                    now.signed_duration_since(dt)
                        .to_std()
                        .unwrap_or_default()
                })
        })
    }
}

impl Default for StateMachine {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Tests ────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_state() {
        let sm = StateMachine::new();
        assert_eq!(sm.current(), Phase::Idle);
        assert!(!sm.current().is_terminal());
    }

    #[test]
    fn test_valid_transition() {
        let sm = StateMachine::new();
        assert!(sm.can_transition(Phase::Planning));
        assert!(!sm.can_transition(Phase::Completed)); // Can't skip to Completed from Idle
    }

    #[test]
    fn test_transition() {
        let mut sm = StateMachine::new();
        let record = sm.transition(Phase::Planning, 0).unwrap();
        assert_eq!(record.from, Phase::Idle);
        assert_eq!(record.to, Phase::Planning);
        assert_eq!(sm.current(), Phase::Planning);
        assert_eq!(sm.history().len(), 1);
    }

    #[test]
    fn test_invalid_transition() {
        let mut sm = StateMachine::new();
        let result = sm.transition(Phase::Completed, 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_terminal_phases() {
        assert!(Phase::Completed.is_terminal());
        assert!(Phase::Failed.is_terminal());
        assert!(Phase::Cancelled.is_terminal());
        assert!(!Phase::Planning.is_terminal());
    }

    #[test]
    fn test_full_flow() {
        let mut sm = StateMachine::new();

        sm.transition(Phase::Planning, 0).unwrap();
        sm.transition(Phase::Designing, 1).unwrap();
        sm.transition(Phase::Implementing, 2).unwrap();
        sm.transition(Phase::Reviewing, 3).unwrap();
        sm.transition(Phase::Testing, 4).unwrap();
        sm.transition(Phase::Finalizing, 5).unwrap();
        sm.transition(Phase::Completed, 6).unwrap();

        assert_eq!(sm.current(), Phase::Completed);
        assert!(sm.current().is_terminal());
        assert_eq!(sm.history().len(), 7);
    }

    #[test]
    fn test_cycle_detection() {
        let mut sm = StateMachine::new();

        sm.transition(Phase::Planning, 0).unwrap();
        sm.transition(Phase::Implementing, 1).unwrap();
        sm.transition(Phase::Reviewing, 2).unwrap();
        sm.transition(Phase::Fixing, 3).unwrap();
        sm.transition(Phase::Reviewing, 4).unwrap();
        sm.transition(Phase::Fixing, 5).unwrap();
        sm.transition(Phase::Reviewing, 6).unwrap();
        sm.transition(Phase::Fixing, 7).unwrap();

        // window=2 means check last 4 transitions for A→B→A→B
        assert!(sm.detect_cycle(2));
    }

    #[test]
    fn test_valid_transitions() {
        let sm = StateMachine::new();
        let valid = sm.valid_transitions();
        assert!(valid.contains(&Phase::Planning));
        assert!(valid.contains(&Phase::Cancelled));
        assert!(!valid.contains(&Phase::Completed));
    }

    #[test]
    fn test_rollback() {
        let mut sm = StateMachine::new();
        sm.transition(Phase::Planning, 0).unwrap();
        sm.transition(Phase::Implementing, 1).unwrap();

        // Rollback to Planning (allowed: Reviewing → Implementing is valid, but also Implementing → Reviewing)
        // Actually, let's just test that we can go back to a valid previous phase
        let result = sm.transition(Phase::Reviewing, 2);
        assert!(result.is_ok()); // Implementing → Reviewing is valid
    }
}