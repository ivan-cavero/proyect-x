//! # Project-X Core Runtime
//!
//! The heart of the system: actor model, state machine, orchestrator,
//! loop controller, drift detection, and context management.

pub mod actor;
pub mod bus;
pub mod drift;
pub mod r#loop;
pub mod machine;
pub mod workflow;
pub mod orchestrator;
pub mod api;

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
}

impl CoreRuntime {
    /// Create and start a new CoreRuntime.
    pub async fn new() -> Result<Self> {
        let bus = EventBus::new();
        let supervisor = actor::Supervisor::spawn().await?;

        Ok(Self { bus, supervisor })
    }

    /// Spawn a new EchoAgent via the supervisor.
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