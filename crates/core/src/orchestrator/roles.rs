//! Role configuration and goal-to-workflow mapping.

use serde::{Deserialize, Serialize};

/// Role configuration from TOML.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleConfig {
    pub name: String,
    pub description: Option<String>,
    pub model: String,
    pub temperature: f32,
    pub max_tokens: u32,
    pub system_prompt: Option<String>,
    pub tools: Vec<String>,
    pub context_profile: Option<String>,
    pub context_priority: Option<String>,
}

impl Default for RoleConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            description: None,
            model: "gpt-5".to_string(),
            temperature: 0.3,
            max_tokens: 4096,
            system_prompt: None,
            tools: Vec::new(),
            context_profile: None,
            context_priority: None,
        }
    }
}

/// Per-goal role overrides.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleOverride {
    pub model: Option<String>,
    pub temperature: Option<f32>,
    pub system_prompt: Option<String>,
    pub context_profile: Option<String>,
}

/// Goal configuration from TOML.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GoalConfig {
    pub name: String,
    pub description: Option<String>,
    pub agents: Vec<String>,
    pub gates: Vec<String>,
    pub max_iterations: Option<u32>,
    pub parallel_reviewers: Option<u32>,
    pub workflow: Option<String>,
    pub agent_overrides: Option<std::collections::HashMap<String, RoleOverride>>,
}

/// Resolved agent role for a specific execution.
#[derive(Debug, Clone)]
pub struct ResolvedRole {
    pub role_name: String,
    pub model: String,
    pub temperature: f32,
    pub max_tokens: u32,
    pub system_prompt: String,
    pub tools: Vec<String>,
    pub context_profile: String,
}

impl ResolvedRole {
    /// Resolve a role from config and overrides.
    pub fn resolve(base: &RoleConfig, override_config: Option<&RoleOverride>) -> Self {
        let model = override_config
            .and_then(|o| o.model.clone())
            .unwrap_or_else(|| base.model.clone());

        let temperature = override_config
            .and_then(|o| o.temperature)
            .unwrap_or(base.temperature);

        let system_prompt = override_config
            .and_then(|o| o.system_prompt.clone())
            .or_else(|| base.system_prompt.clone())
            .unwrap_or_else(|| format!("You are a helpful {} assistant.", base.name));

        let context_profile = override_config
            .and_then(|o| o.context_profile.clone())
            .or_else(|| base.context_profile.clone())
            .unwrap_or_else(|| "balanced".to_string());

        Self {
            role_name: base.name.clone(),
            model,
            temperature,
            max_tokens: base.max_tokens,
            system_prompt,
            tools: base.tools.clone(),
            context_profile,
        }
    }
}

/// Workflow configuration (from TOML or default).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowConfig {
    pub name: String,
    pub phases: Vec<PhaseConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseConfig {
    pub name: String,
    pub agents: Vec<String>,
    pub gate: Option<String>,
    pub parallel: Option<bool>,
}

/// Resolve agent roles for a goal.
pub fn resolve_agents(
    goal: &GoalConfig,
    available_roles: &std::collections::HashMap<String, RoleConfig>,
) -> Result<Vec<ResolvedRole>, String> {
    let mut roles = Vec::new();

    for role_name in &goal.agents {
        let base = available_roles.get(role_name)
            .ok_or_else(|| format!("Role '{}' not found in config", role_name))?;

        let override_config = goal.agent_overrides.as_ref()
            .and_then(|o| o.get(role_name));

        roles.push(ResolvedRole::resolve(base, override_config));
    }

    Ok(roles)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_role_resolve_defaults() {
        let config = RoleConfig {
            name: "coder".to_string(),
            model: "gpt-5".to_string(),
            temperature: 0.3,
            max_tokens: 4096,
            system_prompt: Some("You are a Rust expert.".to_string()),
            tools: vec!["filesystem".to_string()],
            ..Default::default()
        };

        let role = ResolvedRole::resolve(&config, None);
        assert_eq!(role.model, "gpt-5");
        assert_eq!(role.temperature, 0.3);
        assert!(role.system_prompt.contains("Rust expert"));
    }

    #[test]
    fn test_agent_role_resolve_override() {
        let config = RoleConfig {
            name: "coder".to_string(),
            model: "gpt-5".to_string(),
            temperature: 0.3,
            ..Default::default()
        };

        let override_config = RoleOverride {
            model: Some("claude-4-opus".to_string()),
            temperature: Some(0.1),
            system_prompt: None,
            context_profile: None,
        };

        let role = ResolvedRole::resolve(&config, Some(&override_config));
        assert_eq!(role.model, "claude-4-opus"); // Overridden
        assert_eq!(role.temperature, 0.1); // Overridden
    }

    #[test]
    fn test_resolve_agents() {
        let mut available = std::collections::HashMap::new();
        available.insert("coder".to_string(), RoleConfig {
            name: "coder".to_string(),
            model: "gpt-5".to_string(),
            ..Default::default()
        });
        available.insert("reviewer".to_string(), RoleConfig {
            name: "reviewer".to_string(),
            model: "claude-4-opus".to_string(),
            ..Default::default()
        });

        let goal = GoalConfig {
            name: "test".to_string(),
            agents: vec!["coder".to_string(), "reviewer".to_string()],
            ..Default::default()
        };

        let roles = resolve_agents(&goal, &available).unwrap();
        assert_eq!(roles.len(), 2);
        assert_eq!(roles[0].model, "gpt-5");
        assert_eq!(roles[1].model, "claude-4-opus");
    }

    #[test]
    fn test_resolve_agents_missing_role() {
        let available = std::collections::HashMap::new();
        let goal = GoalConfig {
            name: "test".to_string(),
            agents: vec!["nonexistent".to_string()],
            ..Default::default()
        };

        let result = resolve_agents(&goal, &available);
        assert!(result.is_err());
    }
}