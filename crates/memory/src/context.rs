//! Context Window Management — budgeting, compression, health metrics.
//!
//! Ensures agents never exceed model context limits.
//! Applies compression pipeline when pressure is high.



// ─── Token Counter ────────────────────────────────────────────

/// Counts tokens in messages using a simple estimator.
///
/// For production: use tiktoken-rs or model-specific tokenizer.
/// This is a fast approximation for budget calculations.
pub struct TokenCounter {
    /// Average chars per token (model-dependent).
    chars_per_token: f32,
}

impl TokenCounter {
    /// Create a token counter for a specific model family.
    pub fn for_model(model: &str) -> Self {
        let chars_per_token = match model {
            m if m.starts_with("gpt-") => 4.0,
            m if m.starts_with("claude-") => 3.5,
            m if m.starts_with("gemini-") => 4.0,
            m if m.starts_with("llama-") => 3.8,
            _ => 4.0, // Default approximation
        };

        Self { chars_per_token }
    }

    /// Estimate tokens for a string.
    pub fn count_tokens(&self, text: &str) -> u32 {
        (text.len() as f32 / self.chars_per_token).ceil() as u32
    }

    /// Count tokens for a list of messages.
    pub fn count_messages(&self, messages: &[Message]) -> u32 {
        messages.iter().map(|m| {
            let role_tokens = 4; // ~4 tokens for role prefix
            let content_tokens = self.count_tokens(&m.content);
            role_tokens + content_tokens
        }).sum()
    }
}

/// A message in the context window.
#[derive(Debug, Clone)]
pub struct Message {
    pub role: String,
    pub content: String,
}

// ─── Budget Profile ───────────────────────────────────────────

/// Predefined budget allocation profiles.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum BudgetProfile {
    /// Default: balanced allocation across sections.
    Balanced,
    /// More memory for RAG, less for history.
    Generous,
    /// Aggressive compression, minimal context.
    Aggressive,
    /// Max RAG, for research tasks.
    Research,
}

impl BudgetProfile {
    /// Get the allocation percentages for each section.
    pub fn allocations(&self) -> SectionAllocations {
        match self {
            BudgetProfile::Balanced => SectionAllocations {
                system_prompt: 0.05,
                goal_definition: 0.05,
                active_task: 0.15,
                tool_results: 0.10,
                recent_history: 0.35,
                memory_rag: 0.25,
                project_context: 0.05,
            },
            BudgetProfile::Generous => SectionAllocations {
                system_prompt: 0.05,
                goal_definition: 0.05,
                active_task: 0.10,
                tool_results: 0.10,
                recent_history: 0.25,
                memory_rag: 0.35,
                project_context: 0.10,
            },
            BudgetProfile::Aggressive => SectionAllocations {
                system_prompt: 0.05,
                goal_definition: 0.05,
                active_task: 0.20,
                tool_results: 0.05,
                recent_history: 0.20,
                memory_rag: 0.10,
                project_context: 0.05,
            },
            BudgetProfile::Research => SectionAllocations {
                system_prompt: 0.05,
                goal_definition: 0.05,
                active_task: 0.10,
                tool_results: 0.05,
                recent_history: 0.15,
                memory_rag: 0.50,
                project_context: 0.10,
            },
        }
    }
}

/// Token allocation percentages for each section.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SectionAllocations {
    pub system_prompt: f32,
    pub goal_definition: f32,
    pub active_task: f32,
    pub tool_results: f32,
    pub recent_history: f32,
    pub memory_rag: f32,
    pub project_context: f32,
}

// ─── Context Budget ───────────────────────────────────────────

/// Token budget for a context window.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ContextBudget {
    /// Model's maximum context window.
    pub model_max: usize,
    /// Hard limit (typically 70% of model_max).
    pub hard_limit: usize,
    /// Current allocated tokens.
    pub allocated: usize,
    /// Profile determining allocation.
    pub profile: BudgetProfile,
}

impl ContextBudget {
    /// Create a budget from model info.
    pub fn new(model_max: usize, profile: BudgetProfile) -> Self {
        let hard_limit = (model_max as f32 * 0.7) as usize;
        Self {
            model_max,
            hard_limit,
            allocated: 0,
            profile,
        }
    }

    /// Calculate the budget for a specific section.
    pub fn section_budget(&self, section: Section) -> usize {
        let allocations = self.profile.allocations();
        let pct = match section {
            Section::SystemPrompt => allocations.system_prompt,
            Section::GoalDefinition => allocations.goal_definition,
            Section::ActiveTask => allocations.active_task,
            Section::ToolResults => allocations.tool_results,
            Section::RecentHistory => allocations.recent_history,
            Section::MemoryRag => allocations.memory_rag,
            Section::ProjectContext => allocations.project_context,
        };
        (self.hard_limit as f32 * pct) as usize
    }

    /// Current pressure (0.0–1.0).
    pub fn pressure(&self) -> f32 {
        if self.hard_limit == 0 {
            return 0.0;
        }
        (self.allocated as f32 / self.hard_limit as f32).min(1.0)
    }

    /// Check if over budget.
    pub fn is_over_limit(&self) -> bool {
        self.allocated > self.hard_limit
    }

    /// Remaining tokens.
    pub fn remaining(&self) -> usize {
        self.hard_limit.saturating_sub(self.allocated)
    }
}

/// Section types for budget allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    SystemPrompt,
    GoalDefinition,
    ActiveTask,
    ToolResults,
    RecentHistory,
    MemoryRag,
    ProjectContext,
}

// ─── Compression Pipeline ─────────────────────────────────────

/// Result of a compression step.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CompressionResult {
    pub before_tokens: u32,
    pub after_tokens: u32,
    pub ratio: f32,
    pub technique: String,
}

/// The compression pipeline reduces context size when over budget.
pub struct CompressionPipeline;

impl CompressionPipeline {
    /// Compress context to fit within budget.
    /// Returns compression results for logging.
    pub fn compress(context: &mut ContextWindow, budget: &mut ContextBudget) -> Vec<CompressionResult> {
        let mut results = Vec::new();

        while budget.is_over_limit() && !context.is_empty() {
            // Step 1: Truncate tool results
            if let Some(r) = Self::truncate_tool_results(context, budget) {
                results.push(r);
                continue;
            }

            // Step 2: Compress history (summarize old interactions)
            if let Some(r) = Self::compress_history(context, budget) {
                results.push(r);
                continue;
            }

            // Step 3: Reduce RAG chunks
            if let Some(r) = Self::reduce_rag(context, budget) {
                results.push(r);
                continue;
            }

            // Step 4: Prune project context
            if let Some(r) = Self::prune_project_context(context, budget) {
                results.push(r);
                continue;
            }

            // Step 5: Emergency — full consolidation
            if let Some(r) = Self::emergency_consolidation(context, budget) {
                results.push(r);
                break; // Emergency stops the loop
            }

            // Nothing more to compress
            break;
        }

        results
    }

    /// Step 1: Truncate tool results to summaries.
    fn truncate_tool_results(context: &mut ContextWindow, budget: &mut ContextBudget) -> Option<CompressionResult> {
        let before = budget.allocated;

        // Find and truncate long tool results
        for msg in &mut context.messages {
            if msg.role == "tool" && msg.content.len() > 500 {
                let original_len = msg.content.len();
                msg.content = format!("{}... [truncated, {} chars total]",
                    &msg.content[..200], original_len);
                let saved = (original_len - msg.content.len()) as u32 / 4; // ~4 chars per token
                budget.allocated = budget.allocated.saturating_sub(saved as usize);
            }
        }

        if budget.allocated < before {
            Some(CompressionResult {
                before_tokens: before as u32,
                after_tokens: budget.allocated as u32,
                ratio: budget.allocated as f32 / before as f32,
                technique: "truncate_tool_results".to_string(),
            })
        } else {
            None
        }
    }

    /// Step 2: Compress old history (keep last 5, summarize rest).
    fn compress_history(context: &mut ContextWindow, budget: &mut ContextBudget) -> Option<CompressionResult> {
        if context.messages.len() <= 5 {
            return None;
        }

        let before = budget.allocated;
        let keep = 5;
        let to_remove = context.messages.len() - keep;

        // Summarize removed messages
        let summary: String = context.messages[..to_remove]
            .iter()
            .map(|m| format!("{}: {}", m.role, &m.content[..m.content.len().min(50)]))
            .collect::<Vec<_>>()
            .join("; ");

        let summary_msg = Message {
            role: "system".to_string(),
            content: format!("[Context summary of {} previous messages: {}]", to_remove, summary),
        };

        // Replace old messages with summary
        context.messages = {
            let mut new = vec![summary_msg];
            new.extend(context.messages[to_remove..].to_vec());
            new
        };

        let _saved = before.saturating_sub(Self::estimate_tokens(&context.messages));
        budget.allocated = Self::estimate_tokens(&context.messages);

        Some(CompressionResult {
            before_tokens: before as u32,
            after_tokens: budget.allocated as u32,
            ratio: budget.allocated as f32 / before as f32,
            technique: "compress_history".to_string(),
        })
    }

    /// Step 3: Reduce RAG chunks (keep top 3).
    fn reduce_rag(context: &mut ContextWindow, budget: &mut ContextBudget) -> Option<CompressionResult> {
        let rag_count = context.messages.iter()
            .filter(|m| m.role == "rag_context")
            .count();

        if rag_count <= 3 {
            return None;
        }

        let before = budget.allocated;

        // Keep only the last 3 RAG chunks
        let mut rag_seen = 0;
        context.messages.retain(|m| {
            if m.role == "rag_context" {
                rag_seen += 1;
                rag_seen <= 3
            } else {
                true
            }
        });

        budget.allocated = Self::estimate_tokens(&context.messages);

        Some(CompressionResult {
            before_tokens: before as u32,
            after_tokens: budget.allocated as u32,
            ratio: budget.allocated as f32 / before as f32,
            technique: "reduce_rag".to_string(),
        })
    }

    /// Step 4: Prune low-relevance project context.
    fn prune_project_context(context: &mut ContextWindow, budget: &mut ContextBudget) -> Option<CompressionResult> {
        let before = budget.allocated;

        // Remove project context messages (keep first one as summary)
        let mut pc_count = 0;
        context.messages.retain(|m| {
            if m.role == "project_context" {
                pc_count += 1;
                pc_count == 1 // Keep only first
            } else {
                true
            }
        });

        budget.allocated = Self::estimate_tokens(&context.messages);

        if budget.allocated < before {
            Some(CompressionResult {
                before_tokens: before as u32,
                after_tokens: budget.allocated as u32,
                ratio: budget.allocated as f32 / before as f32,
                technique: "prune_project_context".to_string(),
            })
        } else {
            None
        }
    }

    /// Step 5: Emergency consolidation — summarize everything.
    fn emergency_consolidation(context: &mut ContextWindow, budget: &mut ContextBudget) -> Option<CompressionResult> {
        let before = budget.allocated;

        // Create a single summary of everything
        let summary: String = context.messages.iter()
            .take(20) // First 20 messages
            .map(|m| format!("{}: {}", m.role, &m.content[..m.content.len().min(100)]))
            .collect::<Vec<_>>()
            .join("\n");

        context.messages = vec![
            Message {
                role: "system".to_string(),
                content: format!("[Full context consolidated: {} messages summary]\n{}", context.messages.len(), summary),
            }
        ];

        budget.allocated = Self::estimate_tokens(&context.messages);

        Some(CompressionResult {
            before_tokens: before as u32,
            after_tokens: budget.allocated as u32,
            ratio: budget.allocated as f32 / before as f32,
            technique: "emergency_consolidation".to_string(),
        })
    }

    /// Estimate total tokens in messages.
    fn estimate_tokens(messages: &[Message]) -> usize {
        messages.iter().map(|m| {
            (m.content.len() as f32 / 4.0).ceil() as usize + 4
        }).sum()
    }
}

// ─── Context Window (for compression) ─────────────────────────

/// A context window that can be compressed.
#[derive(Debug, Clone)]
pub struct ContextWindow {
    pub messages: Vec<Message>,
}

impl ContextWindow {
    pub fn new() -> Self {
        Self { messages: Vec::new() }
    }

    pub fn push(&mut self, msg: Message) {
        self.messages.push(msg);
    }

    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    pub fn len(&self) -> usize {
        self.messages.len()
    }

    pub fn clear(&mut self) {
        self.messages.clear();
    }
}

// ─── Context Health Metrics ───────────────────────────────────

/// Health metrics for context management.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ContextHealth {
    pub pressure: f32,
    pub compression_ratio: f32,
    pub compression_frequency: u32,
    pub message_count: usize,
    pub total_tokens: usize,
    pub budget_utilization: f32,
}

impl ContextHealth {
    /// Evaluate context health from metrics.
    pub fn evaluate(pressure: f32, compressions: u32, messages: usize, tokens: usize, budget: usize) -> Self {
        let ratio = if budget > 0 { tokens as f32 / budget as f32 } else { 0.0 };
        Self {
            pressure,
            compression_ratio: ratio,
            compression_frequency: compressions,
            message_count: messages,
            total_tokens: tokens,
            budget_utilization: if budget > 0 { tokens as f32 / budget as f32 } else { 0.0 },
        }
    }

    /// Health status based on metrics.
    pub fn status(&self) -> HealthStatus {
        if self.pressure > 0.95 {
            HealthStatus::Critical
        } else if self.pressure > 0.80 {
            HealthStatus::Warning
        } else if self.compression_frequency > 5 {
            HealthStatus::Warning
        } else {
            HealthStatus::Healthy
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HealthStatus {
    Healthy,
    Warning,
    Critical,
}

// ─── Context Manager ──────────────────────────────────────────

/// Orchestrates context budget, compression, and health monitoring.
pub struct ContextManager {
    pub budget: ContextBudget,
    pub health: ContextHealth,
    compression_count: u32,
}

impl ContextManager {
    /// Create a new context manager.
    pub fn new(model_max: usize, profile: BudgetProfile) -> Self {
        Self {
            budget: ContextBudget::new(model_max, profile),
            health: ContextHealth::evaluate(0.0, 0, 0, 0, model_max),
            compression_count: 0,
        }
    }

    /// Prepare context for an LLM call.
    /// Compresses if needed, returns the ready context.
    pub fn prepare<'a>(&'a mut self, context: &'a mut ContextWindow) -> &'a ContextWindow {
        // Calculate current pressure
        self.budget.allocated = CompressionPipeline::estimate_tokens(&context.messages);

        // Compress if over limit
        if self.budget.is_over_limit() {
            let results = CompressionPipeline::compress(context, &mut self.budget);
            self.compression_count += results.len() as u32;

            for r in &results {
                tracing::info!(
                    "Context compression: {} ({:.1}% → {:.1}%)",
                    r.technique,
                    r.before_tokens as f32 / self.budget.hard_limit as f32 * 100.0,
                    r.after_tokens as f32 / self.budget.hard_limit as f32 * 100.0,
                );
            }
        }

        // Update health metrics
        self.health = ContextHealth::evaluate(
            self.budget.pressure(),
            self.compression_count,
            context.len(),
            self.budget.allocated,
            self.budget.hard_limit,
        );

        context
    }

    /// Force emergency consolidation.
    pub fn force_consolidation(&mut self, context: &mut ContextWindow) -> CompressionResult {
        self.budget.allocated = CompressionPipeline::estimate_tokens(&context.messages);
        let result = CompressionPipeline::emergency_consolidation(context, &mut self.budget)
            .unwrap_or(CompressionResult {
                before_tokens: 0,
                after_tokens: 0,
                ratio: 1.0,
                technique: "none".to_string(),
            });
        self.compression_count += 1;
        result
    }

    /// Get current health status.
    pub fn health_status(&self) -> HealthStatus {
        self.health.status()
    }
}

// ─── Tests ────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_counter() {
        let counter = TokenCounter::for_model("gpt-5");
        let tokens = counter.count_tokens("Hello, this is a test message");
        assert!(tokens > 0 && tokens < 20);
    }

    #[test]
    fn test_budget_profile() {
        let profile = BudgetProfile::Balanced;
        let alloc = profile.allocations();
        let total = alloc.system_prompt + alloc.goal_definition + alloc.active_task
            + alloc.tool_results + alloc.recent_history + alloc.memory_rag + alloc.project_context;
        assert!((total - 1.0).abs() < 0.01); // Should sum to ~1.0
    }

    #[test]
    fn test_context_budget() {
        let budget = ContextBudget::new(128_000, BudgetProfile::Balanced);
        assert_eq!(budget.hard_limit, 89_600); // 70% of 128k
        assert!(!budget.is_over_limit());
        assert_eq!(budget.remaining(), 89_600);
    }

    #[test]
    fn test_compression_pipeline() {
        let mut context = ContextWindow::new();

        // Add many messages to exceed budget
        for i in 0..20 {
            context.push(Message {
                role: "user".to_string(),
                content: format!("Message {} with some content to test compression", i),
            });
        }

        let mut budget = ContextBudget::new(1000, BudgetProfile::Balanced);
        budget.allocated = 1500; // Over budget

        let results = CompressionPipeline::compress(&mut context, &mut budget);
        assert!(!results.is_empty());
        assert!(budget.allocated <= budget.hard_limit);
    }

    #[test]
    fn test_emergency_consolidation() {
        let mut context = ContextWindow::new();
        for i in 0..50 {
            context.push(Message {
                role: "user".to_string(),
                content: format!("Long message number {} with detailed content for testing", i),
            });
        }

        let mut budget = ContextBudget::new(500, BudgetProfile::Balanced);
        budget.allocated = 2000;

        let result = CompressionPipeline::emergency_consolidation(&mut context, &mut budget);
        assert!(result.is_some());
        assert_eq!(context.len(), 1); // Only summary left
    }

    #[test]
    fn test_context_manager() {
        let mut manager = ContextManager::new(128_000, BudgetProfile::Balanced);
        let mut context = ContextWindow::new();

        for i in 0..10 {
            context.push(Message {
                role: "user".to_string(),
                content: format!("Test message {}", i),
            });
        }

        let prepared = manager.prepare(&mut context);
        assert!(prepared.len() > 0);
        assert_eq!(manager.health_status(), HealthStatus::Healthy);
    }

    #[test]
    fn test_health_status() {
        let health = ContextHealth::evaluate(0.5, 0, 10, 5000, 10000);
        assert_eq!(health.status(), HealthStatus::Healthy);

        let health = ContextHealth::evaluate(0.85, 0, 10, 8500, 10000);
        assert_eq!(health.status(), HealthStatus::Warning);

        let health = ContextHealth::evaluate(0.96, 0, 10, 9600, 10000);
        assert_eq!(health.status(), HealthStatus::Critical);
    }
}