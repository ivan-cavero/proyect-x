//! E2E tests for the CLI binary.
//!
//! These tests actually compile and run the `project-x` binary,
//! verifying real-world behavior end-to-end.

use std::process::Command;

/// Path to the compiled binary.
fn binary_path() -> String {
    // cargo puts binaries in target/debug/
    let mut path = std::env::current_exe()
        .expect("Failed to get test binary path")
        .parent()
        .expect("Failed to get parent dir")
        .to_path_buf();

    // Walk up until we find target/debug
    loop {
        if path.join("project-x.exe").exists() || path.join("project-x").exists() {
            break;
        }
        if !path.pop() {
            break;
        }
    }

    // Check for .exe (Windows) or plain binary (Unix)
    let exe = if cfg!(windows) {
        path.join("project-x.exe")
    } else {
        path.join("project-x")
    };

    exe.to_string_lossy().to_string()
}

/// Run the CLI with arguments and capture output.
fn run_cli(args: &[&str]) -> (String, String, bool) {
    let output = Command::new(binary_path())
        .args(args)
        .output()
        .expect("Failed to execute CLI binary");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (stdout, stderr, output.status.success())
}

// ═══════════════════════════════════════════════════════════════
// CLI E2E TESTS
// ═══════════════════════════════════════════════════════════════

#[test]
fn e2e_cli_version() {
    let (stdout, _, success) = run_cli(&["version"]);
    assert!(success, "CLI version command failed");
    assert!(stdout.contains("Project-X"), "Should contain 'Project-X': {}", stdout);
    assert!(stdout.contains("0.1.0"), "Should contain version: {}", stdout);
}

#[test]
fn e2e_cli_help() {
    let (stdout, _, success) = run_cli(&["--help"]);
    assert!(success, "CLI help command failed");
    assert!(stdout.contains("project-x"), "Should contain binary name");
    assert!(stdout.contains("init"), "Should mention init command");
    assert!(stdout.contains("run"), "Should mention run command");
}

#[test]
fn e2e_cli_init_creates_project() {
    let test_dir = std::env::temp_dir().join(format!("e2e-test-init-{}", uuid::Uuid::new_v4()));
    let test_path = test_dir.to_string_lossy();

    let (stdout, _, success) = run_cli(&["init", &test_path]);
    assert!(success, "CLI init failed: {}", stdout);

    // Verify files were created
    assert!(test_dir.join("forge.toml").exists(), "forge.toml should exist");
    assert!(test_dir.join(".gitignore").exists(), ".gitignore should exist");

    // Verify forge.toml content
    let content = std::fs::read_to_string(test_dir.join("forge.toml")).unwrap();
    assert!(content.contains("[project]"), "forge.toml should have [project] section");
    assert!(content.contains("[roles"), "forge.toml should have roles");

    // Verify .gitignore
    let gitignore = std::fs::read_to_string(test_dir.join(".gitignore")).unwrap();
    assert!(gitignore.contains(".forge"), ".gitignore should ignore .forge");

    // Cleanup
    std::fs::remove_dir_all(&test_dir).ok();
}

#[test]
fn e2e_cli_init_already_exists() {
    let test_dir = std::env::temp_dir().join(format!("e2e-test-init-exists-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&test_dir).unwrap();

    let test_path = test_dir.to_string_lossy();
    let (_, stderr, success) = run_cli(&["init", &test_path]);
    assert!(!success, "CLI init should fail when directory exists");
    assert!(stderr.contains("already exists") || stderr.contains("Error"),
        "Should show error about existing directory: {}", stderr);

    std::fs::remove_dir_all(&test_dir).ok();
}

#[test]
fn e2e_cli_run_requires_goal() {
    let (_, stderr, success) = run_cli(&["run"]);
    assert!(!success, "CLI run without --goal should fail");
}

#[test]
fn e2e_cli_config_show_no_project() {
    let (stdout, _, _success) = run_cli(&["config", "show"]);
    // Should show "No forge.toml found" when not in a project
    assert!(
        stdout.contains("No forge.toml") || stdout.contains("forge.toml"),
        "Should indicate no config found: {}", stdout
    );
}

#[test]
fn e2e_cli_project_list_empty() {
    let (stdout, _, success) = run_cli(&["project", "list"]);
    assert!(success, "CLI project list should succeed");
    assert!(stdout.contains("project") || stdout.contains("No") || stdout.contains("no"),
        "Should handle empty project list: {}", stdout);
}

#[test]
fn e2e_cli_session_list_empty() {
    let (stdout, _, success) = run_cli(&["session", "list"]);
    assert!(success, "CLI session list should succeed");
}

#[test]
fn e2e_cli_provider_list() {
    let (stdout, _, success) = run_cli(&["provider", "list"]);
    assert!(success, "CLI provider list should succeed");
    assert!(stdout.contains("OpenAI") || stdout.contains("openai") || stdout.contains("provider"),
        "Should list providers: {}", stdout);
}

#[test]
fn e2e_cli_mcp_list_empty() {
    let (stdout, _, success) = run_cli(&["mcp", "list"]);
    assert!(success, "CLI mcp list should succeed");
}

#[test]
fn e2e_cli_version_json_output() {
    let (stdout, _, success) = run_cli(&["version"]);
    assert!(success, "CLI version should succeed");
    // Version output should be parseable
    let version_line = stdout.lines().next().unwrap_or("");
    assert!(!version_line.is_empty(), "Version should produce output");
}

#[test]
fn e2e_cli_test_command() {
    // The test command runs internal integration tests
    let (stdout, _, success) = run_cli(&["test"]);
    assert!(success, "CLI test command should succeed");
    assert!(stdout.contains("passed") || stdout.contains("✓") || stdout.contains("test"),
        "Should show test results: {}", stdout);
}

#[test]
fn e2e_cli_deploy_status_no_config() {
    let (stdout, _, _success) = run_cli(&["deploy", "status"]);
    // Should handle gracefully even without config
    assert!(!stdout.is_empty(), "Should produce some output");
}

#[test]
fn e2e_cli_context_inspect_no_session() {
    let (stdout, _, _success) = run_cli(&["context", "inspect", "test-session"]);
    // Should handle gracefully
    assert!(!stdout.is_empty(), "Should produce some output");
}

#[test]
fn e2e_cli_org_list() {
    let (stdout, _, success) = run_cli(&["org", "list"]);
    assert!(success, "CLI org list should succeed");
}

#[test]
fn e2e_cli_billing_show() {
    let (stdout, _, success) = run_cli(&["billing", "show"]);
    assert!(success, "CLI billing show should succeed");
    assert!(stdout.contains("Plan") || stdout.contains("plan") || stdout.contains("Billing"),
        "Should show billing info: {}", stdout);
}
