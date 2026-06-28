//! WebSocket handler for real-time event streaming.

use axum::{
    extract::ws::{WebSocket, WebSocketUpgrade},
    extract::State,
    response::IntoResponse,
};
use futures_util::StreamExt;
use std::sync::Arc;

use super::AppState;

/// WebSocket upgrade handler.
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(_state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(handle_socket)
}

/// Handle an individual WebSocket connection.
async fn handle_socket(mut socket: WebSocket) {
    tracing::info!("WebSocket client connected");

    // Simple echo loop: read and respond
    while let Some(Ok(msg)) = socket.next().await {
        match msg {
            axum::extract::ws::Message::Text(text) => {
                tracing::debug!("WS received: {}", text);
                let ack = format!("{{\"ack\": \"{}\"}}", text);
                if socket.send(axum::extract::ws::Message::Text(ack.into())).await.is_err() {
                    break;
                }
            }
            axum::extract::ws::Message::Close(_) => {
                tracing::info!("WebSocket client disconnected");
                break;
            }
            _ => {}
        }
    }

    tracing::info!("WebSocket connection closed");
}