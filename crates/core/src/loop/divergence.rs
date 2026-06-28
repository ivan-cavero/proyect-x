//! Divergence detection — oscillation, repetition, stagnation.
//!
//! Monitors agent output patterns and detects when the system
//! is going in circles or stuck.

use std::collections::VecDeque;

// ─── Divergence Detector ──────────────────────────────────────

/// Detects divergence patterns in agent outputs.
pub struct DivergenceDetector {
    /// Rolling window of output hashes.
    window: VecDeque<u64>,
    /// Maximum window size.
    max_window: usize,
    /// Repetition counter for identical consecutive outputs.
    repetition_count: u32,
    /// Maximum allowed repetitions before alert.
    max_repetitions: u32,
    /// History of divergence alerts.
    alerts: Vec<DivergenceAlert>,
}

/// A divergence alert.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DivergenceAlert {
    pub kind: DivergenceKind,
    pub severity: DivergenceSeverity,
    pub details: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DivergenceKind {
    /// Pattern A→B→A→B detected.
    Oscillation,
    /// Same output repeated N times.
    Repetition,
    /// Output hasn't changed significantly in M iterations.
    Stagnation,
    /// Output is empty or too short.
    InsufficientContent,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DivergenceSeverity {
    Warning,
    Critical,
}

impl DivergenceDetector {
    /// Create a new detector.
    pub fn new(max_window: usize, max_repetitions: u32) -> Self {
        Self {
            window: VecDeque::with_capacity(max_window),
            max_window,
            repetition_count: 0,
            max_repetitions,
            alerts: Vec::new(),
        }
    }

    /// Record an output and check for divergence.
    pub fn record_output(&mut self, content: &str) -> Option<DivergenceAlert> {
        let hash = self.hash_output(content);

        // Check for insufficient content
        if content.trim().len() < 10 {
            let alert = DivergenceAlert {
                kind: DivergenceKind::InsufficientContent,
                severity: DivergenceSeverity::Warning,
                details: format!("Output too short: {} chars", content.len()),
                timestamp: chrono::Utc::now().to_rfc3339(),
            };
            self.alerts.push(alert.clone());
            return Some(alert);
        }

        // Check for repetition
        if let Some(&last_hash) = self.window.back() {
            if hash == last_hash {
                self.repetition_count += 1;
                if self.repetition_count >= self.max_repetitions {
                    let alert = DivergenceAlert {
                        kind: DivergenceKind::Repetition,
                        severity: DivergenceSeverity::Critical,
                        details: format!("Same output repeated {} times", self.repetition_count),
                        timestamp: chrono::Utc::now().to_rfc3339(),
                    };
                    self.alerts.push(alert.clone());
                    self.repetition_count = 0; // Reset to avoid spam
                    return Some(alert);
                }
            } else {
                self.repetition_count = 0;
            }
        }

        // Add to window
        self.window.push_back(hash);
        if self.window.len() > self.max_window {
            self.window.pop_front();
        }

        // Check for oscillation (A→B→A→B pattern)
        if self.window.len() >= 4 {
            let window: Vec<u64> = self.window.iter().copied().collect();
            let len = window.len();
            if window[len-1] == window[len-3] && window[len-2] == window[len-4] && window[len-1] != window[len-2] {
                let alert = DivergenceAlert {
                    kind: DivergenceKind::Oscillation,
                    severity: DivergenceSeverity::Critical,
                    details: format!(
                        "Oscillation detected: output alternates between two states ({} iterations)",
                        self.window.len()
                    ),
                    timestamp: chrono::Utc::now().to_rfc3339(),
                };
                self.alerts.push(alert.clone());
                return Some(alert);
            }
        }

        // Check for stagnation (no significant change in last N outputs)
        if self.window.len() >= 5 {
            let window: Vec<u64> = self.window.iter().rev().take(5).copied().collect();
            let unique: std::collections::HashSet<u64> = window.into_iter().collect();
            if unique.len() <= 1 && self.window.len() >= 5 {
                let alert = DivergenceAlert {
                    kind: DivergenceKind::Stagnation,
                    severity: DivergenceSeverity::Warning,
                    details: format!("Output stagnation: only {} unique output in last 5 iterations", unique.len()),
                    timestamp: chrono::Utc::now().to_rfc3339(),
                };
                self.alerts.push(alert.clone());
                return Some(alert);
            }
        }

        None
    }

    /// Simple hash function for output comparison.
    fn hash_output(&self, content: &str) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        content.hash(&mut hasher);
        hasher.finish()
    }

    /// Get all alerts.
    pub fn alerts(&self) -> &[DivergenceAlert] {
        &self.alerts
    }

    /// Get the most recent alert.
    pub fn last_alert(&self) -> Option<&DivergenceAlert> {
        self.alerts.last()
    }

    /// Clear history.
    pub fn reset(&mut self) {
        self.window.clear();
        self.repetition_count = 0;
        self.alerts.clear();
    }
}

impl Default for DivergenceDetector {
    fn default() -> Self {
        Self::new(5, 3)
    }
}

// ─── Tests ────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_divergence() {
        let mut detector = DivergenceDetector::new(5, 3);

        assert!(detector.record_output("This is a unique output one").is_none());
        assert!(detector.record_output("This is a unique output two").is_none());
        assert!(detector.record_output("This is a unique output three").is_none());
        assert!(detector.alerts().is_empty());
    }

    #[test]
    fn test_repetition_detection() {
        let mut detector = DivergenceDetector::new(5, 3);

        detector.record_output("First output with unique content here");
        detector.record_output("First output with unique content here");
        detector.record_output("First output with unique content here");
        let alert = detector.record_output("First output with unique content here");

        assert!(alert.is_some());
        assert_eq!(alert.unwrap().kind, DivergenceKind::Repetition);
    }

    #[test]
    fn test_oscillation_detection() {
        let mut detector = DivergenceDetector::new(5, 10);

        // Need 4+ outputs for oscillation detection
        detector.record_output("Output A - unique text one here");
        detector.record_output("Output B - unique text two here");
        detector.record_output("Output A - unique text one here");
        let alert = detector.record_output("Output B - unique text two here");

        assert!(alert.is_some());
        assert_eq!(alert.unwrap().kind, DivergenceKind::Oscillation);
    }

    #[test]
    fn test_stagnation_detection() {
        let mut detector = DivergenceDetector::new(5, 100);

        // Need 5+ outputs with same hash
        for _ in 0..6 {
            detector.record_output("Exact same output content here for testing purposes");
        }

        let alert = detector.last_alert();
        assert!(alert.is_some());
        assert_eq!(alert.unwrap().kind, DivergenceKind::Stagnation);
    }

    #[test]
    fn test_insufficient_content() {
        let mut detector = DivergenceDetector::new(5, 3);
        let alert = detector.record_output("short");
        assert!(alert.is_some());
        assert_eq!(alert.unwrap().kind, DivergenceKind::InsufficientContent);
    }

    #[test]
    fn test_reset() {
        let mut detector = DivergenceDetector::new(5, 3);
        detector.record_output("test");
        detector.record_output("test");
        detector.record_output("test");

        detector.reset();
        assert!(detector.alerts().is_empty());
        assert_eq!(detector.window.len(), 0);
    }
}