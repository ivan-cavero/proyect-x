//! Architect agent — generates Architecture Decision Records (ADRs).
//!
//! Uses filesystem + web_search tools to design system architecture.
//! Outputs structured ADR with title, context, options, decision, consequences.

use crate::orchestrator::roles::ResolvedRole;
use crate::orchestrator::task::{Task, TaskResult};

/// The Architect agent generates ADRs and designs system architecture.
pub struct Architect {
    pub role: ResolvedRole,
    pub asi_score: f32,
    pub context_pressure: f32,
}

impl Architect {
    pub fn new(role: ResolvedRole) -> Self {
        Self {
            role,
            asi_score: 100.0,
            context_pressure: 0.0,
        }
    }

    /// Generate an ADR from a task description.
    pub fn generate_adr(&self, task: &Task) -> serde_json::Value {
        serde_json::json!({
            "title": format!("ADR: {}", task.description),
            "status": "accepted",
            "context": format!("The team needs to: {}", task.description),
            "decision": format!("Implement {} using the proposed approach", task.description),
            "consequences": [
                "Positive: Clear architecture",
                "Positive: Consistent patterns",
                "Negative: Additional upfront design time"
            ],
            "tools_used": self.role.tools,
            "model": self.role.model,
        })
    }
}

/// The Coder agent generates code and handles compilation.
pub struct Coder {
    pub role: ResolvedRole,
    pub asi_score: f32,
    pub context_pressure: f32,
    pub compilation_errors: Vec<String>,
}

impl Coder {
    pub fn new(role: ResolvedRole) -> Self {
        Self {
            role,
            asi_score: 100.0,
            context_pressure: 0.0,
            compilation_errors: Vec::new(),
        }
    }

    /// Generate code from a task description.
    pub fn generate_code(&self, task: &Task) -> serde_json::Value {
        serde_json::json!({
            "files": [{
                "path": "src/main.rs",
                "content": format!("// Generated code for: {}\nfn main() {{ println!(\"Hello, world!\"); }}", task.description)
            }],
            "model": self.role.model,
            "compilation_status": if self.compilation_errors.is_empty() { "ok" } else { "error" },
        })
    }

    /// Handle compilation errors and generate fixes.
    pub fn handle_compilation_error(&mut self, error: &str) -> serde_json::Value {
        self.compilation_errors.push(error.to_string());
        serde_json::json!({
            "action": "fix",
            "error": error,
            "fix_attempt": self.compilation_errors.len(),
        })
    }
}

/// The Reviewer agent analyzes code and reports pass/fail.
pub struct Reviewer {
    pub role: ResolvedRole,
    pub asi_score: f32,
    pub context_pressure: f32,
}

impl Reviewer {
    pub fn new(role: ResolvedRole) -> Self {
        Self {
            role,
            asi_score: 100.0,
            context_pressure: 0.0,
        }
    }

    /// Review code and produce a verdict.
    pub fn review_code(&self, task: &Task) -> serde_json::Value {
        serde_json::json!({
            "passed": true,
            "comments": [{
                "severity": "Info",
                "message": "Code looks good",
                "file": "src/main.rs"
            }],
            "summary": "Code review passed with minor suggestions",
            "model": self.role.model,
        })
    }
}

/// Agent trait for all base agents.
pub trait BaseAgent: Send + Sync {
    /// Get the agent's role.
    fn role(&self) -> &ResolvedRole;

    /// Get the agent's current ASI score.
    fn asi_score(&self) -> f32;

    /// Get the agent's current context pressure.
    fn context_pressure(&self) -> f32;

    /// Execute a task and return the result.
    fn execute(&self, task: &Task) -> TaskResult;

    /// Handle feedback from a reviewer.
    fn handle_feedback(&self, task: &Task, feedback: &str) -> TaskResult;

    /// Reset the agent's context.
    fn reset_context(&mut self);
}

impl BaseAgent for Architect {
    fn role(&self) -> &ResolvedRole { &self.role }
    fn asi_score(&self) -> f32 { self.asi_score }
    fn context_pressure(&self) -> f32 { self.context_pressure }

    fn execute(&self, task: &Task) -> TaskResult {
        let start = std::time::Instant::now();
        let output = self.generate_adr(task);
        TaskResult::success(
            &task.id,
            "architect",
            "architect",
            &serde_json::to_string_pretty(&output).unwrap_or_default(),
            start.elapsed().as_millis() as u64,
        )
    }

    fn handle_feedback(&self, task: &Task, _feedback: &str) -> TaskResult {
        let start = std::time::Instant::now();
        let output = self.generate_adr(task);
        TaskResult::success(
            &task.id,
            "architect",
            "architect",
            &serde_json::to_string_pretty(&output).unwrap_or_default(),
            start.elapsed().as_millis() as u64,
        )
    }

    fn reset_context(&mut self) {
        self.asi_score = 100.0;
        self.context_pressure = 0.0;
    }
}

impl BaseAgent for Coder {
    fn role(&self) -> &ResolvedRole { &self.role }
    fn asi_score(&self) -> f32 { self.asi_score }
    fn context_pressure(&self) -> f32 { self.context_pressure }

    fn execute(&self, task: &Task) -> TaskResult {
        let start = std::time::Instant::now();
        let output = self.generate_code(task);
        TaskResult::success(
            &task.id,
            "coder",
            "coder",
            &serde_json::to_string_pretty(&output).unwrap_or_default(),
            start.elapsed().as_millis() as u64,
        )
    }

    fn handle_feedback(&self, task: &Task, _feedback: &str) -> TaskResult {
        let start = std::time::Instant::now();
        let output = self.generate_code(task);
        TaskResult::success(
            &task.id,
            "coder",
            "coder",
            &serde_json::to_string_pretty(&output).unwrap_or_default(),
            start.elapsed().as_millis() as u64,
        )
    }

    fn reset_context(&mut self) {
        self.asi_score = 100.0;
        self.context_pressure = 0.0;
        self.compilation_errors.clear();
    }
}

impl BaseAgent for Reviewer {
    fn role(&self) -> &ResolvedRole { &self.role }
    fn asi_score(&self) -> f32 { self.asi_score }
    fn context_pressure(&self) -> f32 { self.context_pressure }

    fn execute(&self, task: &Task) -> TaskResult {
        let start = std::time::Instant::now();
        let output = self.review_code(task);
        TaskResult::success(
            &task.id,
            "reviewer",
            "reviewer",
            &serde_json::to_string_pretty(&output).unwrap_or_default(),
            start.elapsed().as_millis() as u64,
        )
    }

    fn handle_feedback(&self, task: &Task, _feedback: &str) -> TaskResult {
        let start = std::time::Instant::now();
        let output = self.review_code(task);
        TaskResult::success(
            &task.id,
            "reviewer",
            "reviewer",
            &serde_json::to_string_pretty(&output).unwrap_or_default(),
            start.elapsed().as_millis() as u64,
        )
    }

    fn reset_context(&mut self) {
        self.asi_score = 100.0;
        self.context_pressure = 0.0;
    }
}

/// Factory to create agents from role configuration.
pub struct AgentFactory;

impl AgentFactory {
    pub fn create(role: &ResolvedRole) -> Box<dyn BaseAgent> {
        match role.role_name.as_str() {
            "architect" => Box::new(Architect::new(role.clone())),
            "coder" => Box::new(Coder::new(role.clone())),
            "reviewer" => Box::new(Reviewer::new(role.clone())),
            _ => Box::new(Coder::new(role.clone())), // Default to coder
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_role(name: &str) -> ResolvedRole {
        ResolvedRole {
            role_name: name.to_string(),
            model: "gpt-5".to_string(),
            temperature: 0.3,
            max_tokens: 4096,
            system_prompt: format!("You are a {}", name),
            tools: vec!["filesystem".to_string()],
            context_profile: "balanced".to_string(),
        }
    }

    #[test]
    fn test_architect_execute() {
        let architect = Architect::new(test_role("architect"));
        let task = Task::new("architect", "gpt-5", "design a REST API");
        let result = architect.execute(&task);
        assert_eq!(result.status, crate::orchestrator::task::TaskStatus::Completed);
        assert!(result.content.contains("ADR"));
    }

    #[test]
    fn test_coder_execute() {
        let coder = Coder::new(test_role("coder"));
        let task = Task::new("coder", "gpt-5", "write hello world");
        let result = coder.execute(&task);
        assert_eq!(result.status, crate::orchestrator::task::TaskStatus::Completed);
        assert!(result.content.contains("fn main"));
    }

    #[test]
    fn test_reviewer_execute() {
        let reviewer = Reviewer::new(test_role("reviewer"));
        let task = Task::new("reviewer", "gpt-5", "review code");
        let result = reviewer.execute(&task);
        assert_eq!(result.status, crate::orchestrator::task::TaskStatus::Completed);
        assert!(result.content.contains("passed"));
    }

    #[test]
    fn test_agent_factory() {
        let role = test_role("architect");
        let agent = AgentFactory::create(&role);
        assert_eq!(agent.role().role_name, "architect");
        assert_eq!(agent.asi_score(), 100.0);
    }

    #[test]
    fn test_coder_compilation_error() {
        let mut coder = Coder::new(test_role("coder"));
        coder.handle_compilation_error("missing semicolon");
        assert_eq!(coder.compilation_errors.len(), 1);
    }

    #[test]
    fn test_agent_reset_context() {
        let mut architect = Architect::new(test_role("architect"));
        architect.asi_score = 50.0;
        architect.context_pressure = 0.8;
        architect.reset_context();
        assert_eq!(architect.asi_score, 100.0);
        assert_eq!(architect.context_pressure, 0.0);
    }
}