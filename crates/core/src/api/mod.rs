//! HTTP API server — REST + WebSocket for dashboard and external clients.

pub mod routes;
pub mod ws;
pub mod auth;

pub use routes::{ApiServer, AppState};
pub use ws::ws_handler;