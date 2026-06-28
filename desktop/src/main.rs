//! Project-X Desktop — Tauri v2 binary.
//!
//! Embeds the core runtime and serves the dashboard via WebView.

mod commands;
mod events;

use std::sync::Arc;

/// Application state shared across Tauri commands.
pub struct AppState {
    pub runtime: Option<project_x_core::CoreRuntime>,
}

impl AppState {
    pub fn new() -> Self {
        Self { runtime: None }
    }

    pub async fn init(&mut self) -> Result<(), String> {
        self.runtime = Some(
            project_x_core::CoreRuntime::new()
                .await
                .map_err(|e| e.to_string())?,
        );
        Ok(())
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    tauri::Builder::default()
        .setup(|app| {
            tracing::info!("Project-X Desktop starting");

            // Initialize core runtime in background
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                tracing::info!("Core runtime initialized");
                handle.emit("core:ready", ()).ok();
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_version,
            commands::get_status,
            commands::get_projects,
            commands::get_sessions,
            commands::run_goal,
            commands::stop_session,
            commands::get_metrics,
            commands::get_agents,
            commands::get_context,
            commands::send_prompt,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}