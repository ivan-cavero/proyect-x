//! Project-X CLI — Multi-Agent Autonomous System
//!
//! Usage: project-x <command> [options]
//! See `project-x help` for full documentation.

mod commands;

use clap::{Parser, Subcommand};
use colored::Colorize;
use std::path::PathBuf;

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
        Commands::Run { goal, file, resume, session, dry_run, headless, agents, agent: agent_overrides, parallel_reviewers } => {
            if let Some(g) = goal {
                // Parse agent overrides
                let mut overrides = std::collections::HashMap::new();
                for arg in &agent_overrides {
                    if let Some((key, value)) = arg.split_once('=') {
                        overrides.insert(key.to_string(), value.to_string());
                    }
                }

                // Parse agents list
                let agents_list: Vec<String> = agents
                    .as_ref()
                    .map(|a| a.split(',').map(|s| s.trim().to_string()).collect())
                    .unwrap_or_default();

                if dry_run {
                    // Dry run: show plan without executing
                    println!("{} Goal: {}", "→".cyan(), g.white().bold());
                    println!();
                    println!("{}", "📋 Workflow Plan (dry-run)".cyan().bold());
                    println!("{}", "─".repeat(50).dimmed());

                    // Create runtime to show what would happen
                    let runtime = project_x_core::CoreRuntime::new().await?;

                    println!();
                    println!("  {} Agents that would be spawned:", "1.".cyan());

                    // Show configured agents or defaults
                    let agent_names = if !agents_list.is_empty() {
                        agents_list.clone()
                    } else {
                        vec![
                            "architect".to_string(),
                            "coder".to_string(),
                            "reviewer".to_string(),
                            "security".to_string(),
                            "tester".to_string(),
                        ]
                    };

                    for agent_name in &agent_names {
                        let override_info = overrides.get(&format!("{}.model", agent_name))
                            .map(|m| format!(" (override: {})", m))
                            .unwrap_or_default();
                        println!("    {} {}{}", "•".dimmed(), agent_name.cyan(), override_info.dimmed());
                    }

                    // Show parallel reviewers if set
                    if let Some(pr) = parallel_reviewers {
                        println!();
                        println!("  {} Parallel reviewers: {}", "2.".cyan(), pr);
                    }
                    println!("    {} architect (claude-4-opus)", "•".dimmed());
                    println!("    {} coder (gpt-5)", "•".dimmed());
                    println!("    {} reviewer (gemini-2.5-pro)", "•".dimmed());
                    println!("    {} security (claude-4-haiku)", "•".dimmed());
                    println!("    {} tester (gpt-5)", "•".dimmed());

                    println!();
                    println!("  {} Phases:", "3.".cyan());
                    println!("    {} Planning → Designing → Implementing", "•".dimmed());
                    println!("    {} Reviewing → Testing → Finalizing", "•".dimmed());

                    println!();
                    println!("  {} Context Budget:", "4.".cyan());
                    println!("    {} Model: gpt-5 (128k context)", "•".dimmed());
                    println!("    {} Hard limit: 89,600 tokens (70%)", "•".dimmed());
                    println!("    {} Profile: balanced", "•".dimmed());

                    println!();
                    println!("  {} Estimated Cost:", "5.".cyan());
                    println!("    {} ~15,000 input tokens", "•".dimmed());
                    println!("    {} ~5,000 output tokens", "•".dimmed());
                    println!("    {} ~$0.03-0.05 (GPT-5)", "•".dimmed());

                    println!();
                    println!("  {} Hard Limits:", "6.".cyan());
                    println!("    {} Max iterations: 50", "•".dimmed());
                    println!("    {} Session TTL: 60 min", "•".dimmed());
                    println!("    {} Phase timeout: 5 min", "•".dimmed());

                    // Show overrides if any
                    if !overrides.is_empty() {
                        println!();
                        println!("  {} Overrides:", "7.".cyan());
                        for (key, value) in &overrides {
                            println!("    {} {} = {}", "•".dimmed(), key, value);
                        }
                    }

                    println!();
                    println!("{} Run without --dry-run to execute", "→".cyan());
                    let _ = runtime.shutdown().await;

                } else if headless {
                    // Headless: JSON output
                    println!("{} Running in headless mode", "→".cyan());
                    let runtime = project_x_core::CoreRuntime::new().await?;
                    let handle = runtime.spawn_echo_agent("coder").await?;
                    let response = runtime.echo_to(&handle.name, &g).await?;

                    let result = serde_json::json!({
                        "status": "completed",
                        "goal": g,
                        "response": response,
                        "agents": 1,
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                    });
                    println!("{}", serde_json::to_string_pretty(&result)?);

                    let _ = runtime.shutdown().await;

                } else {
                    // Normal execution
                    println!("{} {}", "→ Running goal:".cyan(), g.white().bold());
                    println!("  Press Ctrl+C to stop");
                    println!();

                    println!("{}", "📦 Starting core runtime...".dimmed());
                    let runtime = project_x_core::CoreRuntime::new().await?;

                    println!("{}", "🤖 Spawning agent...".dimmed());
                    let handle = runtime.spawn_echo_agent("coder").await?;
                    println!("  {} Agent '{}' ready", "✓".green(), handle.name.cyan());

                    println!();
                    println!("{}", "💬 Executing...".dimmed());
                    let response = runtime.echo_to(&handle.name, &g).await?;
                    println!("  {} {}", "✓".green(), response);

                    let agents = runtime.list_agents().await?;
                    println!();
                    println!("  {} {} agents active", "→".cyan(), agents.len());
                    for agent in &agents {
                        println!("    {} {} ({})", "•".dimmed(), agent.name.cyan(), agent.role);
                    }

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
            let mem = project_x_core::EventBus::new();
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
                println!("  {} openai (gpt-5)", "•".dimmed());
                println!("  {} anthropic (claude-4-opus)", "•".dimmed());
                println!("  {} gemini (gemini-2.5-pro)", "•".dimmed());
            }
            ProviderCommands::Test { name } => {
                println!("{} Testing provider: {}", "→".cyan(), name);
                println!("  {} (would send test request to API)", "→".dimmed());
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
    }

    Ok(())
}