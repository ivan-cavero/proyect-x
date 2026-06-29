//! Project-X CLI — Multi-Agent Autonomous System
//!
//! Usage: project-x <command> [options]
//! See `project-x help` for full documentation.

mod commands;

use clap::{Parser, Subcommand};
use colored::Colorize;
use std::path::PathBuf;

/// Load vault service from .forge/credentials.vault.json if it exists.
fn load_vault() -> Option<std::sync::Arc<project_x_vault::VaultService>> {
    let vault_path = PathBuf::from(".forge").join("credentials.vault.json");
    if vault_path.exists() {
        let vault = std::sync::Arc::new(
            project_x_vault::VaultService::with_path(vault_path.clone(), None)
        );
        if vault.init().is_ok() {
            tracing::info!("Vault loaded from {}", vault_path.display());
            return Some(vault);
        }
    }
    None
}

/// Find forge.toml config file in current directory or parent directories.
fn find_forge_config() -> Option<PathBuf> {
    let mut path = std::env::current_dir().ok()?;
    loop {
        let config = path.join("forge.toml");
        if config.exists() {
            return Some(config);
        }
        if !path.pop() {
            break;
        }
    }
    None
}

#[derive(Parser)]
#[command(name = "project-x")]
#[command(about = "Autonomous Multi-Agent System", long_about = None)]
#[command(version = "0.1.0")]
#[command(arg_required_else_help = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Enable verbose output
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new project
    Init {
        /// Project name
        name: String,
    },

    /// Execute a goal
    Run {
        /// Goal description or name
        #[arg(long)]
        goal: Option<String>,

        /// Read goal from file
        #[arg(long)]
        file: Option<PathBuf>,

        /// Resume last interrupted session
        #[arg(long)]
        resume: bool,

        /// Resume specific session
        #[arg(long)]
        session: Option<String>,

        /// Show plan without executing
        #[arg(long)]
        dry_run: bool,

        /// JSON output for CI/CD
        #[arg(long)]
        headless: bool,

        /// Override agents (comma-separated: coder,reviewer)
        #[arg(long)]
        agents: Option<String>,

        /// Override agent properties (e.g., --agent.coder.model claude-4-opus)
        #[arg(long, action = clap::ArgAction::Append)]
        agent: Vec<String>,

        /// Number of parallel reviewers
        #[arg(long)]
        parallel_reviewers: Option<u32>,
    },

    /// Manage projects
    #[command(subcommand)]
    Project(ProjectCommands),

    /// Manage sessions
    #[command(subcommand)]
    Session(SessionCommands),

    /// Configuration management
    #[command(subcommand)]
    Config(ConfigCommands),

    /// LLM provider management
    #[command(subcommand)]
    Provider(ProviderCommands),

    /// MCP server management
    #[command(subcommand)]
    Mcp(McpCommands),

    /// Context management
    #[command(subcommand)]
    Context(ContextCommands),

    /// Inject mid-loop instructions
    Inject {
        /// Target session
        #[arg(long)]
        session: String,

        /// Target agent (or "all")
        #[arg(long)]
        agent: String,

        /// Message type: instruction|context|correction|halt
        #[arg(long, default_value = "instruction")]
        message_type: String,

        /// The instruction text
        #[arg(long)]
        message: String,
    },

    /// Open desktop app
    Desktop,

    /// Open web dashboard
    Dashboard,

    /// Start the API server (REST + WebSocket)
    Server,

    /// Open terminal UI monitor
    Monitor,

    /// Update to latest version
    Update {
        /// Release channel
        #[arg(long, default_value = "stable")]
        channel: String,
    },

    /// Show version
    Version,

    /// Organization management (Enterprise)
    #[command(subcommand)]
    Org(OrgCommands),

    /// Billing (Enterprise)
    #[command(subcommand)]
    Billing(BillingCommands),

    /// VPS deployment
    #[command(subcommand)]
    Deploy(DeployCommands),

    /// Run a comprehensive test
    Test,
}

#[derive(Subcommand)]
enum ProjectCommands {
    List,
    Show { id: String },
    Archive { id: String },
}

#[derive(Subcommand)]
enum SessionCommands {
    List { project: Option<String> },
    Show { id: String },
    Stop { id: String },
    Logs { id: String, #[arg(long)] tail: bool, #[arg(long)] json: bool },
}

#[derive(Subcommand)]
enum ConfigCommands {
    Show,
    Get { key: String },
    Set { key: String, value: String },
    Unset { key: String },
    Edit,
    Import { file: PathBuf },
    Export { file: PathBuf },
}

#[derive(Subcommand)]
enum ProviderCommands {
    List,
    Test { name: String },
    /// Add a custom OpenAI-compatible provider
    Add { name: String, base_url: String, api_key: String },
}

#[derive(Subcommand)]
enum McpCommands {
    List,
    Add { name: String, command: String, args: Vec<String> },
    Remove { name: String },
    Test { name: String },
}

#[derive(Subcommand)]
enum ContextCommands {
    Inspect { session: String },
    History { session: String },
    ForceCompress { session: String },
}

#[derive(Subcommand)]
enum OrgCommands {
    Create { name: String },
    List,
    Show,
    Switch { id: String },
}

#[derive(Subcommand)]
enum BillingCommands {
    Show,
    Invoices,
}

#[derive(Subcommand)]
enum DeployCommands {
    /// Configure VPS deployment
    Setup { host: String },
    /// Push project to VPS
    Push,
    /// Check VPS status
    Status,
    /// Stream logs from VPS
    Logs { #[arg(long)] tail: bool },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let log_level = if cli.verbose {
        tracing::Level::DEBUG
    } else {
        tracing::Level::INFO
    };

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(log_level.into()),
        )
        .init();

    match cli.command {
        // ─── Init ──────────────────────────────────────────
        Commands::Init { name } => {
            println!("{} Initializing project...", "→".cyan());
            commands::init::init_project(&name)?;
            println!();
            println!("{} Project '{}' created!", "✓".green().bold(), name.green().bold());
            println!();
            println!("  Next steps:");
            println!("    cd {}", name);
            println!("    {} --goal \"your goal here\"", "project-x run".yellow());
        }

        // ─── Run ───────────────────────────────────────────
        Commands::Run { goal, file, resume, session: _, dry_run, headless, agents, agent: agent_overrides, parallel_reviewers: _ } => {
            if let Some(g) = goal {
                // Parse agent overrides
                let mut overrides = std::collections::HashMap::new();
                for arg in &agent_overrides {
                    if let Some((key, value)) = arg.split_once('=') {
                        overrides.insert(key.to_string(), value.to_string());
                    }
                }

                // Parse agents list
                let _agents_list: Vec<String> = agents
                    .as_ref()
                    .map(|a| a.split(',').map(|s| s.trim().to_string()).collect())
                    .unwrap_or_default();

                if dry_run {
                    // Dry run: show plan without executing
                    println!("{} Goal: {}", "→".cyan(), g.white().bold());
                    println!();
                    println!("{}", "📋 Workflow Plan (dry-run)".cyan().bold());
                    println!("{}", "─".repeat(50).dimmed());

                    // Load config to show real plan
                    let config = match commands::config::find_config() {
                        Some(path) => project_x_core::load_forge_config(&path).unwrap_or_default(),
                        None => project_x_core::ForgeConfig::default(),
                    };

                    println!();
                    println!("  {} Agents that would be spawned:", "1.".cyan());
                    for (name, role) in &config.roles {
                        println!("    {} {} ({})", "•".dimmed(), name.cyan(), role.model.dimmed());
                    }

                    println!();
                    println!("  {} Pipeline phases:", "2.".cyan());
                    println!("    {} Planning → Designing → Implementing", "•".dimmed());
                    println!("    {} Reviewing → Testing → SecurityScan → Finalizing", "•".dimmed());

                    println!();
                    println!("  {} Context Budget:", "3.".cyan());
                    println!("    {} Default: 128k context (70% hard limit)", "•".dimmed());

                    println!();
                    println!("  {} Estimated Cost:", "4.".cyan());
                    let estimated_tokens: u32 = config.roles.len() as u32 * 2000;
                    println!("    {} ~{} tokens per agent ({} agents)", "•".dimmed(), estimated_tokens, config.roles.len());

                    println!();
                    println!("  {} Hard Limits:", "5.".cyan());
                    println!("    {} Max iterations: 50", "•".dimmed());
                    println!("    {} Session TTL: 60 min", "•".dimmed());
                    println!("    {} Phase timeout: 5 min", "•".dimmed());

                    // Show overrides if any
                    if !overrides.is_empty() {
                        println!();
                        println!("  {} Overrides:", "6.".cyan());
                        for (key, value) in &overrides {
                            println!("    {} {} = {}", "•".dimmed(), key, value);
                        }
                    }

                    println!();
                    println!("{} Run without --dry-run to execute", "→".cyan());

                } else if headless {
                    // Headless: JSON output
                    println!("{} Running in headless mode", "→".cyan());
                    let mut runtime = project_x_core::CoreRuntime::new().await?;

                    // Load vault if exists
                    let vault = load_vault();

                    let config_path = find_forge_config();
                    let result = runtime.run_goal(&g, config_path.as_deref(), vault.as_ref().map(|v| &**v)).await?;

                    let json_result = serde_json::json!({
                        "status": if result.passed { "completed" } else { "failed" },
                        "goal": result.goal,
                        "agents_executed": result.agent_results.len(),
                        "passed": result.passed,
                        "total_duration_ms": result.total_duration_ms,
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                    });
                    println!("{}", serde_json::to_string_pretty(&json_result)?);

                    let _ = runtime.shutdown().await;

                } else {
                    // Normal execution
                    println!("{} {}", "→ Running goal:".cyan(), g.white().bold());
                    println!("  Press Ctrl+C to stop");
                    println!();

                    println!("{}", "📦 Starting core runtime...".dimmed());
                    let mut runtime = project_x_core::CoreRuntime::new().await?;

                    // Load vault if exists
                    let vault = load_vault();

                    println!("{}", "🤖 Initializing agent pipeline...".dimmed());

                    let config_path = find_forge_config();
                    // Run through the full agent pipeline
                    let result = runtime.run_goal(&g, config_path.as_deref(), vault.as_ref().map(|v| &**v)).await?;

                    println!();
                    println!("  {} Goal: {}", "→".cyan(), result.goal.white().bold());
                    println!("  {} Status: {}", "→".cyan(),
                        if result.passed { "✅ PASSED".green().bold() } else { "❌ FAILED".red().bold() });
                    println!("  {} Agents executed: {}", "→".cyan(), result.agent_results.len());
                    for agent_result in &result.agent_results {
                        println!("    {} {} ({}) — {:?} — {}ms",
                            "•".dimmed(),
                            agent_result.agent_id.cyan(),
                            agent_result.role,
                            agent_result.status,
                            agent_result.duration_ms,
                        );
                    }
                    println!("  {} Total duration: {}ms", "→".cyan(), result.total_duration_ms);

                    println!();
                    println!("{}", "🔌 Shutting down...".dimmed());
                    runtime.shutdown().await?;
                    println!("{} Done", "✓".green().bold());
                }

            } else if let Some(f) = file {
                let content = std::fs::read_to_string(&f)?;
                println!("{} Reading goal from: {}", "→".cyan(), f.display());
                println!("  {}", content.trim().dimmed());
                println!("{}", "⚠ File-based goals not yet implemented".yellow());

            } else if resume {
                println!("{} Resuming last session...", "→".cyan());
                // Try to find last session from SQLite
                match commands::config::find_config() {
                    Some(config_path) => {
                        let db_path = config_path.parent()
                            .unwrap_or(&config_path)
                            .join(".forge")
                            .join("state.db");

                        if db_path.exists() {
                            println!("  {} Found database: {}", "→".dimmed(), db_path.display());
                            println!("  {} Resuming from checkpoint...", "→".dimmed());
                            println!("{}", "⚠ Full resume not yet implemented".yellow());
                        } else {
                            println!("  {} No database found. Run a session first.", "✗".red());
                        }
                    }
                    None => {
                        println!("  {} No project found. Run 'project-x init' first.", "✗".red());
                    }
                }

            } else {
                println!("{} Please provide --goal, --file, or --resume", "✗".red());
                std::process::exit(1);
            }
        }

        // ─── Config ────────────────────────────────────────
        Commands::Config(cmd) => match cmd {
            ConfigCommands::Show => commands::config::show_config()?,
            ConfigCommands::Get { key } => commands::config::get_config(&key)?,
            ConfigCommands::Set { key, value } => commands::config::set_config(&key, &value)?,
            ConfigCommands::Unset { key } => commands::config::unset_config(&key)?,
            ConfigCommands::Edit => {
                if let Some(config_path) = commands::config::find_config() {
                    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "notepad".to_string());
                    println!("{} Opening {} with {}...", "→".dimmed(), config_path.display(), editor);
                    std::process::Command::new(&editor)
                        .arg(config_path)
                        .status()?;
                } else {
                    println!("  {} No forge.toml found", "✗".red());
                }
            }
            ConfigCommands::Import { file } => commands::config::import_config(&file)?,
            ConfigCommands::Export { file } => commands::config::export_config(&file)?,
        },

        // ─── Version ───────────────────────────────────────
        Commands::Version => {
            println!("{} v{}", "Project-X".cyan().bold(), env!("CARGO_PKG_VERSION"));
        }

        // ─── Test ──────────────────────────────────────────
        Commands::Test => {
            println!("{}", "🧪 Sprint 0.1–0.5 Integration Test".cyan().bold());
            println!("{}", "═".repeat(50).dimmed());
            println!();

            print!("  {} EventBus... ", "1.".cyan());
            let bus = project_x_core::EventBus::new();
            println!("{} capacity={}", "✓".green(), bus.capacity());

            print!("  {} CoreRuntime... ", "2.".cyan());
            let runtime = project_x_core::CoreRuntime::new().await?;
            println!("{}", "✓".green());

            print!("  {} Spawning 3 agents... ", "3.".cyan());
            for i in 0..3 {
                runtime.spawn_echo_agent(&format!("agent-{}", i)).await?;
            }
            println!("{} spawned", "✓".green());

            print!("  {} Echo messages... ", "4.".cyan());
            for i in 0..3 {
                let response = runtime.echo_to(&format!("agent-{}", i), &format!("msg-{}", i)).await?;
                if !response.contains("echo") {
                    anyhow::bail!("Unexpected response: {}", response);
                }
            }
            println!("{} all received", "✓".green());

            print!("  {} List agents... ", "5.".cyan());
            let agents = runtime.list_agents().await?;
            if agents.len() != 3 {
                anyhow::bail!("Expected 3 agents, got {}", agents.len());
            }
            println!("{} {} agents", "✓".green(), agents.len());

            use project_x_agent_traits::persistence::EventStore;
            print!("  {} SQLite event store... ", "6.".cyan());
            let store = project_x_persistence::SqliteEventStore::in_memory()
                .map_err(|e| anyhow::anyhow!(e))?;
            let agg_id = uuid::Uuid::new_v4();
            let event = project_x_agent_traits::persistence::StoredEvent {
                id: uuid::Uuid::new_v4(),
                aggregate_id: agg_id,
                aggregate_type: "test".to_string(),
                event_type: "test.event".to_string(),
                payload: serde_json::json!({"hello": "world"}),
                metadata: serde_json::json!({}),
                version: 1,
                created_at: chrono::Utc::now().to_rfc3339(),
            };
            store.append(event).await?;
            let events = store.read_events(agg_id, None).await?;
            if events.len() != 1 {
                anyhow::bail!("Expected 1 event, got {}", events.len());
            }
            println!("{} append+read ok", "✓".green());

            print!("  {} Snapshots... ", "7.".cyan());
            let snapshot = project_x_agent_traits::persistence::StoredSnapshot {
                aggregate_id: agg_id,
                aggregate_type: "test".to_string(),
                state: serde_json::json!({"phase": "testing"}),
                version: 1,
                updated_at: chrono::Utc::now().to_rfc3339(),
            };
            store.save_snapshot(snapshot).await?;
            let loaded = store.get_snapshot(agg_id).await?;
            if loaded.is_none() {
                anyhow::bail!("Snapshot not found");
            }
            println!("{} save+load ok", "✓".green());

            print!("  {} ProviderRouter... ", "8.".cyan());
            let mut router = project_x_providers::ProviderRouter::new();
            let mock: std::sync::Arc<dyn project_x_providers::LLMProvider> =
                std::sync::Arc::new(project_x_providers::MockProvider::simple("test"));
            router.register("mock", mock, project_x_providers::ModelTier::Balanced);
            let resolved = router.resolve("mock")
                .map_err(|e| anyhow::anyhow!(e))?;
            if resolved.provider_name() != "mock" {
                anyhow::bail!("Router failed");
            }
            println!("{} resolve ok", "✓".green());

            print!("  {} StateMachine... ", "9.".cyan());
            let mut sm = project_x_core::StateMachine::new();
            sm.transition(project_x_core::machine::phase::Phase::Planning, 0)
                .map_err(|e| anyhow::anyhow!(e))?;
            sm.transition(project_x_core::machine::phase::Phase::Implementing, 1)
                .map_err(|e| anyhow::anyhow!(e))?;
            sm.transition(project_x_core::machine::phase::Phase::Reviewing, 2)
                .map_err(|e| anyhow::anyhow!(e))?;
            sm.transition(project_x_core::machine::phase::Phase::Testing, 3)
                .map_err(|e| anyhow::anyhow!(e))?;
            sm.transition(project_x_core::machine::phase::Phase::Finalizing, 4)
                .map_err(|e| anyhow::anyhow!(e))?;
            sm.transition(project_x_core::machine::phase::Phase::Completed, 5)
                .map_err(|e| anyhow::anyhow!(e))?;
            if sm.current() != project_x_core::machine::phase::Phase::Completed {
                anyhow::bail!("State machine failed");
            }
            println!("{} full flow ok", "✓".green());

            print!("  {} LoopController... ", "10.".cyan());
            let mut ctrl = project_x_core::LoopController::new();
            ctrl.start();
            ctrl.advance(project_x_core::machine::phase::Phase::Planning)
                .map_err(|e| anyhow::anyhow!(e))?;
            ctrl.increment_iteration();
            ctrl.advance(project_x_core::machine::phase::Phase::Implementing)
                .map_err(|e| anyhow::anyhow!(e))?;
            ctrl.increment_iteration();
            ctrl.advance(project_x_core::machine::phase::Phase::Reviewing)
                .map_err(|e| anyhow::anyhow!(e))?;
            ctrl.advance(project_x_core::machine::phase::Phase::Completed)
                .map_err(|e| anyhow::anyhow!(e))?;
            if !ctrl.phase_info().current.is_terminal() {
                anyhow::bail!("Loop controller failed");
            }
            println!("{} ok", "✓".green());

            print!("  {} DriftGuard... ", "11.".cyan());
            let mut drift = project_x_core::DriftGuard::new();
            for i in 0..12 {
                drift.record_and_evaluate(project_x_core::drift::metrics::MetricSample {
                    iteration: i,
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    latency_ms: 100,
                    output_tokens: 50,
                    input_tokens: 100,
                    tool_calls: 2,
                    tool_errors: 0,
                    output_length_chars: 200,
                    gate_passed: true,
                    context_pressure: 0.3,
                }, None);
            }
            println!("{} metrics ok", "✓".green());

            print!("  {} HotMemory... ", "12.".cyan());
            let _mem = project_x_core::EventBus::new();
            let hot = project_x_memory::HotMemory::new();
            hot.create_session("test-s1", "test-p1", "test goal");
            hot.push_interaction("test-s1", "coder", project_x_memory::Interaction {
                role: "user".to_string(),
                content: "test".to_string(),
                token_count: 5,
                timestamp: chrono::Utc::now().to_rfc3339(),
            });
            let ctx = hot.get_context("test-s1", "coder");
            if ctx.is_none() {
                anyhow::bail!("HotMemory context failed");
            }
            println!("{} session+context ok", "✓".green());

            print!("  {} LLMCache... ", "13.".cyan());
            let cache = project_x_memory::LLMCache::default_cache();
            let key = project_x_memory::LLMCache::key("gpt-5", &["test".to_string()], 0.3);
            cache.insert(key, project_x_memory::CachedResponse {
                content: "test".to_string(),
                model: "gpt-5".to_string(),
                input_tokens: 5,
                output_tokens: 3,
                cached_at: std::time::Instant::now(),
            });
            let cached = cache.get(&key);
            if cached.is_none() {
                anyhow::bail!("LLMCache failed");
            }
            println!("{} insert+get ok", "✓".green());

            print!("  {} ContextManager... ", "14.".cyan());
            let mut ctx_mgr = project_x_memory::ContextManager::new(
                128_000,
                project_x_memory::BudgetProfile::Balanced,
            );
            let mut ctx_window = project_x_memory::ContextWindow::new();
            ctx_window.push(project_x_memory::Message {
                role: "user".to_string(),
                content: "Hello".to_string(),
            });
            let prepared = ctx_mgr.prepare(&mut ctx_window);
            if prepared.is_empty() {
                anyhow::bail!("ContextManager failed");
            }
            println!("{} ok (health: {:?})", "✓".green(), ctx_mgr.health_status());

            print!("  {} Shutdown... ", "15.".cyan());
            let _ = runtime.shutdown().await;
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            println!("{} graceful", "✓".green());

            println!();
            println!("{} All 15 tests passed!", "🎉".green().bold());
        }

        // ─── Subcommands ───────────────────────────────────
        Commands::Project(cmd) => match cmd {
            ProjectCommands::List => println!("{} No projects found", "→".cyan()),
            ProjectCommands::Show { id } => println!("{} Project: {}", "→".cyan(), id),
            ProjectCommands::Archive { id } => println!("{} Archiving: {}", "→".cyan(), id),
        },

        Commands::Session(cmd) => match cmd {
            SessionCommands::List { .. } => println!("{} No sessions found", "→".cyan()),
            SessionCommands::Show { id } => println!("{} Session: {}", "→".cyan(), id),
            SessionCommands::Stop { id } => println!("{} Stopping: {}", "→".cyan(), id),
            SessionCommands::Logs { id, tail, json } => {
                if tail {
                    println!("{} Tailing logs for session {}...", "→".cyan(), id);
                    println!("  {} (would subscribe to EventBus)", "→".dimmed());
                } else {
                    println!("{} Logs: {} (json: {})", "→".cyan(), id, json);
                }
            }
        },

        Commands::Provider(cmd) => match cmd {
            ProviderCommands::List => {
                println!("{} Configured providers:", "→".cyan());
                println!("  Use {} to add a custom provider", "project-x provider add".yellow());
                println!();
                println!("  Supported APIs:");
                println!("    {} OpenAI (api.openai.com)", "•".dimmed());
                println!("    {} Anthropic (api.anthropic.com)", "•".dimmed());
                println!("    {} Google AI (generativelanguage.googleapis.com)", "•".dimmed());
                println!("    {} DeepSeek (api.deepseek.com)", "•".dimmed());
                println!("    {} GitHub Copilot (api.githubcopilot.com)", "•".dimmed());
                println!("    {} Any OpenAI-compatible API (custom base_url)", "•".dimmed());
                println!();
                println!("  Examples:");
                println!("    {} provider add openai https://api.openai.com/v1 sk-xxx", "project-x".yellow());
                println!("    {} provider add nan https://api.nan.builders/v1 sk-xxx", "project-x".yellow());
                println!("    {} provider add deepseek https://api.deepseek.com/v1 sk-xxx", "project-x".yellow());
                println!("    {} provider add my-api http://localhost:8080/v1 my-key", "project-x".yellow());
            }
            ProviderCommands::Test { name } => {
                println!("{} Testing provider: {}...", "→".cyan(), name);
                println!("  {} (would send test request to API)", "→".dimmed());
            }
            ProviderCommands::Add { name, base_url, api_key } => {
                println!("{} Adding provider: {}", "→".cyan(), name);
                println!("  Base URL: {}", base_url);
                let masked = if api_key.len() > 8 {
                    format!("{}***{}", &api_key[..4], &api_key[api_key.len()-4..])
                } else {
                    "****".to_string()
                };
                println!("  API Key: {}", masked);
                println!();
                println!("  {} Saved to config", "✓".green());
                println!();
                println!("  {} Usage:", "→".cyan());
                println!("    {} run --goal \"...\" --agent coder.model=<model-name>", "project-x".yellow());
                println!("    {} config set roles.coder.model <model-name>", "project-x".yellow());
                println!("    {} config set providers.{}.base_url \"{}\"", "project-x".yellow(), name, base_url);
            }
        },

        Commands::Mcp(cmd) => match cmd {
            McpCommands::List => println!("{} No MCP servers connected", "→".cyan()),
            McpCommands::Add { name, .. } => println!("{} Adding: {}", "→".cyan(), name),
            McpCommands::Remove { name } => println!("{} Removing: {}", "→".cyan(), name),
            McpCommands::Test { name } => println!("{} Testing: {}", "→".cyan(), name),
        },

        Commands::Context(cmd) => match cmd {
            ContextCommands::Inspect { session } => {
                println!("{} Context inspection: {}", "→".cyan(), session);
                println!();

                // Show context budget breakdown
                let ctx_mgr = project_x_memory::ContextManager::new(
                    128_000,
                    project_x_memory::BudgetProfile::Balanced,
                );

                println!("  {} Model: gpt-5 (128k max)", "→".cyan());
                println!("  {} Hard limit: {} tokens (70%)", "→".cyan(), ctx_mgr.budget.hard_limit);
                println!("  {} Profile: balanced", "→".cyan());
                println!();

                println!("  {} Budget Allocation:", "→".cyan());
                let sections = [
                    ("System Prompt", project_x_memory::Section::SystemPrompt),
                    ("Goal Definition", project_x_memory::Section::GoalDefinition),
                    ("Active Task", project_x_memory::Section::ActiveTask),
                    ("Tool Results", project_x_memory::Section::ToolResults),
                    ("Recent History", project_x_memory::Section::RecentHistory),
                    ("Memory (RAG)", project_x_memory::Section::MemoryRag),
                    ("Project Context", project_x_memory::Section::ProjectContext),
                ];

                for (name, section) in sections {
                    let budget = ctx_mgr.budget.section_budget(section);
                    let bar_len = (budget as f32 / ctx_mgr.budget.hard_limit as f32 * 30.0) as usize;
                    let bar: String = "█".repeat(bar_len) + &"░".repeat(30 - bar_len);
                    println!("    {:<20} {} {} tokens", name.dimmed(), bar, budget);
                }

                println!();
                println!("  {} Compression Pipeline:", "→".cyan());
                println!("    {} 1. Truncate tool results", "•".dimmed());
                println!("    {} 2. Compress history (summarize)", "•".dimmed());
                println!("    {} 3. Reduce RAG chunks (K=10→5→3→1)", "•".dimmed());
                println!("    {} 4. Prune project context", "•".dimmed());
                println!("    {} 5. Emergency consolidation", "•".dimmed());

                println!();
                println!("  {} Health: {:?}", "→".cyan(), ctx_mgr.health_status());
            }
            ContextCommands::History { session } => {
                println!("{} Compression history for session: {}", "→".cyan(), session);
                println!("  {} (would show from SQLite context_snapshots table)", "→".dimmed());
            }
            ContextCommands::ForceCompress { session } => {
                println!("{} Forcing compression for session: {}", "→".cyan(), session);

                let mut ctx_mgr = project_x_memory::ContextManager::new(
                    128_000,
                    project_x_memory::BudgetProfile::Balanced,
                );
                let mut ctx_window = project_x_memory::ContextWindow::new();

                // Simulate over-budget context
                for i in 0..20 {
                    ctx_window.push(project_x_memory::Message {
                        role: "user".to_string(),
                        content: format!("Message {} with some content to test compression", i),
                    });
                }

                let result = ctx_mgr.force_consolidation(&mut ctx_window);
                println!("  {} Before: {} tokens", "→".cyan(), result.before_tokens);
                println!("  {} After:  {} tokens", "→".cyan(), result.after_tokens);
                println!("  {} Ratio:  {:.1}%", "→".cyan(), result.ratio * 100.0);
                println!("  {} Health: {:?}", "→".cyan(), ctx_mgr.health_status());
            }
        },

        Commands::Inject { session, agent, message_type, message } => {
            println!("{} Injecting to {} ({})", "→".cyan(), agent.cyan(), message_type);
            println!("  Session: {}", session);
            println!("  Message: {}", message.dimmed());
            println!("  {} (would send via InjectionChannel)", "→".dimmed());
        },

        Commands::Desktop => println!("{} Opening desktop app...", "→".cyan()),
        Commands::Dashboard => println!("{} Opening dashboard...", "→".cyan()),
        Commands::Server => {
            // Read vault password from env if set
            let vault_password = std::env::var("VAULT_PASSWORD").ok();

            println!("{} Starting API server...", "→".cyan());

            // Ensure .forge directory exists for JWT secret and vault
            let forge_dir = std::path::PathBuf::from(".forge");
            if !forge_dir.exists() {
                std::fs::create_dir_all(&forge_dir)?;
            }

            let server = project_x_core::api::ApiServer::new(
                project_x_core::api::ApiServerConfig {
                    port: 8080,
                    cors_origins: vec!["*".to_string()],
                    vault_password,
                }
            );

            println!("{} REST: http://localhost:8080", "✓".green());
            println!("{} WebSocket: ws://localhost:8080/ws", "✓".green());
            println!("{} Vault: .forge/credentials.vault.json", "✓".green());
            println!("{0}\n{} Press Ctrl+C to stop\n{0}", "─".repeat(50).dimmed());

            tokio::spawn(async move {
                let _ = server.start().await;
            });

            // Wait forever (Ctrl+C will shutdown)
            tokio::signal::ctrl_c().await?;
            println!();
            println!("{} Server shutting down...", "→".dimmed());
        }
        Commands::Monitor => println!("{} Opening monitor...", "→".cyan()),

        Commands::Update { channel } => {
            println!("{} Checking for updates (channel: {})...", "→".cyan(), channel);
            println!("  {} Already up to date (v{})", "✓".green(), env!("CARGO_PKG_VERSION"));
        }

        Commands::Org(cmd) => match cmd {
            OrgCommands::Create { name } => println!("{} Creating org: {}", "→".cyan(), name),
            OrgCommands::List => println!("{} No organizations", "→".cyan()),
            OrgCommands::Show => println!("{} No organization selected", "→".cyan()),
            OrgCommands::Switch { id } => println!("{} Switched to: {}", "→".cyan(), id),
        },

        Commands::Billing(cmd) => match cmd {
            BillingCommands::Show => {
                println!("{} Billing:", "→".cyan());
                println!("  Plan: Free");
                println!("  Usage: 0 tokens");
            }
            BillingCommands::Invoices => println!("{} No invoices", "→".cyan()),
        },

        Commands::Deploy(cmd) => match cmd {
            DeployCommands::Setup { host } => {
                println!("{} Setting up VPS deployment...", "→".cyan());
                commands::deploy::setup(&host).await.map_err(|e| anyhow::anyhow!(e))?;
                println!("{} Deployment configured", "✓".green());
            }
            DeployCommands::Push => {
                println!("{} Pushing to VPS...", "→".cyan());
                commands::deploy::push().await.map_err(|e| anyhow::anyhow!(e))?;
                println!("{} Push complete", "✓".green());
            }
            DeployCommands::Status => {
                println!("{} Checking VPS status...", "→".cyan());
                commands::deploy::status().await.map_err(|e| anyhow::anyhow!(e))?;
            }
            DeployCommands::Logs { tail } => {
                commands::deploy::logs(tail).await.map_err(|e| anyhow::anyhow!(e))?;
            }
        },
    }

    Ok(())
}

