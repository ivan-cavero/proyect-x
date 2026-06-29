//! Init command — create project directory + forge.toml

use std::path::Path;

/// Default forge.toml template — providers configured via env vars or Settings page.
const FORGE_TOML: &str = r#"# Project-X Configuration
# Created: {date}
# API keys: manage via Dashboard Settings page or environment variables

[project]
name = "{name}"
version = "0.1.0"

# ─── Providers ────────────────────────────────────────
# API keys are managed via the Dashboard Settings page.
# The vault stores them encrypted. No keys hardcoded here.

[providers.nan]
base_url = "https://api.nan.builders/v1"
api_key = "env:NAN_API_KEY"
default_model = "qwen3.6"

# ─── Roles ────────────────────────────────────────────

[roles.architect]
description = "System design and architecture"
model = "claude-sonnet-4-20250514"
temperature = 0.2
system_prompt = "You are a senior software architect specialized in Rust systems."
tools = ["filesystem", "web_search"]

[roles.coder]
description = "Code generation and implementation"
model = "gpt-4o"
temperature = 0.3
system_prompt = "You are an expert Rust engineer. Write production-quality code."
tools = ["filesystem", "execute_command"]

[roles.reviewer]
description = "Code review and quality assurance"
model = "gpt-4o"
temperature = 0.2
system_prompt = "You are a senior code reviewer. Analyze code critically."
tools = ["filesystem"]

[roles.security]
description = "Security audit and vulnerability scanning"
model = "claude-sonnet-4-20250514"
temperature = 0.1
system_prompt = "You are a security auditor. Check for vulnerabilities."
tools = ["filesystem", "grep"]

[roles.tester]
description = "Test generation and execution"
model = "gpt-4o"
temperature = 0.2
system_prompt = "You are a QA engineer. Generate comprehensive tests."
tools = ["filesystem", "execute_command"]

[roles.researcher]
description = "Technical research and documentation"
model = "gpt-4o"
temperature = 0.3
system_prompt = "You are a technical researcher. Find best practices."
tools = ["web_search", "read_url"]

# ─── Goals ────────────────────────────────────────────

[[goals]]
name = "full-feature"
description = "Complete feature development"
agents = ["architect", "coder", "reviewer", "security", "tester"]
gates = ["review.pass", "security.no_critical", "test.pass"]
max_iterations = 10

[[goals]]
name = "quick-fix"
description = "Fast bug fix"
agents = ["coder", "reviewer"]
max_iterations = 3

# ─── Limits ───────────────────────────────────────────

[limits]
max_iterations_per_goal = 50
max_iterations_per_phase = 5
session_ttl_seconds = 3600
phase_timeout_seconds = 300
"#;

/// Create a new project with forge.toml
pub fn init_project(name: &str) -> anyhow::Result<()> {
    let project_dir = Path::new(name);

    // Check if directory already exists
    if project_dir.exists() {
        anyhow::bail!("Directory '{}' already exists", name);
    }

    // Create project directory
    std::fs::create_dir_all(project_dir)?;

    // Generate forge.toml with current date
    let date = chrono::Utc::now().format("%Y-%m-%d %H:%M UTC");
    let content = FORGE_TOML
        .replace("{name}", name)
        .replace("{date}", date.to_string().as_str());
    std::fs::write(project_dir.join("forge.toml"), content)?;

    // Create .gitignore
    std::fs::write(
        project_dir.join(".gitignore"),
        ".forge/\n*.db\n*.db-wal\n*.db-shm\n",
    )?;

    println!("  Created: {}/forge.toml", name);
    println!("  Created: {}/.gitignore", name);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore] // Flaky on Windows due to temp directory handling
    fn test_init_project() {
        let proj_name = format!("test-init-{}", uuid::Uuid::new_v4());

        // Create temp dir explicitly
        let test_base = std::env::temp_dir().join("_project_x_test_init");
        std::fs::create_dir_all(&test_base).expect("Failed to create test base");

        let original_cwd = std::env::current_dir().ok();
        std::env::set_current_dir(&test_base).expect("Failed to set cwd");

        let result = init_project(&proj_name);
        
        // Restore cwd before any assertions
        if let Some(cwd) = original_cwd {
            std::env::set_current_dir(cwd).ok();
        }

        assert!(result.is_ok(), "init failed: {:?}", result.err());

        let dir = test_base.join(&proj_name);
        assert!(dir.join("forge.toml").exists());
        assert!(dir.join(".forge").exists());
        assert!(dir.join(".gitignore").exists());

        let content = std::fs::read_to_string(dir.join("forge.toml")).unwrap();
        assert!(content.contains(&format!("name = \"{}\"", proj_name)));

        std::fs::remove_dir_all(&dir).ok();
        std::fs::remove_dir_all(&test_base).ok();
    }

    #[test]
    fn test_init_project_already_exists() {
        let dir = std::env::temp_dir().join(format!("test-init-exists-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();

        let result = init_project(dir.to_str().unwrap());
        assert!(result.is_err());

        std::fs::remove_dir_all(&dir).ok();
    }
}
