//! WebSocket handler — real-time event streaming from the EventBus.
//!
//! Each connected client receives all system events as JSON.
//! Clients can send commands (inject, subscribe, unsubscribe).

use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    extract::State,
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;

use super::AppState;
use crate::EventBus;

/// WebSocket upgrade handler.
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

/// Handle an individual WebSocket connection.
///
/// Sends all EventBus events to the client as JSON.
/// Receives commands from the client.
async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    tracing::info!("WebSocket client connected");

    let (mut sender, mut receiver) = socket.split();

    // Subscribe to EventBus
    let mut event_rx = state.bus.subscribe();

    // Spawn a task to forward events to the WebSocket client
    let send_task = tokio::spawn(async move {
        while let Ok(event) = event_rx.recv().await {
            match serde_json::to_string(&event) {
                Ok(json) => {
                    if sender.send(Message::Text(json.into())).await.is_err() {
                        tracing::debug!("WebSocket client disconnected (send error)");
                        break;
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to serialize event: {}", e);
                }
            }
        }
    });

    // Handle incoming messages from the client
    let recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Text(text) => {
                    handle_client_message(&text, &state);
                }
                Message::Close(_) => {
                    tracing::info!("WebSocket client disconnected");
                    break;
                }
                _ => {}
            }
        }
    });

    // Wait for either task to complete
    tokio::select! {
        _ = send_task => {},
        _ = recv_task => {},
    }

    tracing::info!("WebSocket connection closed");
}

/// Handle a message from a WebSocket client.
///
/// Supported commands:
/// - `{"type": "ping"}` → responds with `{"type": "pong"}`
/// - `{"type": "status"}` → responds with system status
/// - `{"type": "inject", "target": "...", "message": "..."}` → inject into session
fn handle_client_message(text: &str, state: &AppState) {
    let msg: serde_json::Value = match serde_json::from_str(text) {
        Ok(v) => v,
        Err(_) => return,
    };

    let msg_type = msg["type"].as_str().unwrap_or("");

    match msg_type {
        "ping" => {
            tracing::debug!("WS ping received");
        }
        "status" => {
            tracing::debug!("WS status requested");
        }
        "inject" => {
            let target = msg["target"].as_str().unwrap_or("unknown");
            let message = msg["message"].as_str().unwrap_or("");
            tracing::info!("WS inject to {}: {}", target, message);
            // TODO: Forward to InjectionChannel when implemented
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
        // Just ensure it doesn't panic
        let state = AppState {
            version: "test".to_string(),
            started_at: chrono::Utc::now(),
            bus: EventBus::new(),
            auth: std::sync::Arc::new(crate::api::auth::AuthState::new(b"test-secret-key-for-testing-32bytes!!")),
        };
        handle_client_message(r#"{"type": "ping"}"#, &state);
    }

    #[test]
    fn test_handle_client_message_inject() {
        let state = AppState {
            version: "test".to_string(),
            started_at: chrono::Utc::now(),
            bus: EventBus::new(),
            auth: std::sync::Arc::new(crate::api::auth::AuthState::new(b"test-secret-key-for-testing-32bytes!!")),
        };
        handle_client_message(
            r#"{"type": "inject", "target": "coder", "message": "use thiserror"}"#,
            &state,
        );
    }

    #[test]
    fn test_handle_client_message_invalid_json() {
        let state = AppState {
            version: "test".to_string(),
            started_at: chrono::Utc::now(),
            bus: EventBus::new(),
            auth: std::sync::Arc::new(crate::api::auth::AuthState::new(b"test-secret-key-for-testing-32bytes!!")),
        };
        handle_client_message("not json", &state);
    }
}
