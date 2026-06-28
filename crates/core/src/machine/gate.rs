//! Quality gates — preconditions for phase transitions.
//!
//! A gate evaluates whether a phase output meets quality criteria
//! before allowing transition to the next phase.

use crate::machine::Phase;

// ─── Gate Verdict ─────────────────────────────────────────────

/// The result of evaluating a gate.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GateVerdict {
    pub gate_name: String,
    pub passed: bool,
    pub details: Vec<String>,
    pub timestamp: String,
}

// ─── Gate Evaluator ───────────────────────────────────────────

/// How a gate determines pass/fail.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum GateEvaluator {
    /// All concurrent reviewers must pass.
    AllAgentsPass,
    /// At least N% of reviewers must pass (0.0–1.0).
    MajorityPass(f32),
    /// No critical or blocking errors allowed.
    NoCritical,
    /// Zero errors of any severity.
    NoFailures,
    /// Test coverage above threshold (0.0–1.0).
    CoverageThreshold(f32),
    /// Custom evaluator (for future use).
    Custom(String),
}

// ─── Gate ─────────────────────────────────────────────────────

/// A quality gate that guards a phase transition.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Gate {
    pub name: String,
    pub evaluator: GateEvaluator,
    pub max_retries: u32,
    pub current_retries: u32,
}

impl Gate {
    pub fn new(name: &str, evaluator: GateEvaluator, max_retries: u32) -> Self {
        Self {
            name: name.to_string(),
            evaluator,
            max_retries,
            current_retries: 0,
        }
    }

    /// Evaluate the gate given a set of review results.
    pub fn evaluate(&mut self, results: &[ReviewResult]) -> GateVerdict {
        let verdict = match &self.evaluator {
            GateEvaluator::AllAgentsPass => {
                let all_pass = results.iter().all(|r| r.passed);
                GateVerdict {
                    gate_name: self.name.clone(),
                    passed: all_pass,
                    details: results.iter().map(|r| format!("{}: {}", r.agent, if r.passed { "PASS" } else { "FAIL" })).collect(),
                    timestamp: chrono::Utc::now().to_rfc3339(),
                }
            }

            GateEvaluator::MajorityPass(threshold) => {
                let pass_count = results.iter().filter(|r| r.passed).count();
                let ratio = if results.is_empty() { 0.0 } else { pass_count as f32 / results.len() as f32 };
                GateVerdict {
                    gate_name: self.name.clone(),
                    passed: ratio >= *threshold,
                    details: vec![format!("{}/{} passed ({:.0}%), need {:.0}%", pass_count, results.len(), ratio * 100.0, threshold * 100.0)],
                    timestamp: chrono::Utc::now().to_rfc3339(),
                }
            }

            GateEvaluator::NoCritical => {
                let has_critical = results.iter().any(|r| {
                    r.comments.iter().any(|c| c.severity == Severity::Critical)
                });
                GateVerdict {
                    gate_name: self.name.clone(),
                    passed: !has_critical,
                    details: if has_critical {
                        vec!["Critical findings detected".to_string()]
                    } else {
                        vec!["No critical findings".to_string()]
                    },
                    timestamp: chrono::Utc::now().to_rfc3339(),
                }
            }

            GateEvaluator::NoFailures => {
                let has_any = results.iter().any(|r| !r.passed);
                GateVerdict {
                    gate_name: self.name.clone(),
                    passed: !has_any,
                    details: if has_any {
                        vec!["Failures detected".to_string()]
                    } else {
                        vec!["All checks passed".to_string()]
                    },
                    timestamp: chrono::Utc::now().to_rfc3339(),
                }
            }

            GateEvaluator::CoverageThreshold(threshold) => {
                // Look for coverage in results
                let coverage = results.iter()
                    .find_map(|r| r.coverage)
                    .unwrap_or(0.0);
                GateVerdict {
                    gate_name: self.name.clone(),
                    passed: coverage >= *threshold,
                    details: vec![format!("Coverage: {:.1}%, need {:.1}%", coverage * 100.0, threshold * 100.0)],
                    timestamp: chrono::Utc::now().to_rfc3339(),
                }
            }

            GateEvaluator::Custom(name) => {
                GateVerdict {
                    gate_name: self.name.clone(),
                    passed: true, // Default: pass
                    details: vec![format!("Custom gate '{}' not implemented, defaulting to pass", name)],
                    timestamp: chrono::Utc::now().to_rfc3339(),
                }
            }
        };

        if !verdict.passed {
            self.current_retries += 1;
        }

        verdict
    }

    /// Check if the gate has been exceeded (too many retries).
    pub fn is_exceeded(&self) -> bool {
        self.current_retries >= self.max_retries
    }

    /// Reset retry counter.
    pub fn reset_retries(&mut self) {
        self.current_retries = 0;
    }
}

// ─── Review Result ────────────────────────────────────────────

/// Result from an agent review.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ReviewResult {
    pub agent: String,
    pub passed: bool,
    pub comments: Vec<ReviewComment>,
    pub coverage: Option<f32>,
}

/// A single review comment.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ReviewComment {
    pub severity: Severity,
    pub file: Option<String>,
    pub line: Option<u32>,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

// ─── Gate Registry ────────────────────────────────────────────

/// Registry of all gates in the system.
pub struct GateRegistry {
    gates: Vec<Gate>,
    phase_gates: std::collections::HashMap<Phase, Vec<String>>,
}

impl GateRegistry {
    pub fn new() -> Self {
        Self {
            gates: Vec::new(),
            phase_gates: std::collections::HashMap::new(),
        }
    }

    /// Register a gate for a specific phase transition.
    pub fn register(&mut self, phase: Phase, gate: Gate) {
        self.phase_gates.entry(phase).or_default().push(gate.name.clone());
        self.gates.push(gate);
    }

    /// Get all gates for a phase.
    pub fn gates_for(&self, phase: &Phase) -> Vec<&Gate> {
        self.phase_gates
            .get(phase)
            .map(|names| {
                names.iter()
                    .filter_map(|name| self.gates.iter().find(|g| g.name == *name))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get a mutable reference to a gate by name.
    pub fn get_mut(&mut self, name: &str) -> Option<&mut Gate> {
        self.gates.iter_mut().find(|g| g.name == name)
    }

    /// Evaluate all gates for a phase.
    pub fn evaluate_phase(&mut self, phase: &Phase, results: &[ReviewResult]) -> Vec<GateVerdict> {
        let gate_names: Vec<String> = self.phase_gates
            .get(phase)
            .cloned()
            .unwrap_or_default();

        gate_names.iter().filter_map(|name| {
            self.gates.iter_mut().find(|g| g.name == *name).map(|g| g.evaluate(results))
        }).collect()
    }
}

impl Default for GateRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Tests ────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn passing_result(agent: &str) -> ReviewResult {
        ReviewResult {
            agent: agent.to_string(),
            passed: true,
            comments: vec![],
            coverage: None,
        }
    }

    fn failing_result(agent: &str) -> ReviewResult {
        ReviewResult {
            agent: agent.to_string(),
            passed: false,
            comments: vec![ReviewComment {
                severity: Severity::Medium,
                file: None,
                line: None,
                message: "Issue found".to_string(),
            }],
            coverage: None,
        }
    }

    fn critical_result(agent: &str) -> ReviewResult {
        ReviewResult {
            agent: agent.to_string(),
            passed: false,
            comments: vec![ReviewComment {
                severity: Severity::Critical,
                file: None,
                line: None,
                message: "Critical issue".to_string(),
            }],
            coverage: None,
        }
    }

    #[test]
    fn test_all_agents_pass() {
        let mut gate = Gate::new("test", GateEvaluator::AllAgentsPass, 3);
        let results = vec![passing_result("a"), passing_result("b")];
        let verdict = gate.evaluate(&results);
        assert!(verdict.passed);
    }

    #[test]
    fn test_all_agents_fail() {
        let mut gate = Gate::new("test", GateEvaluator::AllAgentsPass, 3);
        let results = vec![passing_result("a"), failing_result("b")];
        let verdict = gate.evaluate(&results);
        assert!(!verdict.passed);
        assert_eq!(gate.current_retries, 1);
    }

    #[test]
    fn test_majority_pass() {
        let mut gate = Gate::new("test", GateEvaluator::MajorityPass(0.66), 3);
        let results = vec![passing_result("a"), passing_result("b"), failing_result("c")];
        let verdict = gate.evaluate(&results);
        assert!(verdict.passed); // 2/3 = 66.7% >= 66%
    }

    #[test]
    fn test_majority_fail() {
        let mut gate = Gate::new("test", GateEvaluator::MajorityPass(0.66), 3);
        let results = vec![passing_result("a"), failing_result("b"), failing_result("c")];
        let verdict = gate.evaluate(&results);
        assert!(!verdict.passed); // 1/3 = 33.3% < 66%
    }

    #[test]
    fn test_no_critical() {
        let mut gate = Gate::new("test", GateEvaluator::NoCritical, 3);

        let results = vec![failing_result("a")];
        let verdict = gate.evaluate(&results);
        assert!(verdict.passed); // Medium severity, not critical

        let results = vec![critical_result("a")];
        let verdict = gate.evaluate(&results);
        assert!(!verdict.passed); // Critical!
    }

    #[test]
    fn test_coverage_threshold() {
        let mut gate = Gate::new("test", GateEvaluator::CoverageThreshold(0.8), 3);

        let results = vec![ReviewResult {
            agent: "tester".to_string(),
            passed: true,
            comments: vec![],
            coverage: Some(0.85),
        }];
        let verdict = gate.evaluate(&results);
        assert!(verdict.passed);

        let results = vec![ReviewResult {
            agent: "tester".to_string(),
            passed: true,
            comments: vec![],
            coverage: Some(0.6),
        }];
        let verdict = gate.evaluate(&results);
        assert!(!verdict.passed);
    }

    #[test]
    fn test_gate_exceeded() {
        let mut gate = Gate::new("test", GateEvaluator::AllAgentsPass, 2);

        let results = vec![failing_result("a")];
        gate.evaluate(&results); // retries = 1
        gate.evaluate(&results); // retries = 2
        assert!(gate.is_exceeded());
    }

    #[test]
    fn test_gate_registry() {
        let mut registry = GateRegistry::new();
        registry.register(
            Phase::Reviewing,
            Gate::new("review-pass", GateEvaluator::AllAgentsPass, 3),
        );
        registry.register(
            Phase::Reviewing,
            Gate::new("no-critical", GateEvaluator::NoCritical, 3),
        );

        let gates = registry.gates_for(&Phase::Reviewing);
        assert_eq!(gates.len(), 2);
    }
}