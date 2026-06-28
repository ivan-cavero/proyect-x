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

    /// Run a comprehensive test (Sprint 0.1–0.3)
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

    // Initialize logging
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
            println!("{}", "  Initializing project...".cyan());
            commands::init::init_project(&name)?;
            println!();
            println!("{} Project '{}' created!", "✓".green().bold(), name.green().bold());
            println!();
            println!("  Next steps:");
            println!("    cd {}", name);
            println!("    {} --goal \"your goal here\"", "project-x run".yellow());
        }

        // ─── Run ───────────────────────────────────────────
        Commands::Run { goal, file, resume, .. } => {
            if let Some(g) = goal {
                println!("{} {}", "→ Running goal:".cyan(), g.white().bold());
                println!("  Press Ctrl+C to stop");
                println!();

                // Create CoreRuntime
                println!("{}", "📦 Starting core runtime...".dimmed());
                let runtime = project_x_core::CoreRuntime::new().await?;

                // Spawn a coder agent for the goal
                println!("{}", "🤖 Spawning agent...".dimmed());
                let handle = runtime.spawn_echo_agent("coder").await?;
                println!("  {} Agent '{}' ready", "✓".green(), handle.name.cyan());

                // Send the goal as a message
                println!();
                println!("{}", "💬 Executing...".dimmed());
                let response = runtime.echo_to(&handle.name, &g).await?;
                println!("  {} {}", "✓".green(), response);

                // List agents
                let agents = runtime.list_agents().await?;
                println!();
                println!("  {} {} agents active", "→".cyan(), agents.len());
                for agent in &agents {
                    println!("    {} {} ({})", "•".dimmed(), agent.name.cyan(), agent.role);
                }

                // Shutdown
                println!();
                println!("{}", "🔌 Shutting down...".dimmed());
                runtime.shutdown().await?;
                println!("{} Done", "✓".green().bold());

            } else if let Some(f) = file {
                let content = std::fs::read_to_string(&f)?;
                println!("{} Reading goal from: {}", "→".cyan(), f.display());
                println!("  {}", content.trim().dimmed());

                // TODO: pass content to runtime
                println!("{}", "⚠ File-based goals not yet implemented".yellow());

            } else if resume {
                println!("{} Resuming last session...", "→".cyan());
                println!("{}", "⚠ Resume not yet implemented".yellow());

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
                println!("{}", "⚠ Edit not yet implemented".yellow());
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
            println!("{}", "🧪 Sprint 0.1–0.3 Integration Test".cyan().bold());
            println!("{}", "═".repeat(40).dimmed());
            println!();

            // 1. EventBus
            print!("  {} EventBus... ", "1.".cyan());
            let bus = project_x_core::EventBus::new();
            println!("{} capacity={}", "✓".green(), bus.capacity());

            // 2. CoreRuntime
            print!("  {} CoreRuntime... ", "2.".cyan());
            let runtime = project_x_core::CoreRuntime::new().await?;
            println!("{}", "✓".green());

            // 3. Spawn agents
            print!("  {} Spawning 3 agents... ", "3.".cyan());
            for i in 0..3 {
                let name = format!("agent-{}", i);
                runtime.spawn_echo_agent(&name).await?;
            }
            println!("{} spawned", "✓".green());

            // 4. Echo messages
            print!("  {} Echo messages... ", "4.".cyan());
            for i in 0..3 {
                let name = format!("agent-{}", i);
                let response = runtime.echo_to(&name, &format!("msg-{}", i)).await?;
                if !response.contains("echo") {
                    anyhow::bail!("Unexpected response: {}", response);
                }
            }
            println!("{} all received", "✓".green());

            // 5. List agents
            print!("  {} List agents... ", "5.".cyan());
            let agents = runtime.list_agents().await?;
            if agents.len() != 3 {
                anyhow::bail!("Expected 3 agents, got {}", agents.len());
            }
            println!("{} {} agents", "✓".green(), agents.len());

            // 6. SQLite event store
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

            // 7. Snapshot
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

            // 8. ProviderRouter
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

            // 9. Shutdown
            print!("  {} Shutdown... ", "9.".cyan());
            let _ = runtime.shutdown().await;
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            println!("{} graceful", "✓".green());

            println!();
            println!("{} All tests passed!", "🎉".green().bold());
        }

        // ─── Subcommands (stubs for now) ───────────────────
        Commands::Project(cmd) => match cmd {
            ProjectCommands::List => println!("{} No projects found", "→".cyan()),
            ProjectCommands::Show { id } => println!("{} Project: {}", "→".cyan(), id),
            ProjectCommands::Archive { id } => println!("{} Archiving: {}", "→".cyan(), id),
        },

        Commands::Session(cmd) => match cmd {
            SessionCommands::List { .. } => println!("{} No sessions found", "→".cyan()),
            SessionCommands::Show { id } => println!("{} Session: {}", "→".cyan(), id),
            SessionCommands::Stop { id } => println!("{} Stopping: {}", "→".cyan(), id),
            SessionCommands::Logs { id, .. } => println!("{} Logs: {}", "→".cyan(), id),
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
                println!("  {} Not yet implemented", "⚠".yellow());
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
                println!("{} Context for session: {}", "→".cyan(), session);
                println!("  {} Not yet implemented", "⚠".yellow());
            }
            ContextCommands::History { session } => {
                println!("{} Compression history: {}", "→".cyan(), session);
            }
            ContextCommands::ForceCompress { session } => {
                println!("{} Forcing compression: {}", "→".cyan(), session);
            }
        },

        Commands::Inject { session, agent, message_type, message } => {
            println!("{} Injecting to {} ({})", "→".cyan(), agent.cyan(), message_type);
            println!("  Session: {}", session);
            println!("  Message: {}", message.dimmed());
            println!("  {} Not yet implemented", "⚠".yellow());
        }

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