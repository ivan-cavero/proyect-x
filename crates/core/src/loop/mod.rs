//! Loop controller — the main execution loop.

pub mod controller;
pub mod limits;
pub mod divergence;

pub use controller::{LoopController, Limits, LimitViolation, LoopEvent, PhaseInfo};
pub use divergence::{DivergenceDetector, DivergenceAlert, DivergenceKind, DivergenceSeverity};