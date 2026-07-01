//! WebSocket handler — real-time event streaming from the EventBus.
//!
//! Each connected client receives all system events as JSON.
//! Clients can send commands (inject, subscribe, unsubscribe).

use std::sync::Arc;
use axum::extract::ws::{Message, WebSocket};
use axum::extract::State;
use futures_util::{SinkExt, StreamExt, stream::SplitSink};
use tokio::sync::broadcast;
use super::routes::AppState;

/// Client connection state
#[expect(dead_code, reason = "Planned for client subscription filtering")]
struct ClientState {
    sender: SplitSink<WebSocket, Message>,
    subscriptions: Vec<String>,
}

/// Spawn a task that forwards EventBus events to all connected clients.
pub fn start_event_forwarder(bus: &crate::EventBus, clients: broadcast::Sender<String>) {
    let mut rx = bus.subscribe();
    tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(event) => {
                    let msg = serde_json::json!({
                        "id": event.id,
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                        "kind": format!("{:?}", event.kind),
                        "source": event.source,
                        "metadata": event.metadata,
                    });
                    let payload = serde_json::to_string(&msg).unwrap_or_default();
                    let _ = clients.send(payload);
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("Event forwarder lagged by {} events", n);
                }
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}

/// Handle an incoming WebSocket connection.
pub async fn ws_handler(
    ws: axum::extract::ws::WebSocketUpgrade,
    State(state): axum::extract::State<Arc<AppState>>,
) -> axum::response::Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    let (mut sender, mut receiver) = socket.split();

    // Subscribe to EventBus
    let mut rx = state.bus.subscribe();

    // Spawn a task to forward events to this client
    let send_task = tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(event) => {
                    let msg = serde_json::json!({
                        "id": event.id,
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                        "kind": format!("{:?}", event.kind),
                        "source": event.source,
                        "metadata": event.metadata,
                    });
                    let payload = serde_json::to_string(&msg).unwrap_or_default();
                    if sender.send(Message::Text(payload.into())).await.is_err() {
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    tracing::debug!("Client lagged by {} events", n);
                }
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    });

    // Read incoming messages
    while let Some(msg) = receiver.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                handle_client_message(&text);
            }
            Ok(Message::Close(_)) => break,
            Err(e) => {
                tracing::debug!("WS error: {}", e);
                break;
            }
            _ => {}
        }
    }

    send_task.abort();
}

fn handle_client_message(text: &str) {
    let msg: serde_json::Value = match serde_json::from_str(text) {
        Ok(v) => v,
        Err(_) => {
            tracing::debug!("WS invalid JSON: {}", text);
            return;
        }
    };

    let msg_type = msg["type"].as_str().unwrap_or("unknown");

    match msg_type {
        "ping" => {
            tracing::debug!("WS ping received");
        }
        "inject" => {
            let target = msg["target"].as_str().unwrap_or("unknown");
            let message = msg["message"].as_str().unwrap_or("");
            tracing::info!("WS inject to {}: {}", target, message);
        }
        _ => {
            tracing::debug!("WS unknown command: {}", msg_type);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handle_client_message_ping() {
        handle_client_message(r#"{"type": "ping"}"#);
    }

    #[test]
    fn test_handle_client_message_inject() {
        handle_client_message(
            r#"{"type": "inject", "target": "coder", "message": "use thiserror"}"#,
        );
    }

    #[test]
    fn test_handle_client_message_invalid_json() {
        handle_client_message(r#"not-json"#);
    }

    #[test]
    fn test_handle_client_message_unknown() {
        handle_client_message(r#"{"type": "unknown_command"}"#);
    }
}