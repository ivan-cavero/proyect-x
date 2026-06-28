//! Task dispatch and management.

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// A task to be executed by an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub description: String,
    pub context: String,
    pub phase: String,
    pub max_iterations: u32,
    pub timeout: Duration,
    pub role: String,
    pub model: String,
}

impl Task {
    pub fn new(role: &str, model: &str, description: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            description: description.to_string(),
            context: String::new(),
            phase: String::new(),
            max_iterations: 5,
            timeout: Duration::from_secs(300),
            role: role.to_string(),
            model: model.to_string(),
        }
    }
}

/// Result from an agent task execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub task_id: String,
    pub agent_id: String,
    pub role: String,
    pub status: TaskStatus,
    pub content: String,
    pub tool_calls: Vec<String>,
    pub token_usage: TokenUsage,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    Completed,
    Failed { reason: String },
    TimedOut,
    Cancelled,
}

/// Token usage for a task.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TokenUsage {
    pub input: u32,
    pub output: u32,
    pub total: u32,
}

impl TaskResult {
    pub fn success(task_id: &str, agent_id: &str, role: &str, content: &str, duration_ms: u64) -> Self {
        Self {
            task_id: task_id.to_string(),
            agent_id: agent_id.to_string(),
            role: role.to_string(),
            status: TaskStatus::Completed,
            content: content.to_string(),
            tool_calls: Vec::new(),
            token_usage: TokenUsage::default(),
            duration_ms,
        }
    }

    pub fn failure(task_id: &str, agent_id: &str, role: &str, reason: &str) -> Self {
        Self {
            task_id: task_id.to_string(),
            agent_id: agent_id.to_string(),
            role: role.to_string(),
            status: TaskStatus::Failed { reason: reason.to_string() },
            content: String::new(),
            tool_calls: Vec::new(),
            token_usage: TokenUsage::default(),
            duration_ms: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_creation() {
        let task = Task::new("coder", "gpt-5", "write hello world");
        assert_eq!(task.role, "coder");
        assert_eq!(task.model, "gpt-5");
        assert_eq!(task.description, "write hello world");
        assert!(!task.id.is_empty());
    }

    #[test]
    fn test_task_result_success() {
        let result = TaskResult::success("t1", "a1", "coder", "done", 100);
        assert_eq!(result.status, TaskStatus::Completed);
        assert_eq!(result.content, "done");
    }

    #[test]
    fn test_task_result_failure() {
        let result = TaskResult::failure("t1", "a1", "coder", "timeout");
        match &result.status {
            TaskStatus::Failed { reason } => assert_eq!(reason, "timeout"),
            _ => panic!("Expected Failed"),
        }
    }
}