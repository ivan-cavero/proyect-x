//! EchoAgent — A test actor that echoes messages back.
//!
//! Uses ractor 0.15 API: ActorRef<Msg>, impl Future, no async_trait.

use ractor::{Actor, ActorRef, ActorProcessingErr, RpcReplyPort};

// ─── Messages ─────────────────────────────────────────────────

pub enum EchoMessage {
    Echo { content: String, reply: RpcReplyPort<String> },
    Ping(RpcReplyPort<String>),
    GetStats(RpcReplyPort<EchoStats>),
    Shutdown,
}

impl std::fmt::Debug for EchoMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EchoMessage::Echo { content, .. } => write!(f, "Echo({})", content),
            EchoMessage::Ping(_) => write!(f, "Ping"),
            EchoMessage::GetStats(_) => write!(f, "GetStats"),
            EchoMessage::Shutdown => write!(f, "Shutdown"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct EchoStats {
    pub messages_processed: u64,
    pub uptime_seconds: u64,
    pub agent_id: String,
}

// ─── Actor State ──────────────────────────────────────────────

pub struct EchoState {
    pub id: String,
    pub message_count: u64,
    pub started_at: std::time::Instant,
}

// ─── Actor Implementation (ractor 0.15) ───────────────────────

pub struct EchoAgent;

impl Actor for EchoAgent {
    type Msg = EchoMessage;
    type State = EchoState;
    type Arguments = String;

    fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        agent_id: Self::Arguments,
    ) -> impl std::future::Future<Output = Result<Self::State, ActorProcessingErr>> + Send {
        async move {
            Ok(EchoState {
                id: agent_id,
                message_count: 0,
                started_at: std::time::Instant::now(),
            })
        }
    }

    fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> impl std::future::Future<Output = Result<(), ActorProcessingErr>> + Send {
        async move {
            state.message_count += 1;

            match message {
                EchoMessage::Echo { content, reply } => {
                    let response = format!("[{}] echo: {}", state.id, content);
                    let _ = reply.send(response);
                }
                EchoMessage::Ping(reply) => {
                    let _ = reply.send("pong".to_string());
                }
                EchoMessage::GetStats(reply) => {
                    let _ = reply.send(EchoStats {
                        messages_processed: state.message_count,
                        uptime_seconds: state.started_at.elapsed().as_secs(),
                        agent_id: state.id.clone(),
                    });
                }
                EchoMessage::Shutdown => {
                    tracing::info!("EchoAgent '{}' shutting down", state.id);
                }
            }
            Ok(())
        }
    }
}

// ─── Helper Functions ─────────────────────────────────────────

pub async fn echo(agent: &ActorRef<EchoMessage>, content: &str) -> Result<String, crate::CoreError> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    agent
        .cast(EchoMessage::Echo {
            content: content.to_string(),
            reply: RpcReplyPort::from(tx),
        })
        .map_err(|e| crate::CoreError::Actor(format!("Failed to send echo: {}", e)))?;
    rx.await.map_err(|e| crate::CoreError::Actor(format!("Echo response error: {}", e)))
}

pub async fn ping(agent: &ActorRef<EchoMessage>) -> Result<String, crate::CoreError> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    agent
        .cast(EchoMessage::Ping(RpcReplyPort::from(tx)))
        .map_err(|e| crate::CoreError::Actor(format!("Failed to ping: {}", e)))?;
    rx.await.map_err(|e| crate::CoreError::Actor(format!("Ping response error: {}", e)))
}

pub async fn get_stats(agent: &ActorRef<EchoMessage>) -> Result<EchoStats, crate::CoreError> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    agent
        .cast(EchoMessage::GetStats(RpcReplyPort::from(tx)))
        .map_err(|e| crate::CoreError::Actor(format!("Failed to get stats: {}", e)))?;
    rx.await.map_err(|e| crate::CoreError::Actor(format!("Stats response error: {}", e)))
}