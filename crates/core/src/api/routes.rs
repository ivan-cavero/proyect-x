//! REST API routes.

use axum::{Router, routing::{get, post}, Json, extract::State, http::StatusCode};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Shared state for the API server.
#[derive(Clone)]
pub struct AppState {
    pub version: String,
    pub started_at: chrono::DateTime<chrono::Utc>,
}

/// API server configuration.
pub struct ApiServerConfig {
    pub port: u16,
    pub cors_origins: Vec<String>,
}

impl Default for ApiServerConfig {
    fn default() -> Self {
        Self {
            port: 8080,
            cors_origins: vec!["*".to_string()],
        }
    }
}

/// The main API server.
pub struct ApiServer {
    config: ApiServerConfig,
}

impl ApiServer {
    pub fn new(config: ApiServerConfig) -> Self {
        Self { config }
    }

    /// Build the Axum router with all routes.
    pub fn router(state: AppState) -> Router {
        let cors = tower_http::cors::CorsLayer::permissive();

        Router::new()
            // Health
            .route("/api/health", get(routes::health))
            // Projects
            .route("/api/projects", get(routes::list_projects))
            .route("/api/projects", post(routes::create_project))
            // Sessions
            .route("/api/sessions", get(routes::list_sessions))
            // Metrics
            .route("/api/metrics/tokens", get(routes::token_metrics))
            .route("/api/metrics/summary", get(routes::metrics_summary))
            // Context
            .route("/api/metrics/context", get(routes::context_metrics))
            // WebSocket
            .route("/ws/global", get(super::ws::ws_handler))
            // State
            .with_state(Arc::new(state))
            .layer(cors)
    }

    /// Start the server (non-blocking).
    pub async fn start(self) -> anyhow::Result<()> {
        let state = AppState {
            version: env!("CARGO_PKG_VERSION").to_string(),
            started_at: chrono::Utc::now(),
        };

        let app = Self::router(state);
        let addr = format!("0.0.0.0:{}", self.config.port);
        tracing::info!("API server starting on {}", addr);

        let listener = tokio::net::TcpListener::bind(&addr).await?;
        let local_addr = listener.local_addr()?;
        tracing::info!("API server listening on {}", local_addr);

        axum::serve(listener, app)
            .await
            .map_err(|e| anyhow::anyhow!("Server error: {}", e))?;

        Ok(())
    }
}

// ─── Response Types ───────────────────────────────────────────

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub uptime_seconds: u64,
}

#[derive(Serialize)]
pub struct ProjectResponse {
    pub id: String,
    pub name: String,
    pub created_at: String,
}

#[derive(Serialize)]
pub struct SessionResponse {
    pub id: String,
    pub project_id: String,
    pub status: String,
    pub goal: String,
    pub phase: String,
    pub iteration: u32,
}

#[derive(Serialize)]
pub struct TokenMetricsResponse {
    pub total_input: u64,
    pub total_output: u64,
    pub total_tokens: u64,
    pub by_provider: std::collections::HashMap<String, u64>,
    pub by_model: std::collections::HashMap<String, u64>,
}

#[derive(Serialize)]
pub struct ContextMetricsResponse {
    pub avg_pressure: f32,
    pub max_pressure: f32,
    pub total_compressions: u32,
    pub active_sessions: u32,
}

#[derive(Serialize)]
pub struct MetricsSummaryResponse {
    pub version: String,
    pub uptime_seconds: u64,
    pub active_sessions: u32,
    pub total_tokens: u64,
    pub avg_asi_score: f32,
}

#[derive(Deserialize)]
pub struct CreateProjectRequest {
    pub name: String,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub code: u16,
}

// ─── Route Handlers ───────────────────────────────────────────

pub mod routes {
    use super::*;

    pub async fn health(State(state): State<Arc<AppState>>) -> Json<HealthResponse> {
        let uptime = chrono::Utc::now()
            .signed_duration_since(state.started_at)
            .num_seconds() as u64;

        Json(HealthResponse {
            status: "ok".to_string(),
            version: state.version.clone(),
            uptime_seconds: uptime,
        })
    }

    pub async fn list_projects() -> Json<Vec<ProjectResponse>> {
        Json(vec![])
    }

    pub async fn create_project(
        Json(_request): Json<CreateProjectRequest>,
    ) -> (StatusCode, Json<ProjectResponse>) {
        let response = ProjectResponse {
            id: uuid::Uuid::new_v4().to_string(),
            name: "new-project".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        (StatusCode::CREATED, Json(response))
    }

    pub async fn list_sessions() -> Json<Vec<SessionResponse>> {
        Json(vec![])
    }

    pub async fn token_metrics() -> Json<TokenMetricsResponse> {
        Json(TokenMetricsResponse {
            total_input: 0,
            total_output: 0,
            total_tokens: 0,
            by_provider: std::collections::HashMap::new(),
            by_model: std::collections::HashMap::new(),
        })
    }

    pub async fn context_metrics() -> Json<ContextMetricsResponse> {
        Json(ContextMetricsResponse {
            avg_pressure: 0.0,
            max_pressure: 0.0,
            total_compressions: 0,
            active_sessions: 0,
        })
    }

    pub async fn metrics_summary(State(state): State<Arc<AppState>>) -> Json<MetricsSummaryResponse> {
        let uptime = chrono::Utc::now()
            .signed_duration_since(state.started_at)
            .num_seconds() as u64;

        Json(MetricsSummaryResponse {
            version: state.version.clone(),
            uptime_seconds: uptime,
            active_sessions: 0,
            total_tokens: 0,
            avg_asi_score: 100.0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_response_structure() {
        let state = AppState {
            version: "0.1.0".to_string(),
            started_at: chrono::Utc::now(),
        };

        let uptime = chrono::Utc::now()
            .signed_duration_since(state.started_at)
            .num_seconds() as u64;

        let response = HealthResponse {
            status: "ok".to_string(),
            version: state.version.clone(),
            uptime_seconds: uptime,
        };

        assert_eq!(response.status, "ok");
        assert_eq!(response.version, "0.1.0");
    }

    #[test]
    fn test_project_response_structure() {
        let response = ProjectResponse {
            id: uuid::Uuid::new_v4().to_string(),
            name: "test-project".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        assert!(!response.id.is_empty());
        assert_eq!(response.name, "test-project");
    }

    #[test]
    fn test_metrics_summary_structure() {
        let response = MetricsSummaryResponse {
            version: "0.1.0".to_string(),
            uptime_seconds: 100,
            active_sessions: 5,
            total_tokens: 10000,
            avg_asi_score: 85.0,
        };
        assert_eq!(response.version, "0.1.0");
        assert_eq!(response.active_sessions, 5);
        assert_eq!(response.avg_asi_score, 85.0);
    }
}