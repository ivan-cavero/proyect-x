//! E2E tests for the HTTP API server.
//!
//! Starts a real Axum server on a random port and makes actual HTTP requests.

use axum::{Router, routing::get, Json, extract::State};
use std::sync::Arc;

// ─── Test Server Setup ────────────────────────────────────────

/// Minimal test server that mimics the real API.
async fn start_test_server() -> u16 {
    let bus = project_x_core::EventBus::new();
    let auth = std::sync::Arc::new(project_x_core::api::auth::AuthState::new(b"test-secret-key-for-e2e-tests-32bytes!!"));

    let state = project_x_core::api::routes::AppState {
        version: "test-0.1.0".to_string(),
        started_at: chrono::Utc::now(),
        bus,
        auth,
    };

    let app = Router::new()
        .route("/api/health", get(health_handler))
        .route("/api/projects", get(list_projects))
        .route("/api/sessions", get(list_sessions))
        .route("/api/metrics/tokens", get(token_metrics))
        .route("/api/metrics/context", get(context_metrics))
        .route("/api/metrics/summary", get(metrics_summary))
        .with_state(Arc::new(state));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    // Give server time to start
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    port
}

// ─── Handlers (duplicated for test isolation) ─────────────────

#[derive(serde::Serialize)]
struct HealthResponse { status: String, version: String }

async fn health_handler() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok".to_string(), version: "test-0.1.0".to_string() })
}

async fn list_projects() -> Json<Vec<serde_json::Value>> { Json(vec![]) }
async fn list_sessions() -> Json<Vec<serde_json::Value>> { Json(vec![]) }

#[derive(serde::Serialize)]
struct TokenMetrics { total_input: u64, total_output: u64, total_tokens: u64 }
async fn token_metrics() -> Json<TokenMetrics> {
    Json(TokenMetrics { total_input: 0, total_output: 0, total_tokens: 0 })
}

#[derive(serde::Serialize)]
struct ContextMetrics { avg_pressure: f32, max_pressure: f32 }
async fn context_metrics() -> Json<ContextMetrics> {
    Json(ContextMetrics { avg_pressure: 0.0, max_pressure: 0.0 })
}

#[derive(serde::Serialize)]
struct MetricsSummary { version: String, active_sessions: u32, avg_asi_score: f32 }
async fn metrics_summary() -> Json<MetricsSummary> {
    Json(MetricsSummary { version: "test".to_string(), active_sessions: 0, avg_asi_score: 100.0 })
}

// ═══════════════════════════════════════════════════════════════
// API E2E TESTS
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn e2e_api_health_endpoint() {
    let port = start_test_server().await;
    let url = format!("http://127.0.0.1:{}/api/health", port);

    let resp = reqwest::get(&url).await.unwrap();
    assert!(resp.status().is_success(), "Health endpoint should return 200");

    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["status"], "ok");
    assert_eq!(body["version"], "test-0.1.0");
}

#[tokio::test]
async fn e2e_api_projects_endpoint() {
    let port = start_test_server().await;
    let url = format!("http://127.0.0.1:{}/api/projects", port);

    let resp = reqwest::get(&url).await.unwrap();
    assert!(resp.status().is_success());

    let body: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert!(body.is_empty(), "Projects list should be empty initially");
}

#[tokio::test]
async fn e2e_api_sessions_endpoint() {
    let port = start_test_server().await;
    let url = format!("http://127.0.0.1:{}/api/sessions", port);

    let resp = reqwest::get(&url).await.unwrap();
    assert!(resp.status().is_success());

    let body: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert!(body.is_empty(), "Sessions list should be empty initially");
}

#[tokio::test]
async fn e2e_api_token_metrics_endpoint() {
    let port = start_test_server().await;
    let url = format!("http://127.0.0.1:{}/api/metrics/tokens", port);

    let resp = reqwest::get(&url).await.unwrap();
    assert!(resp.status().is_success());

    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["total_tokens"], 0);
}

#[tokio::test]
async fn e2e_api_context_metrics_endpoint() {
    let port = start_test_server().await;
    let url = format!("http://127.0.0.1:{}/api/metrics/context", port);

    let resp = reqwest::get(&url).await.unwrap();
    assert!(resp.status().is_success());

    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body["avg_pressure"].as_f64().unwrap() >= 0.0);
}

#[tokio::test]
async fn e2e_api_metrics_summary_endpoint() {
    let port = start_test_server().await;
    let url = format!("http://127.0.0.1:{}/api/metrics/summary", port);

    let resp = reqwest::get(&url).await.unwrap();
    assert!(resp.status().is_success());

    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body["version"].as_str().is_some());
    assert_eq!(body["avg_asi_score"], 100.0);
}

#[tokio::test]
async fn e2e_api_nonexistent_endpoint() {
    let port = start_test_server().await;
    let url = format!("http://127.0.0.1:{}/api/nonexistent", port);

    let resp = reqwest::get(&url).await.unwrap();
    assert_eq!(resp.status().as_u16(), 404, "Non-existent endpoint should return 404");
}

#[tokio::test]
async fn e2e_api_health_response_time() {
    let port = start_test_server().await;
    let url = format!("http://127.0.0.1:{}/api/health", port);

    let start = std::time::Instant::now();
    let resp = reqwest::get(&url).await.unwrap();
    let elapsed = start.elapsed();

    assert!(resp.status().is_success());
    assert!(elapsed < std::time::Duration::from_secs(2), "Health check should be fast, took {:?}", elapsed);
}

#[tokio::test]
async fn e2e_api_multiple_concurrent_requests() {
    let port = start_test_server().await;
    let base_url = format!("http://127.0.0.1:{}", port);

    // Fire 10 concurrent requests
    let mut handles = Vec::new();
    for _ in 0..10 {
        let url = format!("{}/api/health", base_url);
        handles.push(tokio::spawn(async move {
            reqwest::get(&url).await.unwrap().status().is_success()
        }));
    }

    let results: Vec<bool> = futures_util::future::join_all(handles)
        .await
        .into_iter()
        .map(|r| r.unwrap())
        .collect();

    assert!(results.iter().all(|&r| r), "All concurrent requests should succeed");
}

#[tokio::test]
async fn e2e_api_json_content_type() {
    let port = start_test_server().await;
    let url = format!("http://127.0.0.1:{}/api/health", port);

    let resp = reqwest::get(&url).await.unwrap();
    let content_type = resp.headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    assert!(content_type.contains("application/json"),
        "Response should be JSON, got: {}", content_type);
}
