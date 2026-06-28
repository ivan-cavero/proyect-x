//! Actor definitions: Supervisor, EchoAgent, and role stubs.

pub mod agent;
pub mod echo;
pub mod supervisor;
pub mod roles;

pub use agent::{Agent, AgentInfo, AgentOutput, AgentRole, AgentStatus, AgentTask};
pub use echo::{EchoAgent, EchoMessage, EchoStats, echo, ping, get_stats};
pub use supervisor::{
    AgentHandle, Supervisor, SupervisorMessage, SupervisorState,
    spawn_echo, supervisor_echo_to, list_children, shutdown_all,
};