//! ProviderRouter — model name → correct provider.
//!
//! Routes model names to the correct LLM provider implementation.
//! Supports OpenAI, Anthropic, Gemini, Ollama, and any OpenAI-compatible endpoint.

use std::collections::HashMap;
use std::sync::Arc;

/// A registered provider with its metadata.
struct RegisteredProvider {
    provider: Arc<dyn project_x_agent_traits::provider::LLMProvider>,
    tier: project_x_agent_traits::provider::ModelTier,
}

/// Routes model names to the correct provider.
pub struct ProviderRouter {
    providers: HashMap<String, RegisteredProvider>,
    /// Default provider when model isn't found.
    default_provider: Option<String>,
}

impl ProviderRouter {
    /// Create an empty router.
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
            default_provider: None,
        }
    }

    /// Register a provider for a set of model names.
    pub fn register(
        &mut self,
        name: &str,
        provider: Arc<dyn project_x_agent_traits::provider::LLMProvider>,
        tier: project_x_agent_traits::provider::ModelTier,
    ) {
        self.providers.insert(
            name.to_string(),
            RegisteredProvider { provider, tier },
        );
    }

    /// Set the default provider.
    pub fn set_default(&mut self, name: &str) {
        self.default_provider = Some(name.to_string());
    }

    /// Resolve a model name to a provider.
    ///
    /// Matching rules:
    /// 1. Exact provider name match (e.g., "openai")
    /// 2. Model name prefix match (e.g., "gpt-5" → openai)
    /// 3. Default provider
    /// 4. Error
    pub fn resolve(&self, model: &str) -> Result<Arc<dyn project_x_agent_traits::provider::LLMProvider>, String> {
        // 1. Direct provider name
        if let Some(reg) = self.providers.get(model) {
            return Ok(reg.provider.clone());
        }

        // 2. Model prefix matching
        let provider_name = Self::detect_provider(model);
        if let Some(name) = provider_name {
            if let Some(reg) = self.providers.get(&name) {
                return Ok(reg.provider.clone());
            }
        }

        // 3. Default
        if let Some(ref default) = self.default_provider {
            if let Some(reg) = self.providers.get(default) {
                return Ok(reg.provider.clone());
            }
        }

        Err(format!("No provider found for model '{}'", model))
    }

    /// Get the tier for a model.
    pub fn tier_for(&self, model: &str) -> Option<project_x_agent_traits::provider::ModelTier> {
        if let Some(name) = Self::detect_provider(model) {
            if let Some(reg) = self.providers.get(&name) {
                return Some(reg.tier.clone());
            }
        }
        None
    }

    /// List all registered providers.
    pub fn list(&self) -> Vec<(String, String)> {
        self.providers
            .iter()
            .map(|(name, reg)| (name.clone(), reg.provider.provider_name().to_string()))
            .collect()
    }

    /// Detect which provider a model name belongs to.
    pub fn detect_provider(model: &str) -> Option<String> {
        let lower = model.to_lowercase();

        if lower.starts_with("gpt-") || lower.starts_with("text-embedding-") {
            Some("openai".to_string())
        } else if lower.starts_with("claude-") {
            Some("anthropic".to_string())
        } else if lower.starts_with("gemini-") {
            Some("gemini".to_string())
        } else if lower.starts_with("llama-") || lower.starts_with("mistral-") || lower.starts_with("qwen-") || lower.starts_with("codellama-") {
            Some("ollama".to_string())
        } else {
            None
        }
    }
}

impl Default for ProviderRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MockProvider;
    use project_x_agent_traits::provider::ModelTier;

    #[test]
    fn test_detect_provider() {
        assert_eq!(ProviderRouter::detect_provider("gpt-5"), Some("openai".to_string()));
        assert_eq!(ProviderRouter::detect_provider("gpt-4o"), Some("openai".to_string()));
        assert_eq!(ProviderRouter::detect_provider("claude-4-opus"), Some("anthropic".to_string()));
        assert_eq!(ProviderRouter::detect_provider("gemini-2.5-pro"), Some("gemini".to_string()));
        assert_eq!(ProviderRouter::detect_provider("llama-4"), Some("ollama".to_string()));
        assert_eq!(ProviderRouter::detect_provider("text-embedding-3-large"), Some("openai".to_string()));
        assert_eq!(ProviderRouter::detect_provider("unknown-model"), None);
    }

    #[tokio::test]
    async fn test_router_resolve() {
        let mut router = ProviderRouter::new();

        let openai: Arc<dyn project_x_agent_traits::provider::LLMProvider> =
            Arc::new(MockProvider::simple("openai response"));
        let anthropic: Arc<dyn project_x_agent_traits::provider::LLMProvider> =
            Arc::new(MockProvider::simple("anthropic response"));

        router.register("openai", openai, ModelTier::Balanced);
        router.register("anthropic", anthropic, ModelTier::Capable);

        // Direct name
        let p = router.resolve("openai").unwrap();
        assert_eq!(p.provider_name(), "mock");

        // Model prefix
        let p = router.resolve("gpt-5").unwrap();
        assert_eq!(p.provider_name(), "mock");

        let p = router.resolve("claude-4-opus").unwrap();
        assert_eq!(p.provider_name(), "mock");

        // Unknown model without default → error
        assert!(router.resolve("unknown-model").is_err());

        // With default
        router.set_default("openai");
        let p = router.resolve("unknown-model").unwrap();
        assert_eq!(p.provider_name(), "mock");
    }
}