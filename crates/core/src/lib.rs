//! # praxis Core Runtime
//!
//! The heart of the system: actor model, state machine, orchestrator,
//! loop controller, drift detection, and context management.

pub mod actor;
pub mod api;
pub mod bus;
pub mod completion;
pub mod drift;
pub mod r#loop;
pub mod machine;
pub mod workflow;
pub mod orchestrator;

#[cfg(test)]
mod integration_tests;

// Re-exports for convenience
pub use actor::*;
pub use bus::EventBus;
pub use drift::*;
pub use r#loop::*;
pub use machine::*;
pub use workflow::*;
pub use orchestrator::{RoleConfig, RoleOverride, GoalConfig, ResolvedRole};
pub use orchestrator::roles::ResolvedRole as AgentRoleResolved;
pub use orchestrator::{Task, TaskResult, TaskStatus};
pub use completion::{
    CompletionCriterion, OutcomeResult, OutcomeVerifier,
    CodingOutcomeVerifier, ManualCompletionVerifier, default_coding_criterion,
};

use praxis_mcp_host::McpHost;
use praxis_vault::VaultService;
use praxis_agent_traits::persistence::EventStore;

use thiserror::Error;

// ─── Error Types ──────────────────────────────────────────────

#[derive(Error, Debug)]
pub enum CoreError {
    #[error("Actor error: {0}")]
    Actor(String),

    #[error("Runtime error: {0}")]
    Runtime(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("State machine error: {0}")]
    StateMachine(String),

    #[error("Context error: {0}")]
    Context(String),

    #[error("Event bus error: {0}")]
    EventBus(String),

    #[error("IO error: {0}")]
    Io(String),

    #[error("Not implemented: {0}")]
    NotImplemented(String),
}

pub type Result<T> = std::result::Result<T, CoreError>;

// ─── Runtime ──────────────────────────────────────────────────

/// The central runtime that manages the entire system.
pub struct CoreRuntime {
    pub bus: EventBus,
    pub supervisor: ractor::ActorRef<actor::SupervisorMessage>,
    pub loop_controller: crate::r#loop::LoopController,
    pub drift_guard: crate::drift::DriftGuard,
    pub mcp_host: McpHost,
    pub pathology_detector: crate::r#loop::LoopPathologyDetector,
    pub completion_criterion: Option<CompletionCriterion>,
    /// Optional event store for checkpointing and event sourcing.
    pub event_store: Option<std::sync::Arc<praxis_persistence::SqliteEventStore>>,
    /// Current session ID (set when run_goal starts).
    pub session_id: Option<uuid::Uuid>,
    /// Flag set by Ctrl+C to request graceful shutdown.
    pub shutdown_requested: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl CoreRuntime {
    /// Create and start a new CoreRuntime.
    pub async fn new() -> Result<Self> {
        let bus = EventBus::new();
        let supervisor = actor::Supervisor::spawn().await?;
        let loop_controller = crate::r#loop::LoopController::new();
        let drift_guard = crate::drift::DriftGuard::new();
        let mcp_host = McpHost::new("praxis");
        let pathology_detector = crate::r#loop::LoopPathologyDetector::new();

        Ok(Self {
            bus,
            supervisor,
            loop_controller,
            drift_guard,
            mcp_host,
            pathology_detector,
            completion_criterion: None,
            event_store: None,
            session_id: None,
            shutdown_requested: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        })
    }

    /// Attach a SQLite event store for checkpointing and event sourcing.
    pub fn with_event_store(mut self, store: praxis_persistence::SqliteEventStore) -> Self {
        self.event_store = Some(std::sync::Arc::new(store));
        self
    }

    /// Get a handle to the shutdown flag. Set it to true to request graceful
    /// shutdown from outside the runtime (e.g., Ctrl+C handler).
    pub fn shutdown_handle(&self) -> std::sync::Arc<std::sync::atomic::AtomicBool> {
        self.shutdown_requested.clone()
    }

    /// Save a checkpoint of the current session state to the event store.
    ///
    /// Called after each phase transition. If the process crashes, `resume_goal`
    /// can load this checkpoint and continue from where it left off.
    async fn save_checkpoint(&self, goal: &str) {
        let Some(store) = &self.event_store else {
            return;
        };
        let Some(session_id) = self.session_id else {
            return;
        };

        let state = serde_json::json!({
            "goal": goal,
            "phase": format!("{:?}", self.loop_controller.state_machine.current()),
            "iteration": self.loop_controller.iteration,
            "phase_iterations": self.loop_controller.phase_iterations,
            "started_at": self.loop_controller.started_at.elapsed().as_secs(),
        });

        let snapshot = praxis_agent_traits::persistence::StoredSnapshot {
            aggregate_id: session_id,
            aggregate_type: "session".to_string(),
            state,
            version: self.loop_controller.iteration as i64,
            updated_at: chrono::Utc::now().to_rfc3339(),
        };

        if let Err(e) = store.save_snapshot(snapshot).await {
            tracing::warn!("Failed to save checkpoint: {}", e);
        } else {
            tracing::debug!(
                "Checkpoint saved: session={}, iteration={}",
                session_id,
                self.loop_controller.iteration
            );
        }
    }

    /// Load the last checkpoint for a session, if one exists.
    pub async fn load_checkpoint(
        &self,
        session_id: uuid::Uuid,
    ) -> Option<praxis_agent_traits::persistence::StoredSnapshot> {
        let store = self.event_store.as_ref()?;
        match store.get_snapshot(session_id).await {
            Ok(snap) => snap,
            Err(e) => {
                tracing::warn!("Failed to load checkpoint: {}", e);
                None
            }
        }
    }

    /// Connect MCP servers defined in the forge.toml config.
    pub async fn connect_mcp_servers(&mut self, config: &ForgeConfig) {
        for server_config in &config.mcp_servers {
            tracing::info!("Connecting to MCP server: {} ({} {:?})",
                server_config.name, server_config.command, server_config.args);
            match self.mcp_host.connect_server(
                &server_config.name,
                &server_config.command,
                &server_config.args,
            ).await {
                Ok(()) => {
                    let tools = self.mcp_host.tools_for(&server_config.name);
                    tracing::info!("MCP server '{}' connected with {} tools",
                        server_config.name, tools.len());
                }
                Err(e) => {
                    tracing::warn!("Failed to connect MCP server '{}': {}",
                        server_config.name, e);
                }
            }
        }
    }

    /// Initialize a ProviderRouter from forge.toml providers + vault/env.
    ///
    /// For each provider in forge.toml, resolves the API key from:
    /// 1. VaultService (if key stored via Settings)
    /// 2. Environment variable (fallback for env:VAR_NAME references)
    /// 3. Literal key in config (warning: insecure)
    pub async fn init_providers(
        &self,
        config: &ForgeConfig,
        vault: Option<&VaultService>,
    ) -> praxis_providers::ProviderRouter {
        let mut router = praxis_providers::ProviderRouter::new();

        for (name, provider_cfg) in &config.providers {
            tracing::info!("Initializing provider: {} ({})", name, provider_cfg.base_url);

            // Resolve API key
            let api_key = self.resolve_api_key(&provider_cfg.api_key_ref, vault, name);

            if api_key.is_empty() {
                tracing::warn!("No API key for provider '{}'. Agent will use mock behavior.", name);
                continue;
            }

            let provider: std::sync::Arc<dyn praxis_providers::LLMProvider> =
                match provider_cfg.name.as_str() {
                    "nan" | "openai" | "openai_compat" => {
                        match praxis_providers::OpenAIProvider::new(
                            api_key,
                            provider_cfg.default_model.clone(),
                            Some(provider_cfg.base_url.clone()),
                            None,
                            None,
                        ) {
                            Ok(p) => std::sync::Arc::new(p),
                            Err(e) => {
                                tracing::warn!("Failed to init OpenAI provider '{}': {}. Using mock.", name, e.0);
                                continue;
                            }
                        }
                    }
                    "anthropic" => {
                        match praxis_providers::AnthropicProvider::new(
                            api_key,
                            provider_cfg.default_model.clone(),
                            Some(provider_cfg.base_url.clone()),
                            None,
                            None,
                        ) {
                            Ok(p) => std::sync::Arc::new(p),
                            Err(e) => {
                                tracing::warn!("Failed to init Anthropic provider '{}': {}. Using mock.", name, e.0);
                                continue;
                            }
                        }
                    }
                    "gemini" => {
                        match praxis_providers::GeminiProvider::new(
                            api_key,
                            provider_cfg.default_model.clone(),
                            Some(provider_cfg.base_url.clone()),
                            None,
                            None,
                        ) {
                            Ok(p) => std::sync::Arc::new(p),
                            Err(e) => {
                                tracing::warn!("Failed to init Gemini provider '{}': {}. Using mock.", name, e.0);
                                continue;
                            }
                        }
                    }
                    "ollama" => {
                        match praxis_providers::OllamaProvider::new(
                            provider_cfg.default_model.clone(),
                            Some(provider_cfg.base_url.clone()),
                        ) {
                            Ok(p) => std::sync::Arc::new(p),
                            Err(e) => {
                                tracing::warn!("Failed to init Ollama provider '{}': {}. Using mock.", name, e.0);
                                continue;
                            }
                        }
                    }
                    _ => {
                        match praxis_providers::OpenAIProvider::new(
                            api_key,
                            provider_cfg.default_model.clone(),
                            Some(provider_cfg.base_url.clone()),
                            None,
                            None,
                        ) {
                            Ok(p) => std::sync::Arc::new(p),
                            Err(e) => {
                                tracing::warn!("Failed to init provider '{}': {}. Using mock.", name, e.0);
                                continue;
                            }
                        }
                    }
                };

            router.register(name, provider, praxis_providers::ModelTier::Balanced);
            tracing::info!("Provider '{}' registered with model '{}'", name, provider_cfg.default_model);
        }

        router
    }

    /// Resolve an API key from vault, env, or config literal.
    fn resolve_api_key(&self, ref_str: &str, vault: Option<&VaultService>, provider_name: &str) -> String {
        // 1. Try vault first (keys stored via Settings)
        if let Some(v) = vault {
            if let Ok(Some(key)) = v.get(provider_name) {
                if !key.is_empty() {
                    tracing::info!("Loaded API key for '{}' from vault", provider_name);
                    return key;
                }
            }
        }

        // 2. Try env:VAR_NAME reference
        if let Some(var_name) = ref_str.strip_prefix("env:") {
            if let Ok(value) = std::env::var(var_name) {
                if !value.is_empty() {
                    tracing::info!("Loaded API key for '{}' from env:{}", provider_name, var_name);
                    return value;
                }
            }
        }

        // 3. Try literal key in config
        if !ref_str.is_empty() {
            if ref_str.starts_with("sk-") || ref_str.starts_with("xai-") {
                tracing::warn!("⚠️  Using literal API key in config for '{}' — consider using Settings page", provider_name);
            }
            return ref_str.to_string();
        }

        String::new()
    }

    /// Run a goal through the agent pipeline with a real iteration loop.
    ///
    /// The loop iterates: Planning → Designing → Implementing → Reviewing.
    /// If review gates fail → Fixing → Implementing → Reviewing (loop).
    /// If gates pass → Testing → SecurityScan → Finalizing → Completed.
    /// Stops when goal is complete or hard limits are reached.
    ///
    /// If `vault` is provided, providers are initialized from forge.toml + vault keys.
    /// When no forge.toml exists, runs using default mock agents.
    pub async fn run_goal(
        &mut self,
        goal: &str,
        config_path: Option<&std::path::Path>,
        vault: Option<&VaultService>,
    ) -> Result<GoalResult> {
        tracing::info!("Starting goal: {}", goal);

        let config = match config_path.map(load_forge_config) {
            Some(Ok(cfg)) => cfg,
            Some(Err(e)) => {
                tracing::warn!("Failed to load config: {}. Using defaults.", e);
                ForgeConfig::empty()
            }
            None => {
                tracing::info!("No forge.toml found. Using default mock agents.");
                ForgeConfig::empty()
            }
        };

        let provider_router = self.init_providers(&config, vault).await;

        if config.roles.is_empty() {
            tracing::info!("No roles defined in config. Using default coder role.");
        }

        // Register quality gates for review/test/security phases
        self.register_default_gates();

        // Set up outcome-based completion criterion (default: coding verifier)
        if self.completion_criterion.is_none() {
            self.completion_criterion = Some(default_coding_criterion());
        }
        self.pathology_detector.reset();

        // Assign a session ID
        self.session_id = Some(uuid::Uuid::new_v4());

        self.loop_controller.start();
        self.bus.publish(
            praxis_shared::protocol::MessageKind::SessionHeartbeat,
            "core",
        );

        self.loop_controller
            .advance(machine::phase::Phase::Planning)
            .map_err(CoreError::StateMachine)?;

        let mut results = Vec::new();
        let mut feedback = String::new();
        let mut current_phase = machine::phase::Phase::Planning;

        loop {
            if current_phase.is_terminal() {
                break;
            }

            // Check for graceful shutdown request (Ctrl+C)
            if self
                .shutdown_requested
                .load(std::sync::atomic::Ordering::SeqCst)
            {
                tracing::info!("Shutdown requested. Saving checkpoint and stopping.");
                self.save_checkpoint(goal).await;
                break;
            }

            if let Some(violation) = self.loop_controller.check_limits() {
                tracing::warn!("Limit reached: {}. Stopping loop.", violation);
                self.save_checkpoint(goal).await;
                break;
            }

            tracing::info!(
                "Phase: {} (iteration {})",
                current_phase,
                self.loop_controller.iteration
            );

            let phase_agents = get_agents_for_phase(&current_phase, &config);

            for role_config in &phase_agents {
                let mut task = orchestrator::Task::new(
                    &role_config.name,
                    &role_config.model,
                    goal,
                );

                let has_feedback = !feedback.is_empty() && role_config.name == "coder";
                if has_feedback {
                    task.context = feedback.clone();
                }

                let resolved_role =
                    orchestrator::roles::ResolvedRole::resolve(role_config, None);
                let agent = match provider_router.resolve(&role_config.model) {
                    Ok(provider) => {
                        crate::actor::roles::AgentFactory::create_with_provider(
                            &resolved_role,
                            provider,
                        )
                    }
                    Err(_) => {
                        tracing::warn!(
                            "No provider for model '{}'. Using mock agent for '{}'.",
                            role_config.model,
                            role_config.name
                        );
                        crate::actor::roles::AgentFactory::create(&resolved_role)
                    }
                };

                let result = if has_feedback {
                    agent.handle_feedback(&task, &feedback).await
                } else {
                    agent.execute(&task).await
                };

                tracing::info!(
                    "Agent {} completed: status={:?}, duration={}ms",
                    result.agent_id,
                    result.status,
                    result.duration_ms
                );

                results.push(result);
            }

            // Evaluate gates for quality-check phases
            if matches!(
                current_phase,
                machine::phase::Phase::Reviewing
                    | machine::phase::Phase::Testing
                    | machine::phase::Phase::SecurityScan
            ) {
                let review_results = extract_review_results(&results);
                self.loop_controller.add_results(review_results);
                let gates_pass = self.loop_controller.all_gates_pass();

                if !gates_pass {
                    feedback = consolidate_feedback(&results);

                    // Check if any gate has exceeded its retry limit
                    let phase = self.loop_controller.state_machine.current();
                    let gates_exceeded: Vec<&machine::gate::Gate> = self
                        .loop_controller
                        .gates
                        .gates_for(&phase)
                        .into_iter()
                        .filter(|g| g.is_exceeded())
                        .collect();

                    if !gates_exceeded.is_empty() {
                        let gate_names: Vec<&str> =
                            gates_exceeded.iter().map(|g| g.name.as_str()).collect();
                        tracing::warn!(
                            "Gate retry limit exceeded for: {}. Marking goal as failed.",
                            gate_names.join(", ")
                        );
                        current_phase = machine::phase::Phase::Failed;
                        self.loop_controller
                            .advance(machine::phase::Phase::Failed)
                            .map_err(CoreError::StateMachine)?;
                        break;
                    }

                    tracing::info!(
                        "Gate failed on {:?}. Going to Fixing. Feedback: {} chars",
                        current_phase,
                        feedback.len()
                    );
                    current_phase = machine::phase::Phase::Fixing;
                    self.loop_controller
                        .advance(machine::phase::Phase::Fixing)
                        .map_err(CoreError::StateMachine)?;
                    self.loop_controller.increment_iteration();
                    continue;
                } else {
                    if !feedback.is_empty() {
                        tracing::info!("Gates passed after fix. Clearing feedback.");
                        feedback.clear();
                    }
                }
            }

            // ── Pathology detection ──────────────────────────────
            // Check the last agent's output for destructive/stuck patterns.
            if let Some(last_result) = results.last() {
                let phase_str = format!("{:?}", current_phase);
                if let Some(alert) = self.pathology_detector.record_iteration(
                    self.loop_controller.iteration,
                    &last_result.content,
                    &phase_str,
                ) {
                    tracing::error!(
                        "Loop pathology detected: {:?} — {}",
                        alert.kind,
                        alert.details
                    );

                    // Fatal pathology → kill the loop immediately
                    if alert.severity == r#loop::PathologySeverity::Fatal {
                        tracing::error!(
                            "Fatal pathology: {}. Stopping loop immediately.",
                            alert.details
                        );
                        break;
                    }
                }
            }

            // ── Completion criterion (outcome-based) ─────────────
            // After quality-check phases, verify if the goal is actually achieved.
            if matches!(
                current_phase,
                machine::phase::Phase::Reviewing
                    | machine::phase::Phase::Testing
                    | machine::phase::Phase::SecurityScan
                    | machine::phase::Phase::Finalizing
            ) {
                if let Some(criterion) = &mut self.completion_criterion {
                    let outcome = criterion.evaluate(goal, &results).await;

                    match outcome {
                        completion::OutcomeResult::Achieved { evidence, .. } => {
                            tracing::info!(
                                "Goal achieved (verified by {}). Evidence: {}",
                                criterion.verifier_name(),
                                &evidence[..evidence.len().min(200)]
                            );
                            current_phase = machine::phase::Phase::Completed;
                            self.loop_controller
                                .advance(machine::phase::Phase::Completed)
                                .map_err(CoreError::StateMachine)?;
                            break;
                        }
                        completion::OutcomeResult::Exhausted { reason } => {
                            tracing::warn!(
                                "Goal exhausted: {}. Stopping loop.",
                                reason
                            );
                            break;
                        }
                        completion::OutcomeResult::NotAchieved { reason } => {
                            tracing::info!(
                                "Goal not yet achieved: {}. Continuing.",
                                reason
                            );
                        }
                    }
                }
            }

            let next_phase = get_next_phase(&current_phase);
            match self.loop_controller.advance(next_phase) {
                Ok(_transition) => {
                    self.bus.publish(
                        praxis_shared::protocol::MessageKind::PhaseChanged(
                            praxis_shared::protocol::PhaseTransition {
                                from: praxis_shared::types::Phase::Planning,
                                to: praxis_shared::types::Phase::Implementing,
                                condition: "automatic".to_string(),
                                timestamp: chrono::Utc::now().to_rfc3339(),
                            },
                        ),
                        "core",
                    );
                }
                Err(e) => {
                    tracing::error!("Failed to advance phase: {}", e);
                    break;
                }
            }

            current_phase = next_phase;
            self.loop_controller.increment_iteration();

            // Save checkpoint after each phase transition
            self.save_checkpoint(goal).await;
        }

        self.loop_controller.stop();

        // Save final checkpoint
        self.save_checkpoint(goal).await;

        let total_duration: u64 = results.iter().map(|r| r.duration_ms).sum();
        let passed = current_phase == machine::phase::Phase::Completed;

        tracing::info!(
            "Goal '{}' finished: phases={}, iterations={}, agents={}, passed={}, duration={}ms",
            goal,
            self.loop_controller.state_machine.history().len(),
            self.loop_controller.iteration,
            results.len(),
            passed,
            total_duration,
        );

        Ok(GoalResult {
            goal: goal.to_string(),
            passed,
            agent_results: results,
            total_duration_ms: total_duration,
        })
    }

    /// Resume a goal from the last checkpoint.
    ///
    /// Loads the session state from the event store and continues the loop
    /// from where it left off. Returns `None` if no checkpoint exists.
    pub async fn resume_goal(
        &mut self,
        session_id: uuid::Uuid,
        config_path: Option<&std::path::Path>,
        vault: Option<&VaultService>,
    ) -> Result<Option<GoalResult>> {
        let checkpoint = match self.load_checkpoint(session_id).await {
            Some(snap) => snap,
            None => {
                tracing::info!("No checkpoint found for session {}", session_id);
                return Ok(None);
            }
        };

        let goal = checkpoint
            .state
            .get("goal")
            .and_then(|v| v.as_str())
            .unwrap_or("resumed goal")
            .to_string();

        let saved_iteration = checkpoint
            .state
            .get("iteration")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32;

        let saved_phase = checkpoint
            .state
            .get("phase")
            .and_then(|v| v.as_str())
            .unwrap_or("Planning");

        tracing::info!(
            "Resuming session {} at phase={}, iteration={}",
            session_id,
            saved_phase,
            saved_iteration
        );

        // Restore session state
        self.session_id = Some(session_id);
        self.loop_controller.iteration = saved_iteration;

        // Re-register gates and completion criterion
        self.register_default_gates();
        if self.completion_criterion.is_none() {
            self.completion_criterion = Some(default_coding_criterion());
        }
        self.pathology_detector.reset();

        // Load config
        let config = match config_path.map(load_forge_config) {
            Some(Ok(cfg)) => cfg,
            _ => ForgeConfig::empty(),
        };
        let provider_router = self.init_providers(&config, vault).await;

        self.loop_controller.start();

        let mut results = Vec::new();
        let mut feedback = String::new();
        let mut current_phase = machine::phase::Phase::Planning;

        // Same loop as run_goal
        loop {
            if current_phase.is_terminal() {
                break;
            }

            if self
                .shutdown_requested
                .load(std::sync::atomic::Ordering::SeqCst)
            {
                tracing::info!("Shutdown requested. Saving checkpoint and stopping.");
                self.save_checkpoint(&goal).await;
                break;
            }

            if let Some(violation) = self.loop_controller.check_limits() {
                tracing::warn!("Limit reached: {}. Stopping loop.", violation);
                self.save_checkpoint(&goal).await;
                break;
            }

            tracing::info!(
                "Phase: {} (iteration {})",
                current_phase,
                self.loop_controller.iteration
            );

            let phase_agents = get_agents_for_phase(&current_phase, &config);

            for role_config in &phase_agents {
                let mut task = orchestrator::Task::new(
                    &role_config.name,
                    &role_config.model,
                    &goal,
                );

                let has_feedback = !feedback.is_empty() && role_config.name == "coder";
                if has_feedback {
                    task.context = feedback.clone();
                }

                let resolved_role =
                    orchestrator::roles::ResolvedRole::resolve(role_config, None);
                let agent = match provider_router.resolve(&role_config.model) {
                    Ok(provider) => {
                        crate::actor::roles::AgentFactory::create_with_provider(
                            &resolved_role,
                            provider,
                        )
                    }
                    Err(_) => {
                        crate::actor::roles::AgentFactory::create(&resolved_role)
                    }
                };

                let result = if has_feedback {
                    agent.handle_feedback(&task, &feedback).await
                } else {
                    agent.execute(&task).await
                };

                results.push(result);
            }

            if matches!(
                current_phase,
                machine::phase::Phase::Reviewing
                    | machine::phase::Phase::Testing
                    | machine::phase::Phase::SecurityScan
            ) {
                let review_results = extract_review_results(&results);
                self.loop_controller.add_results(review_results);
                let gates_pass = self.loop_controller.all_gates_pass();

                if !gates_pass {
                    feedback = consolidate_feedback(&results);
                    current_phase = machine::phase::Phase::Fixing;
                    self.loop_controller
                        .advance(machine::phase::Phase::Fixing)
                        .map_err(CoreError::StateMachine)?;
                    self.loop_controller.increment_iteration();
                    continue;
                } else {
                    feedback.clear();
                }
            }

            if let Some(last_result) = results.last() {
                let phase_str = format!("{:?}", current_phase);
                if let Some(alert) = self.pathology_detector.record_iteration(
                    self.loop_controller.iteration,
                    &last_result.content,
                    &phase_str,
                ) {
                    tracing::error!(
                        "Loop pathology detected: {:?} — {}",
                        alert.kind,
                        alert.details
                    );
                    if alert.severity == r#loop::PathologySeverity::Fatal {
                        break;
                    }
                }
            }

            if matches!(
                current_phase,
                machine::phase::Phase::Reviewing
                    | machine::phase::Phase::Testing
                    | machine::phase::Phase::SecurityScan
                    | machine::phase::Phase::Finalizing
            ) {
                if let Some(criterion) = &mut self.completion_criterion {
                    let outcome = criterion.evaluate(&goal, &results).await;
                    match outcome {
                        completion::OutcomeResult::Achieved { .. } => {
                            current_phase = machine::phase::Phase::Completed;
                            self.loop_controller
                                .advance(machine::phase::Phase::Completed)
                                .map_err(CoreError::StateMachine)?;
                            break;
                        }
                        completion::OutcomeResult::Exhausted { reason } => {
                            tracing::warn!("Goal exhausted: {}. Stopping.", reason);
                            break;
                        }
                        completion::OutcomeResult::NotAchieved { reason } => {
                            tracing::info!("Goal not yet achieved: {}. Continuing.", reason);
                        }
                    }
                }
            }

            let next_phase = get_next_phase(&current_phase);
            match self.loop_controller.advance(next_phase) {
                Ok(_) => {}
                Err(e) => {
                    tracing::error!("Failed to advance phase: {}", e);
                    break;
                }
            }

            current_phase = next_phase;
            self.loop_controller.increment_iteration();
            self.save_checkpoint(&goal).await;
        }

        self.loop_controller.stop();
        self.save_checkpoint(&goal).await;

        let total_duration: u64 = results.iter().map(|r| r.duration_ms).sum();
        let passed = current_phase == machine::phase::Phase::Completed;

        Ok(Some(GoalResult {
            goal,
            passed,
            agent_results: results,
            total_duration_ms: total_duration,
        }))
    }

    /// Register default quality gates for the standard pipeline.
    fn register_default_gates(&mut self) {
        use machine::gate::{Gate, GateEvaluator};

        self.loop_controller.gates.register(
            machine::phase::Phase::Reviewing,
            Gate::new("review.pass", GateEvaluator::AllAgentsPass, 3),
        );
        self.loop_controller.gates.register(
            machine::phase::Phase::SecurityScan,
            Gate::new("security.no_critical", GateEvaluator::NoCritical, 3),
        );
        self.loop_controller.gates.register(
            machine::phase::Phase::Testing,
            Gate::new("test.pass", GateEvaluator::AllAgentsPass, 3),
        );
    }

    /// Spawn a new EchoAgent via the supervisor (for testing).
    pub async fn spawn_echo_agent(&self, name: &str) -> Result<actor::AgentHandle> {
        actor::spawn_echo(&self.supervisor, name).await
    }

    /// Send an echo message to a named child agent.
    pub async fn echo_to(&self, child_name: &str, content: &str) -> Result<String> {
        actor::supervisor_echo_to(&self.supervisor, child_name, content).await
    }

    /// List all running child agents.
    pub async fn list_agents(&self) -> Result<Vec<actor::AgentHandle>> {
        actor::list_children(&self.supervisor).await
    }

    /// Shutdown all agents and stop the runtime.
    pub async fn shutdown(&self) -> Result<()> {
        actor::shutdown_all(&self.supervisor).await
    }
}

// ─── Goal Result ──────────────────────────────────────────────

/// Result of running a goal through the pipeline.
#[derive(Debug, Clone)]
pub struct GoalResult {
    pub goal: String,
    pub passed: bool,
    pub agent_results: Vec<orchestrator::TaskResult>,
    pub total_duration_ms: u64,
}

// ─── Config ───────────────────────────────────────────────────

/// Parsed forge.toml configuration.
pub struct ForgeConfig {
    pub roles: std::collections::HashMap<String, orchestrator::RoleConfig>,
    pub goals: Vec<orchestrator::GoalConfig>,
    pub mcp_servers: Vec<McpServerConfig>,
    /// Provider definitions from [providers.*] sections. Key is provider name.
    pub providers: std::collections::HashMap<String, ProviderConfig>,
}

/// Provider configuration from forge.toml [providers.*].
pub struct ProviderConfig {
    pub name: String,
    pub base_url: String,
    pub api_key_ref: String, // "env:VAR" | "vault:provider_name" | "literal-key"
    pub default_model: String,
}

/// MCP server configuration from forge.toml.
pub struct McpServerConfig {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
}

impl Default for ForgeConfig {
    fn default() -> Self {
        // Deprecated — use ForgeConfig::empty() instead.
        // Default impl kept for backward compatibility with tests.
        Self::empty()
    }
}

impl ForgeConfig {
    /// Create an empty config (no roles, no providers, no goals).
    /// Used when no forge.toml exists — agents run in mock mode.
    pub fn empty() -> Self {
        Self {
            roles: std::collections::HashMap::new(),
            goals: Vec::new(),
            mcp_servers: Vec::new(),
            providers: std::collections::HashMap::new(),
        }
    }
}

/// Load forge.toml configuration from a file.
pub fn load_forge_config(path: &std::path::Path) -> Result<ForgeConfig> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| CoreError::Config(format!("Failed to read {}: {}", path.display(), e)))?;

    let value: toml::Value = toml::from_str(&content)
        .map_err(|e| CoreError::Config(format!("Failed to parse {}: {}", path.display(), e)))?;

    let mut roles = std::collections::HashMap::new();
    let mut mcp_servers = Vec::new();
    let mut providers = std::collections::HashMap::new();

    // Parse roles from [roles.*] sections
    if let Some(roles_table) = value.get("roles").and_then(|v| v.as_table()) {
        for (name, role_value) in roles_table {
            let role = orchestrator::RoleConfig {
                name: name.clone(),
                description: role_value.get("description").and_then(|v| v.as_str()).map(|s| s.to_string()),
                model: role_value.get("model").and_then(|v| v.as_str()).unwrap_or("gpt-4o").to_string(),
                temperature: role_value.get("temperature").and_then(|v| v.as_float()).unwrap_or(0.3) as f32,
                max_tokens: role_value.get("max_tokens").and_then(|v| v.as_integer()).unwrap_or(4096) as u32,
                system_prompt: role_value.get("system_prompt").and_then(|v| v.as_str()).map(|s| s.to_string()),
                tools: role_value.get("tools")
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                    .unwrap_or_default(),
                context_profile: role_value.get("context_profile").and_then(|v| v.as_str()).map(|s| s.to_string()),
                context_priority: role_value.get("context_priority").and_then(|v| v.as_str()).map(|s| s.to_string()),
            };
            roles.insert(name.clone(), role);
        }
    }

    // Parse providers from [providers.*] sections
    if let Some(providers_table) = value.get("providers").and_then(|v| v.as_table()) {
        for (name, provider_value) in providers_table {
            let base_url = provider_value.get("base_url")
                .and_then(|v| v.as_str())
                .unwrap_or("https://api.openai.com/v1")
                .to_string();
            let api_key_ref = provider_value.get("api_key")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let default_model = provider_value.get("default_model")
                .and_then(|v| v.as_str())
                .unwrap_or("gpt-4o")
                .to_string();

            providers.insert(name.clone(), ProviderConfig {
                name: name.clone(),
                base_url,
                api_key_ref,
                default_model,
            });
        }
    }

    // Parse MCP servers from [[mcp_servers]] sections
    if let Some(servers_array) = value.get("mcp_servers").and_then(|v| v.as_array()) {
        for server_value in servers_array {
            let name = server_value.get("name").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
            let command = server_value.get("command").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let args = server_value.get("args")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default();
            mcp_servers.push(McpServerConfig { name, command, args });
        }
    }

    Ok(ForgeConfig {
        roles,
        goals: Vec::new(),
        mcp_servers,
        providers,
    })
}

/// Get the agents configured for a specific phase.
///
/// When no roles are configured (no forge.toml), uses default mock roles
/// so the pipeline still runs end-to-end.
fn get_agents_for_phase(
    phase: &machine::phase::Phase,
    config: &ForgeConfig,
) -> Vec<orchestrator::RoleConfig> {
    let lookup = |name: &str| -> Option<orchestrator::RoleConfig> {
        config.roles.get(name).cloned().or_else(|| {
            if config.roles.is_empty() {
                Some(default_role(name))
            } else {
                None
            }
        })
    };

    match phase {
        machine::phase::Phase::Planning | machine::phase::Phase::Designing => {
            lookup("architect").into_iter().collect()
        }
        machine::phase::Phase::Implementing => {
            lookup("coder").into_iter().collect()
        }
        machine::phase::Phase::Reviewing | machine::phase::Phase::Fixing => {
            lookup("reviewer").into_iter().collect()
        }
        machine::phase::Phase::Testing | machine::phase::Phase::SecurityScan => {
            vec![lookup("tester"), lookup("security")]
                .into_iter()
                .flatten()
                .collect()
        }
        machine::phase::Phase::Finalizing => Vec::new(),
        _ => Vec::new(),
    }
}

/// Create a default role config for when no forge.toml exists.
fn default_role(name: &str) -> orchestrator::RoleConfig {
    orchestrator::RoleConfig {
        name: name.to_string(),
        description: Some(format!("Default {} agent (mock mode)", name)),
        model: "gpt-4o".to_string(),
        temperature: 0.3,
        max_tokens: 4096,
        system_prompt: Some(format!("You are a helpful {} assistant.", name)),
        tools: Vec::new(),
        context_profile: Some("balanced".to_string()),
        context_priority: Some("normal".to_string()),
    }
}

/// Get the next phase in the pipeline.
fn get_next_phase(current: &machine::phase::Phase) -> machine::phase::Phase {
    match current {
        machine::phase::Phase::Idle => machine::phase::Phase::Planning,
        machine::phase::Phase::Planning => machine::phase::Phase::Designing,
        machine::phase::Phase::Designing => machine::phase::Phase::Implementing,
        machine::phase::Phase::Implementing => machine::phase::Phase::Reviewing,
        machine::phase::Phase::Reviewing => machine::phase::Phase::Testing,
        machine::phase::Phase::Testing => machine::phase::Phase::SecurityScan,
        machine::phase::Phase::SecurityScan => machine::phase::Phase::Finalizing,
        machine::phase::Phase::Finalizing => machine::phase::Phase::Completed,
        machine::phase::Phase::Researching => machine::phase::Phase::Designing,
        machine::phase::Phase::Fixing => machine::phase::Phase::Implementing,
        _ => machine::phase::Phase::Completed,
    }
}

/// Extract review results from agent task results.
///
/// Converts the most recent reviewer/security/tester output into a
/// `ReviewResult` that the gate system can evaluate. Parses the agent's
/// text output for PASS/FAIL keywords.
fn extract_review_results(results: &[orchestrator::TaskResult]) -> Vec<machine::gate::ReviewResult> {
    results
        .iter()
        .rev()
        .find(|r| matches!(r.role.as_str(), "reviewer" | "security" | "tester"))
        .map(|r| {
            let content_lower = r.content.to_lowercase();
            let passed = !content_lower.contains("fail");

            let has_critical = content_lower.contains("critical")
                && !content_lower.contains("0 critical")
                && !content_lower.contains("no critical");

            let comments = if has_critical {
                vec![machine::gate::ReviewComment {
                    severity: machine::gate::Severity::Critical,
                    file: None,
                    line: None,
                    message: "Critical finding detected".to_string(),
                }]
            } else {
                Vec::new()
            };

            let coverage = if r.role == "tester" {
                if content_lower.contains("coverage") {
                    Some(0.85)
                } else {
                    Some(0.5)
                }
            } else {
                None
            };

            machine::gate::ReviewResult {
                agent: r.agent_id.clone(),
                passed,
                comments,
                coverage,
            }
        })
        .into_iter()
        .collect()
}

/// Consolidate feedback from failed gates into a single message for the coder.
fn consolidate_feedback(results: &[orchestrator::TaskResult]) -> String {
    let review_feedback: Vec<&str> = results
        .iter()
        .rev()
        .filter(|r| matches!(r.role.as_str(), "reviewer" | "security" | "tester"))
        .map(|r| r.content.as_str())
        .collect();

    if review_feedback.is_empty() {
        "Previous iteration had issues. Please review and fix.".to_string()
    } else {
        format!(
            "Previous review feedback:\n{}",
            review_feedback.join("\n---\n")
        )
    }
}

// ─── Tests ────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_event_bus_basic() {
        let bus = EventBus::new();
        let mut rx = bus.subscribe();
        bus.publish(
            praxis_shared::protocol::MessageKind::SessionHeartbeat,
            "test",
        );
        let event = tokio::time::timeout(std::time::Duration::from_secs(1), rx.recv())
            .await
            .expect("timeout")
            .expect("recv error");
        assert_eq!(event.source, "test");
    }

    #[tokio::test]
    async fn test_event_bus_multiple_subscribers() {
        let bus = EventBus::new();
        let mut rx1 = bus.subscribe();
        let mut rx2 = bus.subscribe();
        bus.publish(
            praxis_shared::protocol::MessageKind::SessionHeartbeat,
            "test",
        );
        let _ = tokio::time::timeout(std::time::Duration::from_secs(1), rx1.recv()).await.expect("timeout").expect("recv error");
        let _ = tokio::time::timeout(std::time::Duration::from_secs(1), rx2.recv()).await.expect("timeout").expect("recv error");
    }

    #[tokio::test]
    async fn test_echo_agent() {
        let (actor_ref, _handle) = ractor::Actor::spawn(
            Some("test-echo".to_string()),
            actor::EchoAgent,
            "test-echo".to_string(),
        )
        .await
        .expect("Failed to spawn EchoAgent");

        let response = actor::echo(&actor_ref, "hello").await.expect("echo failed");
        assert!(response.contains("hello"));

        let pong = actor::ping(&actor_ref).await.expect("ping failed");
        assert_eq!(pong, "pong");

        let stats = actor::get_stats(&actor_ref).await.expect("stats failed");
        assert_eq!(stats.messages_processed, 3);
        assert_eq!(stats.agent_id, "test-echo");

        actor_ref.get_cell().stop(None);
    }

    #[tokio::test]
    async fn test_supervisor() {
        let supervisor = actor::Supervisor::spawn().await.expect("Failed to spawn Supervisor");

        let handle = actor::spawn_echo(&supervisor, "agent-1").await.expect("spawn failed");
        assert_eq!(handle.name, "agent-1");

        let handle2 = actor::spawn_echo(&supervisor, "agent-2").await.expect("spawn failed");
        assert_eq!(handle2.name, "agent-2");

        let response = actor::supervisor_echo_to(&supervisor, "agent-1", "test msg").await.expect("echo failed");
        assert!(response.contains("test msg"));

        let children = actor::list_children(&supervisor).await.expect("list failed");
        assert_eq!(children.len(), 2);

        let _ = actor::shutdown_all(&supervisor).await;
    }

    #[tokio::test]
    async fn test_core_runtime() {
        let runtime = CoreRuntime::new().await.expect("Failed to create runtime");

        let handle = runtime.spawn_echo_agent("test-agent").await.expect("spawn failed");
        assert_eq!(handle.name, "test-agent");

        let response = runtime.echo_to("test-agent", "hello runtime").await.expect("echo failed");
        assert!(response.contains("hello runtime"));

        let agents = runtime.list_agents().await.expect("list failed");
        assert_eq!(agents.len(), 1);

        let _ = runtime.shutdown().await;
    }

    #[tokio::test]
    async fn test_run_goal_completes_with_mock_agents() {
        let mut runtime = CoreRuntime::new().await.expect("Failed to create runtime");

        let result = runtime
            .run_goal("Create a hello world program", None, None)
            .await
            .expect("run_goal failed");

        assert!(!result.agent_results.is_empty(), "should have executed agents");
        assert!(result.passed, "goal should pass with mock agents (all gates pass)");

        let _ = runtime.shutdown().await;
    }

    #[tokio::test]
    async fn test_run_goal_respects_iteration_limit() {
        let mut runtime = CoreRuntime::new().await.expect("Failed to create runtime");
        runtime.loop_controller.limits.max_iterations_per_goal = 3;

        let result = runtime
            .run_goal("Limited goal", None, None)
            .await
            .expect("run_goal failed");

        assert!(
            runtime.loop_controller.iteration <= 3,
            "should not exceed max iterations: got {}",
            runtime.loop_controller.iteration
        );

        let _ = runtime.shutdown().await;
    }

    #[test]
    fn test_extract_review_results_pass() {
        let results = vec![orchestrator::TaskResult::success(
            "t1", "reviewer", "reviewer",
            "Review: PASS\nNo issues found", 100,
        )];
        let review = extract_review_results(&results);
        assert_eq!(review.len(), 1);
        assert!(review[0].passed, "should pass when content says PASS");
    }

    #[test]
    fn test_extract_review_results_fail() {
        let results = vec![orchestrator::TaskResult::success(
            "t1", "reviewer", "reviewer",
            "Review: FAIL\nCritical issue found", 100,
        )];
        let review = extract_review_results(&results);
        assert_eq!(review.len(), 1);
        assert!(!review[0].passed, "should fail when content says FAIL");
        assert!(!review[0].comments.is_empty(), "should have critical comments");
    }

    #[test]
    fn test_consolidate_feedback() {
        let results = vec![
            orchestrator::TaskResult::success("t1", "coder", "coder", "code here", 100),
            orchestrator::TaskResult::success("t2", "reviewer", "reviewer", "Fix the error handling", 100),
        ];
        let feedback = consolidate_feedback(&results);
        assert!(feedback.contains("Fix the error handling"), "should include reviewer feedback");
    }

    #[tokio::test]
    async fn test_checkpoint_saved_and_loaded() {
        let store = praxis_persistence::SqliteEventStore::in_memory()
            .expect("Failed to create store");

        let mut runtime = CoreRuntime::new()
            .await
            .expect("Failed to create runtime")
            .with_event_store(store);

        runtime
            .run_goal("Test checkpointing", None, None)
            .await
            .expect("run_goal failed");

        let session_id = runtime.session_id.expect("session_id should be set");
        let checkpoint = runtime.load_checkpoint(session_id).await;
        assert!(checkpoint.is_some(), "checkpoint should exist after run");

        let checkpoint = checkpoint.unwrap();
        assert_eq!(checkpoint.aggregate_type, "session");
        assert!(
            checkpoint.state.get("goal").is_some(),
            "checkpoint should contain goal"
        );
        assert!(
            checkpoint.state.get("iteration").is_some(),
            "checkpoint should contain iteration"
        );

        let _ = runtime.shutdown().await;
    }

    #[tokio::test]
    async fn test_graceful_shutdown_request() {
        let mut runtime = CoreRuntime::new()
            .await
            .expect("Failed to create runtime");

        let handle = runtime.shutdown_handle();

        // Simulate Ctrl+C before running
        handle.store(true, std::sync::atomic::Ordering::SeqCst);

        let result = runtime
            .run_goal("Should stop immediately", None, None)
            .await
            .expect("run_goal failed");

        // Should have stopped early due to shutdown request
        assert!(
            runtime.loop_controller.iteration <= 1,
            "should stop on first iteration check"
        );

        let _ = runtime.shutdown().await;
    }

    #[tokio::test]
    async fn test_resume_goal_no_checkpoint() {
        let store = praxis_persistence::SqliteEventStore::in_memory()
            .expect("Failed to create store");

        let mut runtime = CoreRuntime::new()
            .await
            .expect("Failed to create runtime")
            .with_event_store(store);

        let fake_session_id = uuid::Uuid::new_v4();
        let result = runtime
            .resume_goal(fake_session_id, None, None)
            .await
            .expect("resume_goal failed");

        assert!(result.is_none(), "should return None when no checkpoint exists");

        let _ = runtime.shutdown().await;
    }

    #[tokio::test]
    async fn test_resume_goal_from_checkpoint() {
        let store = praxis_persistence::SqliteEventStore::in_memory()
            .expect("Failed to create store");

        let mut runtime = CoreRuntime::new()
            .await
            .expect("Failed to create runtime")
            .with_event_store(store);

        // Run a goal to create a checkpoint
        runtime
            .run_goal("Test resume", None, None)
            .await
            .expect("run_goal failed");

        let session_id = runtime.session_id.expect("session_id should be set");

        // Reset runtime state
        runtime.loop_controller = crate::r#loop::LoopController::new();

        // Resume from the checkpoint
        let result = runtime
            .resume_goal(session_id, None, None)
            .await
            .expect("resume_goal failed");

        assert!(result.is_some(), "should resume from checkpoint");
        let result = result.unwrap();
        assert_eq!(result.goal, "Test resume");

        let _ = runtime.shutdown().await;
    }
}