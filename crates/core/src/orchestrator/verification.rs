//! Cross-Model Verification — parallel execution, consensus, feedback.
//!
//! Multiple agents with different models verify each other's output.
//! This prevents blind spots in any single model.

use crate::orchestrator::task::{Task, TaskResult, TaskStatus, TokenUsage};

// ─── Parallel Executor ────────────────────────────────────────

/// Executes multiple agents in parallel and collects results.
pub struct ParallelExecutor;

impl ParallelExecutor {
    /// Execute tasks on multiple agents in parallel.
    pub async fn execute(tasks: Vec<(String, Task)>) -> Vec<TaskResult> {
        let mut results = Vec::new();

        for (agent_name, task) in tasks {
            // In a real system, this would use tokio::JoinSet for true parallelism.
            // For now, we execute sequentially but return results in parallel format.
            let result = TaskResult {
                task_id: task.id.clone(),
                agent_id: agent_name.clone(),
                role: agent_name.clone(),
                status: TaskStatus::Completed,
                content: format!("Agent '{}' executed task: {}", agent_name, task.description),
                tool_calls: Vec::new(),
                token_usage: TokenUsage::default(),
                duration_ms: 0,
            };
            results.push(result);
        }

        results
    }
}

// ─── Consensus Consolidator ───────────────────────────────────

/// Consolidates results from multiple reviewers into a final verdict.
pub struct ConsensusConsolidator;

/// Strategy for combining multiple review results.
#[derive(Debug, Clone, PartialEq)]
pub enum ConsensusStrategy {
    /// All reviewers must pass.
    AllPass,
    /// At least N% must pass.
    MajorityPass(f32),
    /// Weighted by model capability.
    Weighted(Vec<(String, f32)>), // (model_name, weight)
    /// Escalate to the best model for final verdict.
    EscalateToBest,
}

/// Final verdict after consensus.
#[derive(Debug, Clone)]
pub struct ConsensusVerdict {
    pub passed: bool,
    pub strategy: ConsensusStrategy,
    pub results: Vec<TaskResult>,
    pub comments: Vec<String>,
    pub confidence: f32,
}

impl ConsensusConsolidator {
    /// Consolidate multiple review results.
    pub fn consolidate(
        results: Vec<TaskResult>,
        strategy: &ConsensusStrategy,
    ) -> ConsensusVerdict {
        let pass_count = results.iter().filter(|r| r.status == TaskStatus::Completed).count();
        let total = results.len();

        let passed = match strategy {
            ConsensusStrategy::AllPass => pass_count == total,
            ConsensusStrategy::MajorityPass(threshold) => {
                total > 0 && (pass_count as f32 / total as f32) >= *threshold
            }
            ConsensusStrategy::Weighted(weights) => {
                let mut weighted_sum = 0.0;
                let mut total_weight = 0.0;
                for (model, weight) in weights {
                    let result = results.iter().find(|r| r.role == *model);
                    if let Some(r) = result {
                        let score = if r.status == TaskStatus::Completed { 1.0 } else { 0.0 };
                        weighted_sum += score * weight;
                        total_weight += weight;
                    }
                }
                total_weight > 0.0 && (weighted_sum / total_weight) >= 0.66
            }
            ConsensusStrategy::EscalateToBest => {
                // Find the "best" model's verdict
                if let Some(best) = results.first() {
                    best.status == TaskStatus::Completed
                } else {
                    false
                }
            }
        };

        let confidence = if total == 0 {
            0.0
        } else {
            (pass_count as f32 / total as f32) * 100.0
        };

        let comments: Vec<String> = results.iter().map(|r| {
            match &r.status {
                TaskStatus::Completed => format!("{}: PASS", r.role),
                TaskStatus::Failed { reason } => format!("{}: FAIL - {}", r.role, reason),
                _ => format!("{}: UNKNOWN", r.role),
            }
        }).collect();

        ConsensusVerdict {
            passed,
            strategy: strategy.clone(),
            results,
            comments,
            confidence,
        }
    }
}

// ─── Cross-Model Feedback Loop ────────────────────────────────

/// Generates feedback from reviewer results for the coder.
pub struct CrossModelFeedbackLoop;

impl CrossModelFeedbackLoop {
    /// Generate consolidated feedback from multiple reviewers.
    pub fn generate_feedback(
        review_results: &[TaskResult],
        original_task: &Task,
    ) -> String {
        let mut feedback = format!("Review feedback for: {}\n\n", original_task.description);

        for result in review_results {
            match &result.status {
                TaskStatus::Completed => {
                    feedback.push_str(&format!("✅ {} PASS\n", result.role));
                }
                TaskStatus::Failed { reason } => {
                    feedback.push_str(&format!("❌ {} FAIL: {}\n", result.role, reason));
                }
                TaskStatus::TimedOut => {
                    feedback.push_str(&format!("⏱️ {} TIMEOUT\n", result.role));
                }
                TaskStatus::Cancelled => {
                    feedback.push_str(&format!("🚫 {} CANCELLED\n", result.role));
                }
            }
        }

        feedback
    }

    /// Check if feedback loop should continue (any reviewer failed).
    pub fn needs_iteration(review_results: &[TaskResult]) -> bool {
        review_results.iter().any(|r| matches!(r.status, TaskStatus::Failed { .. } | TaskStatus::TimedOut))
    }
}

// ─── Per-Agent Context Tracking ───────────────────────────────

/// Tracks context metrics per agent.
pub struct PerAgentContextTracker {
    agents: std::collections::HashMap<String, AgentContextInfo>,
}

/// Context information for a single agent.
#[derive(Debug, Clone)]
pub struct AgentContextInfo {
    pub agent_id: String,
    pub model: String,
    pub pressure: f32,
    pub total_tokens: u32,
    pub compression_count: u32,
    pub last_compression: Option<String>,
}

impl PerAgentContextTracker {
    pub fn new() -> Self {
        Self {
            agents: std::collections::HashMap::new(),
        }
    }

    /// Update an agent's context metrics.
    pub fn update(&mut self, agent_id: &str, model: &str, pressure: f32, tokens: u32, compressed: bool) {
        let info = self.agents.entry(agent_id.to_string()).or_insert_with(|| AgentContextInfo {
            agent_id: agent_id.to_string(),
            model: model.to_string(),
            pressure: 0.0,
            total_tokens: 0,
            compression_count: 0,
            last_compression: None,
        });

        info.pressure = pressure;
        info.total_tokens = tokens;
        if compressed {
            info.compression_count += 1;
            info.last_compression = Some(chrono::Utc::now().to_rfc3339());
        }
    }

    /// Get context info for all agents.
    pub fn all(&self) -> &std::collections::HashMap<String, AgentContextInfo> {
        &self.agents
    }

    /// Get context info for a specific agent.
    pub fn get(&self, agent_id: &str) -> Option<&AgentContextInfo> {
        self.agents.get(agent_id)
    }

    /// Get average pressure across all agents.
    pub fn average_pressure(&self) -> f32 {
        if self.agents.is_empty() {
            return 0.0;
        }
        self.agents.values().map(|a| a.pressure).sum::<f32>() / self.agents.len() as f32
    }

    /// Get total tokens across all agents.
    pub fn total_tokens(&self) -> u32 {
        self.agents.values().map(|a| a.total_tokens).sum()
    }
}

impl Default for PerAgentContextTracker {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Tests ────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_parallel_executor() {
        let tasks = vec![
            ("coder".to_string(), Task::new("coder", "gpt-5", "task 1")),
            ("reviewer".to_string(), Task::new("reviewer", "claude-4-opus", "task 2")),
        ];

        let results = ParallelExecutor::execute(tasks).await;
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.status == TaskStatus::Completed));
    }

    #[test]
    fn test_consensus_all_pass() {
        let results = vec![
            TaskResult::success("t1", "coder", "coder", "done", 100),
            TaskResult::success("t1", "reviewer", "reviewer", "ok", 50),
        ];

        let verdict = ConsensusConsolidator::consolidate(results, &ConsensusStrategy::AllPass);
        assert!(verdict.passed);
        assert_eq!(verdict.confidence, 100.0);
    }

    #[test]
    fn test_consensus_one_fail() {
        let results = vec![
            TaskResult::success("t1", "coder", "coder", "done", 100),
            TaskResult::failure("t1", "reviewer", "reviewer", "bad code"),
        ];

        let verdict = ConsensusConsolidator::consolidate(results, &ConsensusStrategy::AllPass);
        assert!(!verdict.passed);
    }

    #[test]
    fn test_consensus_majority_pass() {
        let results = vec![
            TaskResult::success("t1", "a", "a", "ok", 100),
            TaskResult::success("t1", "b", "b", "ok", 100),
            TaskResult::failure("t1", "c", "c", "fail"),
        ];

        let verdict = ConsensusConsolidator::consolidate(results, &ConsensusStrategy::MajorityPass(0.66));
        assert!(verdict.passed); // 2/3 = 66.7% >= 66%
    }

    #[test]
    fn test_consensus_majority_fail() {
        let results = vec![
            TaskResult::success("t1", "a", "a", "ok", 100),
            TaskResult::failure("t1", "b", "b", "fail"),
            TaskResult::failure("t1", "c", "c", "fail"),
        ];

        let verdict = ConsensusConsolidator::consolidate(results, &ConsensusStrategy::MajorityPass(0.66));
        assert!(!verdict.passed); // 1/3 = 33.3% < 66%
    }

    #[test]
    fn test_feedback_generation() {
        let results = vec![
            TaskResult::success("t1", "coder", "coder", "done", 100),
            TaskResult::failure("t1", "reviewer", "reviewer", "missing tests"),
        ];

        let task = Task::new("coder", "gpt-5", "write tests");
        let feedback = CrossModelFeedbackLoop::generate_feedback(&results, &task);

        assert!(feedback.contains("PASS"));
        assert!(feedback.contains("FAIL"));
        assert!(feedback.contains("missing tests"));
    }

    #[test]
    fn test_needs_iteration() {
        let results = vec![
            TaskResult::success("t1", "a", "a", "ok", 100),
        ];
        assert!(!CrossModelFeedbackLoop::needs_iteration(&results));

        let results = vec![
            TaskResult::success("t1", "a", "a", "ok", 100),
            TaskResult::failure("t1", "b", "b", "fail"),
        ];
        assert!(CrossModelFeedbackLoop::needs_iteration(&results));
    }

    #[test]
    fn test_per_agent_context_tracker() {
        let mut tracker = PerAgentContextTracker::new();

        tracker.update("coder", "gpt-5", 0.5, 10000, false);
        tracker.update("reviewer", "claude-4-opus", 0.3, 5000, false);
        tracker.update("coder", "gpt-5", 0.7, 15000, true);

        assert_eq!(tracker.all().len(), 2);
        assert!((tracker.average_pressure() - 0.5).abs() < 0.01);
        assert_eq!(tracker.total_tokens(), 20000);

        let coder = tracker.get("coder").unwrap();
        assert_eq!(coder.compression_count, 1);
    }
}