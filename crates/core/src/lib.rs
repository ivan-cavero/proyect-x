//! # Project-X Core Runtime
//!
//! The heart of the system: actor model, state machine, orchestrator,
//! loop controller, drift detection, and context management.

pub mod actor;
pub mod api;
pub mod bus;
pub mod drift;
pub mod r#loop;
pub mod machine;
pub mod workflow;
pub mod orchestrator;
pub mod rbac;

#[cfg(test)]
mod integration_tests;

// Re-exports for convenience
pub use actor::*;
pub use bus::EventBus;
pub use drift::*;
pub use r#loop::*;
pub use machine::*;
pub use workflow::*;
pub use orchestrator::{RoleConfig, RoleOverride, GoalConfig, ResolvedRole};
pub use orchestrator::roles::ResolvedRole as AgentRoleResolved;
pub use orchestrator::{Task, TaskResult, TaskStatus};

use project_x_mcp_host::McpHost;

use thiserror::Error;

// ─── Error Types ──────────────────────────────────────────────

#[derive(Error, Debug)]
pub enum CoreError {
    #[error("Actor error: {0}")]
    Actor(String),

    #[error("Runtime error: {0}")]
    Runtime(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("State machine error: {0}")]
    StateMachine(String),

    #[error("Context error: {0}")]
    Context(String),

    #[error("Event bus error: {0}")]
    EventBus(String),

    #[error("IO error: {0}")]
    Io(String),

    #[error("Not implemented: {0}")]
    NotImplemented(String),
}

pub type Result<T> = std::result::Result<T, CoreError>;

// ─── Runtime ──────────────────────────────────────────────────

/// The central runtime that manages the entire system.
pub struct CoreRuntime {
    pub bus: EventBus,
    pub supervisor: ractor::ActorRef<actor::SupervisorMessage>,
    pub loop_controller: crate::r#loop::LoopController,
    pub drift_guard: crate::drift::DriftGuard,
    pub mcp_host: McpHost,
}

impl CoreRuntime {
    /// Create and start a new CoreRuntime.
    pub async fn new() -> Result<Self> {
        let bus = EventBus::new();
        let supervisor = actor::Supervisor::spawn().await?;
        let loop_controller = crate::r#loop::LoopController::new();
        let drift_guard = crate::drift::DriftGuard::new();
        let mcp_host = McpHost::new("project-x");

        Ok(Self { bus, supervisor, loop_controller, drift_guard, mcp_host })
    }

    /// Connect MCP servers defined in the forge.toml config.
    pub async fn connect_mcp_servers(&mut self, config: &ForgeConfig) {
        for server_config in &config.mcp_servers {
            tracing::info!("Connecting to MCP server: {} ({} {:?})",
                server_config.name, server_config.command, server_config.args);
            match self.mcp_host.connect_server(
                &server_config.name,
                &server_config.command,
                &server_config.args,
            ).await {
                Ok(()) => {
                    let tools = self.mcp_host.tools_for(&server_config.name);
                    tracing::info!("MCP server '{}' connected with {} tools",
                        server_config.name, tools.len());
                }
                Err(e) => {
                    tracing::warn!("Failed to connect MCP server '{}': {}",
                        server_config.name, e);
                }
            }
        }
    }

    /// Run a goal through the full agent pipeline.
    ///
    /// Flow: Planning → Designing → Implementing → Reviewing → Testing → Finalizing
    /// Each phase spawns the appropriate agents and collects results.
    pub async fn run_goal(&mut self, goal: &str, config_path: Option<&std::path::Path>) -> Result<GoalResult> {
        tracing::info!("Starting goal: {}", goal);

        // Load config from forge.toml
        let config = match config_path {
            Some(path) => load_forge_config(path)?,
            None => {
                tracing::warn!("No config path provided, using defaults");
                ForgeConfig::default()
            }
        };

        // Start the loop
        self.loop_controller.start();
        self.bus.publish(
            project_x_shared::protocol::MessageKind::SessionHeartbeat,
            "core",
        );

        // Advance state machine to Planning (first phase)
        self.loop_controller.advance(machine::phase::Phase::Planning).ok();

        let mut results = Vec::new();
        let mut current_phase = machine::phase::Phase::Planning;

        // Navigate through phases
        loop {
            if current_phase.is_terminal() {
                break;
            }

            tracing::info!("Phase: {}", current_phase);

            // Get agents for this phase
            let phase_agents = get_agents_for_phase(&current_phase, &config);

            // Execute agents in this phase
            for role_config in &phase_agents {
                let task = orchestrator::Task::new(
                    &role_config.name,
                    &role_config.model,
                    goal,
                );

                let agent = crate::actor::roles::AgentFactory::create(
                    &orchestrator::roles::ResolvedRole::resolve(role_config, None),
                );

                let result = agent.execute(&task);
                tracing::info!(
                    "Agent {} completed: status={:?}, duration={}ms",
                    result.agent_id,
                    result.status,
                    result.duration_ms
                );

                results.push(result);
            }

            // Advance to next phase
            let next_phase = get_next_phase(&current_phase);
            match self.loop_controller.advance(next_phase) {
                Ok(_transition) => {
                    self.bus.publish(
                        project_x_shared::protocol::MessageKind::PhaseChanged(
                            project_x_shared::protocol::PhaseTransition {
                                from: project_x_shared::types::Phase::Planning,
                                to: project_x_shared::types::Phase::Implementing,
                                condition: "automatic".to_string(),
                                timestamp: chrono::Utc::now().to_rfc3339(),
                            },
                        ),
                        "core",
                    );
                }
                Err(e) => {
                    tracing::error!("Failed to advance phase: {}", e);
                    break;
                }
            }

            current_phase = next_phase;
            self.loop_controller.increment_iteration();
        }

        self.loop_controller.stop();

        let total_duration: u64 = results.iter().map(|r| r.duration_ms).sum();
        let passed = results.iter().all(|r| r.status == orchestrator::task::TaskStatus::Completed);

        tracing::info!(
            "Goal '{}' completed: phases={}, agents={}, passed={}, duration={}ms",
            goal,
            self.loop_controller.state_machine.history().len(),
            results.len(),
            passed,
            total_duration,
        );

        Ok(GoalResult {
            goal: goal.to_string(),
            passed,
            agent_results: results,
            total_duration_ms: total_duration,
        })
    }

    /// Spawn a new EchoAgent via the supervisor (for testing).
    pub async fn spawn_echo_agent(&self, name: &str) -> Result<actor::AgentHandle> {
        actor::spawn_echo(&self.supervisor, name).await
    }

    /// Send an echo message to a named child agent.
    pub async fn echo_to(&self, child_name: &str, content: &str) -> Result<String> {
        actor::supervisor_echo_to(&self.supervisor, child_name, content).await
    }

    /// List all running child agents.
    pub async fn list_agents(&self) -> Result<Vec<actor::AgentHandle>> {
        actor::list_children(&self.supervisor).await
    }

    /// Shutdown all agents and stop the runtime.
    pub async fn shutdown(&self) -> Result<()> {
        actor::shutdown_all(&self.supervisor).await
    }
}

// ─── Goal Result ──────────────────────────────────────────────

/// Result of running a goal through the pipeline.
#[derive(Debug, Clone)]
pub struct GoalResult {
    pub goal: String,
    pub passed: bool,
    pub agent_results: Vec<orchestrator::TaskResult>,
    pub total_duration_ms: u64,
}

// ─── Config ───────────────────────────────────────────────────

/// Parsed forge.toml configuration.
pub struct ForgeConfig {
    pub roles: std::collections::HashMap<String, orchestrator::RoleConfig>,
    pub goals: Vec<orchestrator::GoalConfig>,
    pub mcp_servers: Vec<McpServerConfig>,
}

/// MCP server configuration from forge.toml.
pub struct McpServerConfig {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
}

impl Default for ForgeConfig {
    fn default() -> Self {
        let mut roles = std::collections::HashMap::new();
        roles.insert("architect".to_string(), orchestrator::RoleConfig {
            name: "architect".to_string(),
            model: "gpt-4o".to_string(),
            temperature: 0.3,
            max_tokens: 4096,
            system_prompt: Some("You are a senior software architect.".to_string()),
            tools: vec!["filesystem".to_string()],
            ..Default::default()
        });
        roles.insert("coder".to_string(), orchestrator::RoleConfig {
            name: "coder".to_string(),
            model: "gpt-4o".to_string(),
            temperature: 0.3,
            max_tokens: 4096,
            system_prompt: Some("You are an expert Rust engineer.".to_string()),
            tools: vec!["filesystem".to_string(), "execute_command".to_string()],
            ..Default::default()
        });
        roles.insert("reviewer".to_string(), orchestrator::RoleConfig {
            name: "reviewer".to_string(),
            model: "gpt-4o".to_string(),
            temperature: 0.2,
            max_tokens: 4096,
            system_prompt: Some("You are a senior code reviewer.".to_string()),
            tools: vec!["filesystem".to_string()],
            ..Default::default()
        });
        roles.insert("security".to_string(), orchestrator::RoleConfig {
            name: "security".to_string(),
            model: "gpt-4o-mini".to_string(),
            temperature: 0.1,
            max_tokens: 4096,
            system_prompt: Some("You are a security auditor.".to_string()),
            tools: vec!["filesystem".to_string()],
            ..Default::default()
        });
        roles.insert("tester".to_string(), orchestrator::RoleConfig {
            name: "tester".to_string(),
            model: "gpt-4o".to_string(),
            temperature: 0.2,
            max_tokens: 4096,
            system_prompt: Some("You are a QA engineer.".to_string()),
            tools: vec!["filesystem".to_string(), "execute_command".to_string()],
            ..Default::default()
        });

        Self {
            roles,
            goals: vec![orchestrator::GoalConfig {
                name: "full-feature".to_string(),
                agents: vec![
                    "architect".to_string(),
                    "coder".to_string(),
                    "reviewer".to_string(),
                    "security".to_string(),
                    "tester".to_string(),
                ],
                ..Default::default()
            }],
            mcp_servers: Vec::new(),
        }
    }
}

/// Load forge.toml configuration from a file.
pub fn load_forge_config(path: &std::path::Path) -> Result<ForgeConfig> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| CoreError::Config(format!("Failed to read {}: {}", path.display(), e)))?;

    let value: toml::Value = toml::from_str(&content)
        .map_err(|e| CoreError::Config(format!("Failed to parse {}: {}", path.display(), e)))?;

    let mut roles = std::collections::HashMap::new();
    let mut mcp_servers = Vec::new();

    // Parse roles from [roles.*] sections
    if let Some(roles_table) = value.get("roles").and_then(|v| v.as_table()) {
        for (name, role_value) in roles_table {
            let role = orchestrator::RoleConfig {
                name: name.clone(),
                description: role_value.get("description").and_then(|v| v.as_str()).map(|s| s.to_string()),
                model: role_value.get("model").and_then(|v| v.as_str()).unwrap_or("gpt-4o").to_string(),
                temperature: role_value.get("temperature").and_then(|v| v.as_float()).unwrap_or(0.3) as f32,
                max_tokens: role_value.get("max_tokens").and_then(|v| v.as_integer()).unwrap_or(4096) as u32,
                system_prompt: role_value.get("system_prompt").and_then(|v| v.as_str()).map(|s| s.to_string()),
                tools: role_value.get("tools")
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                    .unwrap_or_default(),
                context_profile: role_value.get("context_profile").and_then(|v| v.as_str()).map(|s| s.to_string()),
                context_priority: role_value.get("context_priority").and_then(|v| v.as_str()).map(|s| s.to_string()),
            };
            roles.insert(name.clone(), role);
        }
    }

    // Parse MCP servers from [[mcp_servers]] sections
    if let Some(servers_array) = value.get("mcp_servers").and_then(|v| v.as_array()) {
        for server_value in servers_array {
            let name = server_value.get("name").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
            let command = server_value.get("command").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let args = server_value.get("args")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default();
            mcp_servers.push(McpServerConfig { name, command, args });
        }
    }

    Ok(ForgeConfig {
        roles,
        goals: Vec::new(),
        mcp_servers,
    })
}

/// Get the agents configured for a specific phase.
fn get_agents_for_phase(
    phase: &machine::phase::Phase,
    config: &ForgeConfig,
) -> Vec<orchestrator::RoleConfig> {
    match phase {
        machine::phase::Phase::Planning | machine::phase::Phase::Designing => {
            config.roles.get("architect").cloned().into_iter().collect()
        }
        machine::phase::Phase::Implementing => {
            config.roles.get("coder").cloned().into_iter().collect()
        }
        machine::phase::Phase::Reviewing | machine::phase::Phase::Fixing => {
            config.roles.get("reviewer").cloned().into_iter().collect()
        }
        machine::phase::Phase::Testing | machine::phase::Phase::SecurityScan => {
            vec![
                config.roles.get("tester").cloned(),
                config.roles.get("security").cloned(),
            ]
            .into_iter()
            .flatten()
            .collect()
        }
        machine::phase::Phase::Finalizing => {
            // No agents in finalizing
            Vec::new()
        }
        _ => Vec::new(),
    }
}

/// Get the next phase in the pipeline.
fn get_next_phase(current: &machine::phase::Phase) -> machine::phase::Phase {
    match current {
        machine::phase::Phase::Idle => machine::phase::Phase::Planning,
        machine::phase::Phase::Planning => machine::phase::Phase::Designing,
        machine::phase::Phase::Designing => machine::phase::Phase::Implementing,
        machine::phase::Phase::Implementing => machine::phase::Phase::Reviewing,
        machine::phase::Phase::Reviewing => machine::phase::Phase::Testing,
        machine::phase::Phase::Testing => machine::phase::Phase::SecurityScan,
        machine::phase::Phase::SecurityScan => machine::phase::Phase::Finalizing,
        machine::phase::Phase::Finalizing => machine::phase::Phase::Completed,
        machine::phase::Phase::Researching => machine::phase::Phase::Designing,
        machine::phase::Phase::Fixing => machine::phase::Phase::Reviewing,
        _ => machine::phase::Phase::Completed,
    }
}

// ─── Tests ────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_event_bus_basic() {
        let bus = EventBus::new();
        let mut rx = bus.subscribe();
        bus.publish(
            project_x_shared::protocol::MessageKind::SessionHeartbeat,
            "test",
        );
        let event = tokio::time::timeout(std::time::Duration::from_secs(1), rx.recv())
            .await
            .expect("timeout")
            .expect("recv error");
        assert_eq!(event.source, "test");
    }

    #[tokio::test]
    async fn test_event_bus_multiple_subscribers() {
        let bus = EventBus::new();
        let mut rx1 = bus.subscribe();
        let mut rx2 = bus.subscribe();
        bus.publish(
            project_x_shared::protocol::MessageKind::SessionHeartbeat,
            "test",
        );
        let _ = tokio::time::timeout(std::time::Duration::from_secs(1), rx1.recv()).await.expect("timeout").expect("recv error");
        let _ = tokio::time::timeout(std::time::Duration::from_secs(1), rx2.recv()).await.expect("timeout").expect("recv error");
    }

    #[tokio::test]
    async fn test_echo_agent() {
        let (actor_ref, _handle) = ractor::Actor::spawn(
            Some("test-echo".to_string()),
            actor::EchoAgent,
            "test-echo".to_string(),
        )
        .await
        .expect("Failed to spawn EchoAgent");

        let response = actor::echo(&actor_ref, "hello").await.expect("echo failed");
        assert!(response.contains("hello"));

        let pong = actor::ping(&actor_ref).await.expect("ping failed");
        assert_eq!(pong, "pong");

        let stats = actor::get_stats(&actor_ref).await.expect("stats failed");
        assert_eq!(stats.messages_processed, 3);
        assert_eq!(stats.agent_id, "test-echo");

        actor_ref.get_cell().stop(None);
    }

    #[tokio::test]
    async fn test_supervisor() {
        let supervisor = actor::Supervisor::spawn().await.expect("Failed to spawn Supervisor");

        let handle = actor::spawn_echo(&supervisor, "agent-1").await.expect("spawn failed");
        assert_eq!(handle.name, "agent-1");

        let handle2 = actor::spawn_echo(&supervisor, "agent-2").await.expect("spawn failed");
        assert_eq!(handle2.name, "agent-2");

        let response = actor::supervisor_echo_to(&supervisor, "agent-1", "test msg").await.expect("echo failed");
        assert!(response.contains("test msg"));

        let children = actor::list_children(&supervisor).await.expect("list failed");
        assert_eq!(children.len(), 2);

        let _ = actor::shutdown_all(&supervisor).await;
    }

    #[tokio::test]
    async fn test_core_runtime() {
        let runtime = CoreRuntime::new().await.expect("Failed to create runtime");

        let handle = runtime.spawn_echo_agent("test-agent").await.expect("spawn failed");
        assert_eq!(handle.name, "test-agent");

        let response = runtime.echo_to("test-agent", "hello runtime").await.expect("echo failed");
        assert!(response.contains("hello runtime"));

        let agents = runtime.list_agents().await.expect("list failed");
        assert_eq!(agents.len(), 1);

        let _ = runtime.shutdown().await;
    }
}