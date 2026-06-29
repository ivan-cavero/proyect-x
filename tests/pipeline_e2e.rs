//! E2E tests for the full pipeline: config loading → agent execution → phase transitions.

use std::sync::Arc;

// ═══════════════════════════════════════════════════════════════
// PIPELINE E2E TESTS
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn e2e_pipeline_full_goal_execution() {
    // Create a temp directory with forge.toml
    let test_dir = std::env::temp_dir().join(format!("e2e-pipeline-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&test_dir).unwrap();
    std::fs::write(test_dir.join("forge.toml"), r#"
[project]
name = "test-project"
version = "0.1.0"

[roles.architect]
model = "gpt-4o"
temperature = 0.3
max_tokens = 4096
system_prompt = "You are a senior software architect."
tools = ["filesystem"]

[roles.coder]
model = "gpt-4o"
temperature = 0.3
max_tokens = 4096
system_prompt = "You are an expert Rust engineer."
tools = ["filesystem", "execute_command"]

[roles.reviewer]
model = "gpt-4o"
temperature = 0.2
max_tokens = 4096
system_prompt = "You are a senior code reviewer."
tools = ["filesystem"]

[roles.security]
model = "gpt-4o-mini"
temperature = 0.1
max_tokens = 4096
system_prompt = "You are a security auditor."
tools = ["filesystem"]

[roles.tester]
model = "gpt-4o"
temperature = 0.2
max_tokens = 4096
system_prompt = "You are a QA engineer."
tools = ["filesystem", "execute_command"]
"#).unwrap();

    // Load config
    let config_path = test_dir.join("forge.toml");
    let config = project_x_core::load_forge_config(&config_path).expect("Failed to load config");

    assert!(!config.roles.is_empty(), "Config should have roles");
    assert!(config.roles.contains_key("architect"), "Should have architect role");
    assert!(config.roles.contains_key("coder"), "Should have coder role");
    assert!(config.roles.contains_key("reviewer"), "Should have reviewer role");
    assert!(config.roles.contains_key("security"), "Should have security role");
    assert!(config.roles.contains_key("tester"), "Should have tester role");

    // Verify role configs
    let architect = config.roles.get("architect").unwrap();
    assert_eq!(architect.model, "gpt-4o");
    assert_eq!(architect.temperature, 0.3);

    // Create runtime and run goal
    let mut runtime = project_x_core::CoreRuntime::new().await.expect("Failed to create runtime");

    let result = runtime.run_goal("build a REST API for user management", Some(&config_path))
        .await
        .expect("Failed to run goal");

    // Verify pipeline executed through all phases
    assert!(!result.agent_results.is_empty(), "Should have agent results, got {}", result.agent_results.len());
    // total_duration_ms may be 0 in mock mode if execution is instant
    // The important thing is that agents were executed

    // All agents should have completed (mock mode)
    for agent_result in &result.agent_results {
        assert_eq!(
            agent_result.status,
            project_x_core::orchestrator::task::TaskStatus::Completed,
            "Agent {} should have completed",
            agent_result.agent_id
        );
    }

    // Verify phases were traversed
    let history = runtime.loop_controller.state_machine.history();
    eprintln!("History length: {}", history.len());
    for (i, transition) in history.iter().enumerate() {
        eprintln!("  Phase {}: {} → {}", i, transition.from, transition.to);
    }
    assert!(history.len() >= 3, "Should have traversed at least 3 phases, got {}", history.len());

    // Cleanup
    runtime.shutdown().await.ok();
    std::fs::remove_dir_all(&test_dir).ok();
}

#[tokio::test]
async fn e2e_pipeline_config_parsing() {
    let test_dir = std::env::temp_dir().join(format!("e2e-config-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&test_dir).unwrap();

    // Test minimal config
    std::fs::write(test_dir.join("forge.toml"), r#"
[project]
name = "minimal"

[roles.coder]
model = "gpt-4o"
"#).unwrap();

    let config = project_x_core::load_forge_config(&test_dir.join("forge.toml")).unwrap();
    assert_eq!(config.roles.len(), 1);
    assert!(config.roles.contains_key("coder"));
    assert_eq!(config.roles.get("coder").unwrap().model, "gpt-4o");

    // Test config with MCP servers
    std::fs::write(test_dir.join("forge.toml"), r#"
[project]
name = "with-mcp"

[roles.coder]
model = "gpt-4o"

[[mcp_servers]]
name = "filesystem"
command = "project-x-mcp-filesystem"
args = ["--root", "/tmp"]
"#).unwrap();

    let config = project_x_core::load_forge_config(&test_dir.join("forge.toml")).unwrap();
    assert_eq!(config.mcp_servers.len(), 1);
    assert_eq!(config.mcp_servers[0].name, "filesystem");
    assert_eq!(config.mcp_servers[0].command, "project-x-mcp-filesystem");

    std::fs::remove_dir_all(&test_dir).ok();
}

#[tokio::test]
async fn e2e_pipeline_phase_transitions() {
    let mut runtime = project_x_core::CoreRuntime::new().await.unwrap();

    // Start the loop
    runtime.loop_controller.start();
    assert!(runtime.loop_controller.running);

    // Navigate through all phases
    let phases = vec![
        project_x_core::machine::phase::Phase::Planning,
        project_x_core::machine::phase::Phase::Designing,
        project_x_core::machine::phase::Phase::Implementing,
        project_x_core::machine::phase::Phase::Reviewing,
        project_x_core::machine::phase::Phase::Testing,
        project_x_core::machine::phase::Phase::SecurityScan,
        project_x_core::machine::phase::Phase::Finalizing,
        project_x_core::machine::phase::Phase::Completed,
    ];

    for phase in &phases {
        let result = runtime.loop_controller.advance(*phase);
        assert!(result.is_ok(), "Should be able to advance to {:?}: {}", phase, result.unwrap_err());
        runtime.loop_controller.increment_iteration();
    }

    assert!(runtime.loop_controller.state_machine.current().is_terminal());

    runtime.loop_controller.stop();
    assert!(!runtime.loop_controller.running);

    runtime.shutdown().await.ok();
}

#[tokio::test]
async fn e2e_pipeline_agent_factory_all_roles() {
    use project_x_core::actor::roles::base::AgentFactory;
    use project_x_core::orchestrator::task::Task;

    let roles = vec!["architect", "coder", "reviewer", "security", "tester", "git"];

    for role_name in roles {
        let role = project_x_core::orchestrator::roles::ResolvedRole {
            role_name: role_name.to_string(),
            model: "gpt-4o".to_string(),
            temperature: 0.3,
            max_tokens: 4096,
            system_prompt: format!("You are a {}", role_name),
            tools: vec!["filesystem".to_string()],
            context_profile: "balanced".to_string(),
        };

        let agent = AgentFactory::create(&role);
        let task = Task::new(role_name, "gpt-4o", "test task");
        let result = agent.execute(&task);

        assert_eq!(
            result.status,
            project_x_core::orchestrator::task::TaskStatus::Completed,
            "{} agent should complete",
            role_name
        );
        assert!(!result.content.is_empty(), "{} agent should produce output", role_name);
    }
}

#[tokio::test]
async fn e2e_pipeline_event_bus_during_pipeline() {
    let mut runtime = project_x_core::CoreRuntime::new().await.unwrap();
    let mut rx = runtime.bus.subscribe();

    // Start loop and trigger events
    runtime.loop_controller.start();
    runtime.bus.publish(
        project_x_shared::protocol::MessageKind::SessionHeartbeat,
        "test",
    );

    // Advance a phase
    runtime.loop_controller.advance(project_x_core::machine::phase::Phase::Planning).ok();

    // Verify events were published
    let event = tokio::time::timeout(
        std::time::Duration::from_secs(1),
        rx.recv(),
    ).await;

    assert!(event.is_ok(), "Should receive event within timeout");
    let event = event.unwrap().unwrap();
    assert_eq!(event.source, "test");

    runtime.shutdown().await.ok();
}

#[tokio::test]
async fn e2e_pipeline_limits_enforcement() {
    let limits = project_x_core::r#loop::Limits {
        max_iterations_per_goal: 3,
        max_iterations_per_phase: 2,
        session_ttl_seconds: 3600,
        phase_timeout_seconds: 300,
        cycle_detection_window: 4,
    };

    let mut runtime = project_x_core::CoreRuntime::new().await.unwrap();
    runtime.loop_controller = project_x_core::r#loop::LoopController::with_limits(limits);
    runtime.loop_controller.start();

    // Advance to planning
    runtime.loop_controller.advance(project_x_core::machine::phase::Phase::Planning).ok();

    // Do 2 iterations (should be OK)
    runtime.loop_controller.increment_iteration();
    assert!(runtime.loop_controller.check_limits().is_none());
    runtime.loop_controller.increment_iteration();
    assert!(runtime.loop_controller.check_limits().is_none());

    // 3rd iteration should violate phase limit
    runtime.loop_controller.increment_iteration();
    assert!(runtime.loop_controller.check_limits().is_some(), "Should detect phase limit violation");

    runtime.shutdown().await.ok();
}

#[tokio::test]
async fn e2e_pipeline_drift_guard_during_pipeline() {
    let mut runtime = project_x_core::CoreRuntime::new().await.unwrap();

    // Record some metrics
    for i in 0..15 {
        runtime.drift_guard.record_and_evaluate(
            project_x_core::drift::metrics::MetricSample {
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
            },
            None,
        );
    }

    // Verify health summary
    let health = runtime.drift_guard.health_summary();
    assert!(health.overall_asi > 80.0, "Healthy pipeline should have high ASI");

    runtime.shutdown().await.ok();
}
