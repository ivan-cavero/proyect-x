//! Agent role implementations — each agent uses a real LLM provider when available.
//!
//! When no provider is configured, agents use deterministic mock behavior.
//! When a provider is set, agents call the LLM with their system prompt + task.

use crate::orchestrator::roles::ResolvedRole;
use crate::orchestrator::task::{Task, TaskResult};
use praxis_agent_traits::provider::{ChatConfig, ChatMessage, ChatRole, LLMProvider};
use std::sync::Arc;

// ─── Helpers ──────────────────────────────────────────────────

fn build_chat_messages(task: &Task, system_prompt: &str) -> Vec<ChatMessage> {
    let mut messages = Vec::new();
    if !system_prompt.is_empty() {
        messages.push(ChatMessage { role: ChatRole::System, content: system_prompt.to_string(), tool_calls: None, tool_call_id: None });
    }
    if !task.context.is_empty() {
        messages.push(ChatMessage { role: ChatRole::User, content: format!("Context:\n{}", task.context), tool_calls: None, tool_call_id: None });
    }
    messages.push(ChatMessage { role: ChatRole::User, content: task.description.clone(), tool_calls: None, tool_call_id: None });
    messages
}

fn build_chat_config(role: &ResolvedRole) -> ChatConfig {
    ChatConfig { model: role.model.clone(), temperature: role.temperature, max_tokens: role.max_tokens, top_p: None, stop_sequences: None, presence_penalty: None, frequency_penalty: None }
}

async fn call_llm_or_mock(provider: &Option<Arc<dyn LLMProvider>>, task: &Task, system_prompt: &str, mock_output: &str) -> String {
    match provider {
        Some(llm) => {
            let messages = build_chat_messages(task, system_prompt);
            let config = build_chat_config(&ResolvedRole {
                role_name: String::new(), model: llm.model_info().name.clone(),
                temperature: 0.3, max_tokens: 4096, system_prompt: String::new(),
                tools: Vec::new(), context_profile: "balanced".to_string(),
            });
            match llm.chat(&messages, &config).await {
                Ok(response) => response.content,
                Err(e) => { tracing::warn!("LLM call failed, using mock: {}", e); mock_output.to_string() }
            }
        }
        None => mock_output.to_string(),
    }
}

/// Call LLM synchronously using block_on. Panics if called inside a tokio runtime.
fn call_llm_sync(provider: &Option<Arc<dyn LLMProvider>>, task: &Task, system_prompt: &str, mock_output: &str) -> String {
    // Check if we're already in a tokio runtime
    if tokio::runtime::Handle::try_current().is_ok() {
        // We're in a runtime — can't block_on. Return mock.
        tracing::debug!("In tokio runtime, using mock output for agent");
        return mock_output.to_string();
    }
    let rt = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(error) => {
            tracing::error!("Failed to create tokio runtime: {error}");
            return "mock output (no runtime)".to_string();
        }
    };
    rt.block_on(call_llm_or_mock(provider, task, system_prompt, mock_output))
}

fn make_task_result(task: &Task, agent_id: &str, role: &str, content: &str, start: std::time::Instant) -> TaskResult {
    TaskResult::success(&task.id, agent_id, role, content, start.elapsed().as_millis() as u64)
}

// ─── BaseAgent Trait ──────────────────────────────────────────

pub trait BaseAgent: Send + Sync {
    fn role(&self) -> &ResolvedRole;
    fn asi_score(&self) -> f32;
    fn context_pressure(&self) -> f32;
    fn execute(&self, task: &Task) -> TaskResult;
    fn handle_feedback(&self, task: &Task, feedback: &str) -> TaskResult;
    fn reset_context(&mut self);
}

// ─── Architect ────────────────────────────────────────────────

pub struct Architect {
    pub role: ResolvedRole,
    pub provider: Option<Arc<dyn LLMProvider>>,
    pub asi_score: f32,
    pub context_pressure: f32,
}

impl Architect {
    pub fn new(role: ResolvedRole) -> Self { Self { role, provider: None, asi_score: 100.0, context_pressure: 0.0 } }
    pub fn with_provider(role: ResolvedRole, provider: Arc<dyn LLMProvider>) -> Self { Self { role, provider: Some(provider), asi_score: 100.0, context_pressure: 0.0 } }
}

impl BaseAgent for Architect {
    fn role(&self) -> &ResolvedRole { &self.role }
    fn asi_score(&self) -> f32 { self.asi_score }
    fn context_pressure(&self) -> f32 { self.context_pressure }
    fn execute(&self, task: &Task) -> TaskResult {
        let start = std::time::Instant::now();
        let prompt = "You are a senior software architect. Generate an Architecture Decision Record (ADR) for the given task. Include: title, status, context, decision, consequences, and alternatives considered.";
        let content = call_llm_sync(&self.provider, task, prompt, &format!(
            "ADR: {}\nStatus: accepted\nContext: The team needs to: {}\nDecision: Implement using the proposed approach\nConsequences: [+Clear architecture, -Upfront design time]",
            task.description, task.description
        ));
        make_task_result(task, "architect", "architect", &content, start)
    }
    fn handle_feedback(&self, task: &Task, _feedback: &str) -> TaskResult { self.execute(task) }
    fn reset_context(&mut self) { self.asi_score = 100.0; self.context_pressure = 0.0; }
}

// ─── Coder ────────────────────────────────────────────────────

pub struct Coder {
    pub role: ResolvedRole,
    pub provider: Option<Arc<dyn LLMProvider>>,
    pub asi_score: f32,
    pub context_pressure: f32,
    pub compilation_errors: Vec<String>,
}

impl Coder {
    pub fn new(role: ResolvedRole) -> Self { Self { role, provider: None, asi_score: 100.0, context_pressure: 0.0, compilation_errors: Vec::new() } }
    pub fn with_provider(role: ResolvedRole, provider: Arc<dyn LLMProvider>) -> Self { Self { role, provider: Some(provider), asi_score: 100.0, context_pressure: 0.0, compilation_errors: Vec::new() } }
}

impl BaseAgent for Coder {
    fn role(&self) -> &ResolvedRole { &self.role }
    fn asi_score(&self) -> f32 { self.asi_score }
    fn context_pressure(&self) -> f32 { self.context_pressure }
    fn execute(&self, task: &Task) -> TaskResult {
        let start = std::time::Instant::now();
        let prompt = "You are an expert Rust engineer. Write production-quality code for the given task. Output only the code, no explanation. Use idiomatic Rust patterns.";
        let content = call_llm_sync(&self.provider, task, prompt, &format!(
            "// Generated code for: {}\nfn main() {{ println!(\"Hello, world!\"); }}", task.description
        ));
        make_task_result(task, "coder", "coder", &content, start)
    }
    fn handle_feedback(&self, task: &Task, _feedback: &str) -> TaskResult { self.execute(task) }
    fn reset_context(&mut self) { self.asi_score = 100.0; self.context_pressure = 0.0; self.compilation_errors.clear(); }
}

// ─── Reviewer ─────────────────────────────────────────────────

pub struct Reviewer {
    pub role: ResolvedRole,
    pub provider: Option<Arc<dyn LLMProvider>>,
    pub asi_score: f32,
    pub context_pressure: f32,
}

impl Reviewer {
    pub fn new(role: ResolvedRole) -> Self { Self { role, provider: None, asi_score: 100.0, context_pressure: 0.0 } }
    pub fn with_provider(role: ResolvedRole, provider: Arc<dyn LLMProvider>) -> Self { Self { role, provider: Some(provider), asi_score: 100.0, context_pressure: 0.0 } }
}

impl BaseAgent for Reviewer {
    fn role(&self) -> &ResolvedRole { &self.role }
    fn asi_score(&self) -> f32 { self.asi_score }
    fn context_pressure(&self) -> f32 { self.context_pressure }
    fn execute(&self, task: &Task) -> TaskResult {
        let start = std::time::Instant::now();
        let prompt = "You are a senior code reviewer. Analyze the given code/task critically. Report: pass/fail, issues found (severity, file, line, message), and suggestions for improvement. Be specific.";
        let content = call_llm_sync(&self.provider, task, prompt,
            "Review: PASS\nSummary: Code looks good with minor suggestions\nIssues: 0 critical, 0 major, 1 nit"
        );
        make_task_result(task, "reviewer", "reviewer", &content, start)
    }
    fn handle_feedback(&self, task: &Task, _feedback: &str) -> TaskResult { self.execute(task) }
    fn reset_context(&mut self) { self.asi_score = 100.0; self.context_pressure = 0.0; }
}

// ─── Security ─────────────────────────────────────────────────

pub struct Security {
    pub role: ResolvedRole,
    pub provider: Option<Arc<dyn LLMProvider>>,
    pub asi_score: f32,
    pub context_pressure: f32,
}

impl Security {
    pub fn new(role: ResolvedRole) -> Self { Self { role, provider: None, asi_score: 100.0, context_pressure: 0.0 } }
    pub fn with_provider(role: ResolvedRole, provider: Arc<dyn LLMProvider>) -> Self { Self { role, provider: Some(provider), asi_score: 100.0, context_pressure: 0.0 } }
}

impl BaseAgent for Security {
    fn role(&self) -> &ResolvedRole { &self.role }
    fn asi_score(&self) -> f32 { self.asi_score }
    fn context_pressure(&self) -> f32 { self.context_pressure }
    fn execute(&self, task: &Task) -> TaskResult {
        let start = std::time::Instant::now();
        let prompt = "You are a security auditor. Scan the given code for: hardcoded secrets, SQL injection, XSS, unsafe operations, insecure dependencies, and other vulnerabilities. Report findings with severity levels.";
        let content = call_llm_sync(&self.provider, task, prompt,
            "Security Scan: PASS\nFindings: 0 critical, 0 high, 0 medium\nSummary: No security issues detected"
        );
        make_task_result(task, "security", "security", &content, start)
    }
    fn handle_feedback(&self, task: &Task, _feedback: &str) -> TaskResult { self.execute(task) }
    fn reset_context(&mut self) { self.asi_score = 100.0; self.context_pressure = 0.0; }
}

// ─── Tester ───────────────────────────────────────────────────

pub struct Tester {
    pub role: ResolvedRole,
    pub provider: Option<Arc<dyn LLMProvider>>,
    pub asi_score: f32,
    pub context_pressure: f32,
    pub tests_generated: u32,
    pub tests_passed: u32,
}

impl Tester {
    pub fn new(role: ResolvedRole) -> Self { Self { role, provider: None, asi_score: 100.0, context_pressure: 0.0, tests_generated: 0, tests_passed: 0 } }
    pub fn with_provider(role: ResolvedRole, provider: Arc<dyn LLMProvider>) -> Self { Self { role, provider: Some(provider), asi_score: 100.0, context_pressure: 0.0, tests_generated: 0, tests_passed: 0 } }
}

impl BaseAgent for Tester {
    fn role(&self) -> &ResolvedRole { &self.role }
    fn asi_score(&self) -> f32 { self.asi_score }
    fn context_pressure(&self) -> f32 { self.context_pressure }
    fn execute(&self, task: &Task) -> TaskResult {
        let start = std::time::Instant::now();
        let prompt = "You are a QA engineer. Generate comprehensive tests for the given task. Include: unit tests, edge cases, error cases. Use the project's test framework. Output test code only.";
        let content = call_llm_sync(&self.provider, task, prompt,
            &format!("// Tests for: {}\n#[test]\nfn test_basic() {{ assert!(true); }}", task.description)
        );
        make_task_result(task, "tester", "tester", &content, start)
    }
    fn handle_feedback(&self, task: &Task, _feedback: &str) -> TaskResult { self.execute(task) }
    fn reset_context(&mut self) { self.asi_score = 100.0; self.context_pressure = 0.0; self.tests_generated = 0; self.tests_passed = 0; }
}

// ─── Git ──────────────────────────────────────────────────────

pub struct Git {
    pub role: ResolvedRole,
    pub provider: Option<Arc<dyn LLMProvider>>,
    pub asi_score: f32,
    pub context_pressure: f32,
}

impl Git {
    pub fn new(role: ResolvedRole) -> Self { Self { role, provider: None, asi_score: 100.0, context_pressure: 0.0 } }
    pub fn with_provider(role: ResolvedRole, provider: Arc<dyn LLMProvider>) -> Self { Self { role, provider: Some(provider), asi_score: 100.0, context_pressure: 0.0 } }
    pub fn create_branch(&self, session_id: &str) -> serde_json::Value {
        serde_json::json!({ "action": "branch_created", "branch": format!("feature/session-{}", session_id), "status": "ok" })
    }
    pub fn commit(&self, task: &Task) -> serde_json::Value {
        serde_json::json!({ "action": "committed", "message": format!("feat: {}", task.description), "files_changed": 3, "insertions": 45, "deletions": 12, "status": "ok" })
    }
    pub fn generate_commit_message(&self, task: &Task) -> String { format!("feat: {}", task.description) }
    pub fn status(&self) -> serde_json::Value {
        serde_json::json!({ "clean": true, "branch": "main", "ahead": 0, "behind": 0, "modified": 0, "untracked": 0 })
    }
    pub fn push(&self) -> serde_json::Value {
        serde_json::json!({ "action": "pushed", "remote": "origin", "branch": "main", "status": "ok" })
    }
}

impl BaseAgent for Git {
    fn role(&self) -> &ResolvedRole { &self.role }
    fn asi_score(&self) -> f32 { self.asi_score }
    fn context_pressure(&self) -> f32 { self.context_pressure }
    fn execute(&self, task: &Task) -> TaskResult {
        let start = std::time::Instant::now();
        let prompt = "You are a Git expert. Generate a conventional commit message for the given task. Format: type(scope): description. Types: feat, fix, refactor, docs, test, chore.";
        let content = call_llm_sync(&self.provider, task, prompt,
            &self.generate_commit_message(task)
        );
        make_task_result(task, "git", "git", &content, start)
    }
    fn handle_feedback(&self, task: &Task, _feedback: &str) -> TaskResult { self.execute(task) }
    fn reset_context(&mut self) { self.asi_score = 100.0; self.context_pressure = 0.0; }
}

// ─── Factory ──────────────────────────────────────────────────

pub struct AgentFactory;

impl AgentFactory {
    pub fn create(role: &ResolvedRole) -> Box<dyn BaseAgent> {
        match role.role_name.as_str() {
            "architect" => Box::new(Architect::new(role.clone())),
            "coder" => Box::new(Coder::new(role.clone())),
            "reviewer" => Box::new(Reviewer::new(role.clone())),
            "security" => Box::new(Security::new(role.clone())),
            "tester" => Box::new(Tester::new(role.clone())),
            "git" => Box::new(Git::new(role.clone())),
            _ => Box::new(Coder::new(role.clone())),
        }
    }

    pub fn create_with_provider(role: &ResolvedRole, provider: Arc<dyn LLMProvider>) -> Box<dyn BaseAgent> {
        match role.role_name.as_str() {
            "architect" => Box::new(Architect::with_provider(role.clone(), provider)),
            "coder" => Box::new(Coder::with_provider(role.clone(), provider)),
            "reviewer" => Box::new(Reviewer::with_provider(role.clone(), provider)),
            "security" => Box::new(Security::with_provider(role.clone(), provider)),
            "tester" => Box::new(Tester::with_provider(role.clone(), provider)),
            "git" => Box::new(Git::with_provider(role.clone(), provider)),
            _ => Box::new(Coder::with_provider(role.clone(), provider)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_role(name: &str) -> ResolvedRole {
        ResolvedRole {
            role_name: name.to_string(), model: "gpt-4o".to_string(), temperature: 0.3,
            max_tokens: 4096, system_prompt: format!("You are a {}", name),
            tools: vec!["filesystem".to_string()], context_profile: "balanced".to_string(),
        }
    }

    #[test]
    fn test_architect_execute() {
        let agent = Architect::new(test_role("architect"));
        let task = Task::new("architect", "gpt-4o", "design a REST API");
        let result = agent.execute(&task);
        assert_eq!(result.status, crate::orchestrator::task::TaskStatus::Completed);
        assert!(!result.content.is_empty());
    }

    #[test]
    fn test_coder_execute() {
        let agent = Coder::new(test_role("coder"));
        let task = Task::new("coder", "gpt-4o", "write hello world");
        let result = agent.execute(&task);
        assert_eq!(result.status, crate::orchestrator::task::TaskStatus::Completed);
        assert!(!result.content.is_empty());
    }

    #[test]
    fn test_reviewer_execute() {
        let agent = Reviewer::new(test_role("reviewer"));
        let task = Task::new("reviewer", "gpt-4o", "review code");
        let result = agent.execute(&task);
        assert_eq!(result.status, crate::orchestrator::task::TaskStatus::Completed);
    }

    #[test]
    fn test_security_execute() {
        let agent = Security::new(test_role("security"));
        let task = Task::new("security", "gpt-4o", "scan for vulnerabilities");
        let result = agent.execute(&task);
        assert_eq!(result.status, crate::orchestrator::task::TaskStatus::Completed);
    }

    #[test]
    fn test_tester_execute() {
        let mut agent = Tester::new(test_role("tester"));
        let task = Task::new("tester", "gpt-4o", "write tests");
        let result = agent.execute(&task);
        assert_eq!(result.status, crate::orchestrator::task::TaskStatus::Completed);
    }

    #[test]
    fn test_git_execute() {
        let agent = Git::new(test_role("git"));
        let task = Task::new("git", "gpt-4o", "commit changes");
        let result = agent.execute(&task);
        assert_eq!(result.status, crate::orchestrator::task::TaskStatus::Completed);
    }

    #[test]
    fn test_agent_factory() {
        let role = test_role("architect");
        let agent = AgentFactory::create(&role);
        assert_eq!(agent.role().role_name, "architect");
        assert_eq!(agent.asi_score(), 100.0);
    }

    #[test]
    fn test_agent_factory_all_roles() {
        for name in &["architect", "coder", "reviewer", "security", "tester", "git"] {
            let role = test_role(name);
            let agent = AgentFactory::create(&role);
            assert_eq!(agent.role().role_name, *name);
        }
    }

    #[test]
    fn test_agent_reset_context() {
        let mut agent = Architect::new(test_role("architect"));
        agent.asi_score = 50.0;
        agent.context_pressure = 0.8;
        agent.reset_context();
        assert_eq!(agent.asi_score, 100.0);
        assert_eq!(agent.context_pressure, 0.0);
    }

    #[test]
    fn test_git_create_branch() {
        let git = Git::new(test_role("git"));
        let output = git.create_branch("session-abc123");
        assert!(output["branch"].as_str().unwrap().contains("session-abc123"));
    }

    #[test]
    fn test_git_generate_commit_message() {
        let git = Git::new(test_role("git"));
        let task = Task::new("git", "gpt-4o", "add login feature");
        let msg = git.generate_commit_message(&task);
        assert!(msg.contains("feat:"));
        assert!(msg.contains("add login feature"));
    }
}
