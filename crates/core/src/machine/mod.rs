//! State machine: phase definitions, transitions, gates.

pub mod phase;
pub mod gate;
pub mod transition;

pub use phase::{Phase, PhaseTransition, StateMachine};
pub use gate::{Gate, GateEvaluator, GateRegistry, GateVerdict, ReviewResult, ReviewComment, Severity};
pub use transition::{Transition, TransitionCondition};