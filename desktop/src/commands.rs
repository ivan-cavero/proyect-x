//! Tauri IPC commands — functions exposed to the frontend.

use serde::{Deserialize, Serialize};

// ─── Response Types ───────────────────────────────────────────

#[derive(Serialize)]
pub struct VersionInfo {
    pub version: String,
    pub commit: String,
}

#[derive(Serialize)]
pub struct StatusInfo {
    pub running: bool,
    pub uptime: u64,
    pub version: String,
}

#[derive(Serialize)]
pub struct ProjectInfo {
    pub id: String,
    pub name: String,
    pub created_at: String,
}

#[derive(Serialize)]
pub struct SessionInfo {
    pub id: String,
    pub project_id: String,
    pub status: String,
    pub goal: String,
    pub phase: String,
    pub iteration: u32,
}

#[derive(Serialize)]
pub struct AgentInfo {
    pub id: String,
    pub role: String,
    pub model: String,
    pub status: String,
    pub asi_score: f32,
    pub context_pressure: f32,
}

#[derive(Serialize)]
pub struct MetricsInfo {
    pub total_tokens: u64,
    pub avg_asi: f32,
    pub active_sessions: u32,
    pub context_pressure: f32,
}

#[derive(Serialize)]
pub struct ContextInfo {
    pub model: String,
    pub context_window: usize,
    pub hard_limit: usize,
    pub pressure: f32,
    pub profile: String,
}

#[derive(Deserialize)]
pub struct RunGoalRequest {
    pub goal: String,
    pub agents: Option<Vec<String>>,
}

// ─── Commands ─────────────────────────────────────────────────

#[tauri::command]
pub fn get_version() -> VersionInfo {
    VersionInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        commit: option_env!("GIT_COMMIT").unwrap_or("unknown").to_string(),
    }
}

#[tauri::command]
pub async fn get_status() -> Result<StatusInfo, String> {
    Ok(StatusInfo {
        running: true,
        uptime: 0,
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

#[tauri::command]
pub fn get_projects() -> Vec<ProjectInfo> {
    vec![]
}

#[tauri::command]
pub fn get_sessions() -> Vec<SessionInfo> {
    vec![]
}

#[tauri::command]
pub async fn run_goal(request: RunGoalRequest) -> Result<String, String> {
    tracing::info!("Running goal: {}", request.goal);

    // Create core runtime
    let runtime = project_x_core::CoreRuntime::new()
        .await
        .map_err(|e| e.to_string())?;

    // Spawn agent
    let handle = runtime.spawn_echo_agent("coder")
        .await
        .map_err(|e| e.to_string())?;

    // Send goal
    let response = runtime.echo_to(&handle.name, &request.goal)
        .await
        .map_err(|e| e.to_string())?;

    // Shutdown
    let _ = runtime.shutdown().await;

    Ok(response)
}

#[tauri::command]
pub async fn stop_session(session_id: String) -> Result<(), String> {
    tracing::info!("Stopping session: {}", session_id);
    Ok(())
}

#[tauri::command]
pub fn get_metrics() -> MetricsInfo {
    MetricsInfo {
        total_tokens: 0,
        avg_asi: 100.0,
        active_sessions: 0,
        context_pressure: 0.0,
    }
}

#[tauri::command]
pub fn get_agents() -> Vec<AgentInfo> {
    vec![]
}

#[tauri::command]
pub fn get_context() -> ContextInfo {
    ContextInfo {
        model: "gpt-5".to_string(),
        context_window: 128_000,
        hard_limit: 89_600,
        pressure: 0.0,
        profile: "balanced".to_string(),
    }
}

#[tauri::command]
pub async fn send_prompt(agent_id: String, prompt: String) -> Result<String, String> {
    tracing::info!("Sending prompt to {}: {}", agent_id, prompt);
    Ok(format!("Received prompt: {}", prompt))
}