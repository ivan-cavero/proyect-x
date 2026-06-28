//! Loop controller — the main execution loop.
//!
//! Orchestrates: StateMachine → Agent execution → Gate evaluation → Checkpoint → Next phase.
//! Includes limits, divergence detection, and context management hooks.

use crate::machine::phase::{Phase, PhaseTransition, StateMachine};
use crate::machine::gate::{GateRegistry, GateVerdict, ReviewResult};
use std::collections::VecDeque;

// ─── Loop Controller ──────────────────────────────────────────

/// The main loop controller for goal execution.
pub struct LoopController {
    /// The state machine managing phase transitions.
    pub state_machine: StateMachine,
    /// Quality gates for phase transitions.
    pub gates: GateRegistry,
    /// Hard limits for the loop.
    pub limits: Limits,
    /// Current iteration count.
    pub iteration: u32,
    /// Phase-specific iteration counts.
    pub phase_iterations: std::collections::HashMap<Phase, u32>,
    /// Results from the current phase (to feed into gates).
    pub current_results: Vec<ReviewResult>,
    /// Events emitted during the loop (for dashboard/CLI).
    pub events: VecDeque<LoopEvent>,
    /// Whether the loop is running.
    pub running: bool,
    /// Session start time.
    pub started_at: std::time::Instant,
}

impl LoopController {
    /// Create a new loop controller with default limits.
    pub fn new() -> Self {
        Self {
            state_machine: StateMachine::new(),
            gates: GateRegistry::new(),
            limits: Limits::default(),
            iteration: 0,
            phase_iterations: std::collections::HashMap::new(),
            current_results: Vec::new(),
            events: VecDeque::with_capacity(256),
            running: false,
            started_at: std::time::Instant::now(),
        }
    }

    /// Create with custom limits.
    pub fn with_limits(limits: Limits) -> Self {
        Self {
            limits,
            ..Self::new()
        }
    }

    /// Start the loop.
    pub fn start(&mut self) {
        self.running = true;
        self.started_at = std::time::Instant::now();
        self.push_event(LoopEvent::Started {
            timestamp: chrono::Utc::now().to_rfc3339(),
        });
    }

    /// Stop the loop.
    pub fn stop(&mut self) {
        self.running = false;
        self.push_event(LoopEvent::Stopped {
            timestamp: chrono::Utc::now().to_rfc3339(),
        });
    }

    /// Check if any hard limit has been exceeded.
    pub fn check_limits(&self) -> Option<LimitViolation> {
        // Global iteration limit
        if self.iteration >= self.limits.max_iterations_per_goal {
            return Some(LimitViolation::MaxIterationsPerGoal {
                current: self.iteration,
                max: self.limits.max_iterations_per_goal,
            });
        }

        // Phase iteration limit
        let phase_count = self.phase_iterations
            .get(&self.state_machine.current())
            .copied()
            .unwrap_or(0);
        if phase_count > self.limits.max_iterations_per_phase {
            return Some(LimitViolation::MaxIterationsPerPhase {
                phase: self.state_machine.current(),
                current: phase_count,
                max: self.limits.max_iterations_per_phase,
            });
        }

        // Session TTL
        if self.started_at.elapsed() >= std::time::Duration::from_secs(self.limits.session_ttl_seconds) {
            return Some(LimitViolation::SessionTtl {
                elapsed: self.started_at.elapsed(),
                max: std::time::Duration::from_secs(self.limits.session_ttl_seconds),
            });
        }

        // Phase timeout
        if let Some(duration) = self.state_machine.phase_duration() {
            if duration >= std::time::Duration::from_secs(self.limits.phase_timeout_seconds) {
                return Some(LimitViolation::PhaseTimeout {
                    phase: self.state_machine.current(),
                    duration,
                    max: std::time::Duration::from_secs(self.limits.phase_timeout_seconds),
                });
            }
        }

        None
    }

    /// Record review results for the current phase.
    pub fn add_results(&mut self, results: Vec<ReviewResult>) {
        self.current_results.extend(results);
    }

    /// Evaluate all gates for the current phase.
    pub fn evaluate_gates(&mut self) -> Vec<GateVerdict> {
        let phase = self.state_machine.current();
        self.gates.evaluate_phase(&phase, &self.current_results)
    }

    /// Check if all gates pass.
    pub fn all_gates_pass(&mut self) -> bool {
        let verdicts = self.evaluate_gates();
        verdicts.iter().all(|v| v.passed)
    }

    /// Advance to the next phase.
    pub fn advance(&mut self, to: Phase) -> Result<PhaseTransition, String> {
        let transition = self.state_machine.transition(to, self.iteration)?;

        self.push_event(LoopEvent::PhaseChanged {
            from: transition.from,
            to: transition.to,
            iteration: self.iteration,
            timestamp: chrono::Utc::now().to_rfc3339(),
        });

        // Reset phase-specific counters for the new phase
        self.phase_iterations.entry(to).or_insert(0);
        self.current_results.clear();

        Ok(transition)
    }

    /// Increment the global iteration counter.
    pub fn increment_iteration(&mut self) {
        self.iteration += 1;
        // Also increment phase-specific counter
        let phase = self.state_machine.current();
        *self.phase_iterations.entry(phase).or_insert(0) += 1;
        self.push_event(LoopEvent::Iteration {
            iteration: self.iteration,
            phase,
            timestamp: chrono::Utc::now().to_rfc3339(),
        });
    }

    /// Check for cycle detection.
    pub fn detect_cycle(&self) -> bool {
        self.state_machine.detect_cycle(self.limits.cycle_detection_window)
    }

    /// Get the current phase info.
    pub fn phase_info(&self) -> PhaseInfo {
        PhaseInfo {
            current: self.state_machine.current(),
            iteration: self.iteration,
            phase_iteration: self.phase_iterations
                .get(&self.state_machine.current())
                .copied()
                .unwrap_or(0),
            elapsed: self.started_at.elapsed(),
            valid_transitions: self.state_machine.valid_transitions(),
        }
    }

    /// Push an event to the event queue.
    fn push_event(&mut self, event: LoopEvent) {
        if self.events.len() >= self.events.capacity() {
            self.events.pop_front();
        }
        self.events.push_back(event);
    }

    /// Drain all pending events.
    pub fn drain_events(&mut self) -> Vec<LoopEvent> {
        self.events.drain(..).collect()
    }
}

impl Default for LoopController {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Limits ───────────────────────────────────────────────────

/// Hard limits for the execution loop.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Limits {
    pub max_iterations_per_goal: u32,
    pub max_iterations_per_phase: u32,
    pub session_ttl_seconds: u64,
    pub phase_timeout_seconds: u64,
    pub cycle_detection_window: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_iterations_per_goal: 50,
            max_iterations_per_phase: 5,
            session_ttl_seconds: 3600,
            phase_timeout_seconds: 300,
            cycle_detection_window: 4,
        }
    }
}

// ─── Limit Violations ─────────────────────────────────────────

/// What limit was exceeded.
#[derive(Debug, Clone)]
pub enum LimitViolation {
    MaxIterationsPerGoal { current: u32, max: u32 },
    MaxIterationsPerPhase { phase: Phase, current: u32, max: u32 },
    SessionTtl { elapsed: std::time::Duration, max: std::time::Duration },
    PhaseTimeout { phase: Phase, duration: std::time::Duration, max: std::time::Duration },
}

impl std::fmt::Display for LimitViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MaxIterationsPerGoal { current, max } => {
                write!(f, "Max iterations per goal reached: {}/{}", current, max)
            }
            Self::MaxIterationsPerPhase { phase, current, max } => {
                write!(f, "Max iterations for {} phase reached: {}/{}", phase, current, max)
            }
            Self::SessionTtl { elapsed, max } => {
                write!(f, "Session TTL exceeded: {:?}/{:?}", elapsed, max)
            }
            Self::PhaseTimeout { phase, duration, max } => {
                write!(f, "Phase {} timeout: {:?}/{:?}", phase, duration, max)
            }
        }
    }
}

// ─── Events ───────────────────────────────────────────────────

/// Events emitted by the loop controller.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum LoopEvent {
    Started { timestamp: String },
    Stopped { timestamp: String },
    Iteration { iteration: u32, phase: Phase, timestamp: String },
    PhaseChanged { from: Phase, to: Phase, iteration: u32, timestamp: String },
    GateEvaluated { gate: String, passed: bool, timestamp: String },
    LimitReached { violation: String, timestamp: String },
    CycleDetected { window: usize, timestamp: String },
    CheckpointSaved { session_id: String, timestamp: String },
}

// ─── Phase Info ───────────────────────────────────────────────

/// Current phase information for display.
#[derive(Debug, Clone)]
pub struct PhaseInfo {
    pub current: Phase,
    pub iteration: u32,
    pub phase_iteration: u32,
    pub elapsed: std::time::Duration,
    pub valid_transitions: Vec<Phase>,
}

// ─── Tests ────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loop_controller_start_stop() {
        let mut ctrl = LoopController::new();
        ctrl.start();
        assert!(ctrl.running);
        ctrl.stop();
        assert!(!ctrl.running);
    }

    #[test]
    fn test_limits_check() {
        let limits = Limits {
            max_iterations_per_goal: 5,
            ..Default::default()
        };
        let mut ctrl = LoopController::with_limits(limits);
        ctrl.start();

        ctrl.increment_iteration(); // 1
        assert!(ctrl.check_limits().is_none());

        ctrl.increment_iteration(); // 2
        assert!(ctrl.check_limits().is_none());

        ctrl.increment_iteration(); // 3
        assert!(ctrl.check_limits().is_none());

        ctrl.increment_iteration(); // 4
        assert!(ctrl.check_limits().is_none());

        ctrl.increment_iteration(); // 5
        assert!(ctrl.check_limits().is_some()); // 5 >= 5 = exceeded
    }

    #[test]
    fn test_phase_iteration_limit() {
        let limits = Limits {
            max_iterations_per_phase: 2,
            ..Default::default()
        };
        let mut ctrl = LoopController::with_limits(limits);
        ctrl.start();

        // Advance to Planning phase
        ctrl.advance(Phase::Planning).unwrap();

        // First iteration in Planning — should be OK
        ctrl.increment_iteration();
        assert!(ctrl.check_limits().is_none(), "1st iteration should be OK");

        // Second iteration in Planning — should be OK
        ctrl.increment_iteration();
        assert!(ctrl.check_limits().is_none(), "2nd iteration should be OK");

        // Third iteration in Planning — exceeds limit (2 >= 2)
        ctrl.increment_iteration();
        assert!(ctrl.check_limits().is_some(), "3rd iteration should exceed limit");
    }

    #[test]
    fn test_advance() {
        let mut ctrl = LoopController::new();
        ctrl.start();

        let transition = ctrl.advance(Phase::Planning).unwrap();
        assert_eq!(transition.from, Phase::Idle);
        assert_eq!(transition.to, Phase::Planning);
        assert_eq!(ctrl.state_machine.current(), Phase::Planning);
    }

    #[test]
    fn test_events() {
        let mut ctrl = LoopController::new();
        ctrl.start();
        ctrl.advance(Phase::Planning).unwrap();

        let events = ctrl.drain_events();
        assert_eq!(events.len(), 2); // Started + PhaseChanged
    }

    #[test]
    fn test_phase_info() {
        let mut ctrl = LoopController::new();
        ctrl.start();
        ctrl.advance(Phase::Planning).unwrap();
        ctrl.increment_iteration();

        let info = ctrl.phase_info();
        assert_eq!(info.current, Phase::Planning);
        assert_eq!(info.iteration, 1);
        assert_eq!(info.phase_iteration, 1);
    }
}