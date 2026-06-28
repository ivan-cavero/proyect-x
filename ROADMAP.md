# Project-X — ROADMAP

> **30 weeks total** — 6 Phases, 16 Sprints, ~220 tasks  
> **Stack:** Rust nightly · Tauri v2 · Vue 3 · SQLite · Qdrant embedded · Ractor  
> **Distribution:** Single binary, zero external dependencies  
> **Enterprise:** Multi-tenant, RBAC, SSO, billing, audit (Phase 6)

---

## Table of Contents

- [Core Systems](#core-systems)
- [Phase 0: Foundation (Weeks 1–3)](#phase-0-foundation-weeks-1-3)
- [Phase 1: Smart Loop (Weeks 4–8)](#phase-1-smart-loop-weeks-4-8)
- [Phase 2: Multi-Agent (Weeks 9–13)](#phase-2-multi-agent-weeks-9-13)
- [Phase 3: Desktop + Dashboard (Weeks 14–19)](#phase-3-desktop--dashboard-weeks-14-19)
- [Phase 4: Memory & Self-Healing (Weeks 20–24)](#phase-4-memory--self-healing-weeks-20-24)
- [Phase 5: Production (Weeks 25–27)](#phase-5-production-weeks-25-27)
- [Phase 6: Enterprise (Weeks 28–30)](#phase-6-enterprise-weeks-28-30)
- [Appendix](#appendix)

---

## Legend

- `[ ]` — Task not started
- `[~]` — Task in progress
- `[x]` — Task completed
- **Bold** — Dependency
- ⚠ — Risk / Requires attention
- 🧪 — Validation milestone
- 🧠 — Context Management concern
- 🏢 — Enterprise concern

---

## Core Systems

### Context Window Management — The Spine of the System

This is not a feature. It is a **cross-cutting architectural concern** that affects every single agent call, every memory operation, every phase transition. It is designed so the agent **never reaches the model's maximum context window**.

```rust
// Core principle: never exceed 70% of model's max context
// Safety margin: 30% headroom for unexpected tool results

pub struct ContextBudget {
    pub model_max: usize,           // e.g. 128_000 for GPT-5
    pub hard_limit: usize,           // 70% of model_max = 89_600
    pub allocated: usize,            // Tokens currently allocated
    pub sections: Vec<ContextSection>,
}

pub struct ContextSection {
    pub name: SectionType,
    pub priority: u8,                // 0 (drop first) to 100 (keep at all costs)
    pub max_tokens: usize,
    pub current_tokens: usize,
    pub content: CompressedContent,
}

pub enum SectionType {
    SystemPrompt,        // Priority 100 — never dropped
    GoalDefinition,      // Priority 90 — the active goal
    ActiveTask,          // Priority 80 — current instruction
    ToolResults,         // Priority 70 — output from tool calls
    RecentHistory,       // Priority 60 — last N interactions
    MemoryRAG,           // Priority 50 — retrieved memory chunks
    ProjectContext,      // Priority 40 — project intelligence
    AgentState,          // Priority 30 — current agent variables
    Feedback,            // Priority 20 — pending feedback to process
}
```

**The Compression Pipeline (applied in order when budget is exceeded):**

```
1. Truncate Tool Results
   └─ Full output → summary only (via LLM summarizer)

2. Compress Recent History  
   └─ Each interaction → one-line summary
   └─ Then: group → paragraph summary
   └─ Then: all recent history → consolidated paragraph

3. Reduce RAG Chunks
   └─ K=10 → K=5 → K=3 → K=1 → drop

4. Prune Project Context
   └─ Remove low-relevance sections
   └─ Keep only current-goal-related context

5. Emergency Consolidation (EMC)
   └─ Full context dump → SummarizerAgent → structured summary
   └─ Clear everything except system prompt + summary

6. Model Upgrade (last resort)
   └─ Switch to model with larger context window
   └─ Or switch to more capable model that handles complexity better
```

**Per-Model Context Registry:**

```toml
[models.gpt-5]
context_window = 128000
hard_limit_pct = 70    # 89,600 tokens
budget = "balanced"    # predefined budget profile

[models.claude-4-opus]
context_window = 200000
hard_limit_pct = 75    # 150,000 tokens
budget = "generous"    # more memory, more RAG

[models.gemini-2.5-pro]
context_window = 1048576  # 1M tokens
hard_limit_pct = 60       # 628,000 — still safe
budget = "research"       # max context, for deep research tasks

[models.local-llama]
context_window = 32768
hard_limit_pct = 50        # 16,384 — very constrained
budget = "aggressive"      # aggressive compression
```

**Context Health Metrics (published as events):**

| Metric | What It Measures | Warning | Critical |
|--------|-----------------|---------|----------|
| `context_pressure` | % of hard_limit used | > 80% | > 95% |
| `compression_ratio` | original vs compressed | < 2x | < 1.5x |
| `compression_frequency` | times compressed in last 5 calls | > 3 | > 5 |
| `rag_relevance` | avg score of retrieved chunks | < 0.6 | < 0.4 |
| `history_fidelity` | how much detail is retained | < 50% | < 30% |

When critical pressure is detected: pause agent, force EMC, log event, notify dashboard.

---

### Enterprise Architecture (Multi-Tenant)

```
┌─────────────────────────────────────────────────────────────┐
│                    ORGANIZATION                              │
│  ┌─────────────────────────────────────────────────────────┐│
│  │  TEAM A                    TEAM B                       ││
│  │  ┌────────────────────┐   ┌────────────────────┐       ││
│  │  │ Project X          │   │ Project Z          │       ││
│  │  │ Project Y          │   │                     │       ││
│  │  └────────────────────┘   └────────────────────┘       ││
│  │                                                         ││
│  │  Users: Alice (admin)                                   ││
│  │         Bob (dev)                                       ││
│  │         Charlie (viewer)                                ││
│  └─────────────────────────────────────────────────────────┘│
│                                                             │
│  DATA ISOLATION:                                            │
│  - Each organization has isolated SQLite (or PG schema)     │
│  - Qdrant collections scoped by org_id                      │
│  - Memory never crosses org boundaries                      │
│  - Billing per org (token usage × seat count)               │
│  - Audit logs per org                                       │
└─────────────────────────────────────────────────────────────┘
```

---

### Workspace Intelligence Layer

Every project builds a **live intelligence model** while being developed:

```
Workspace Intelligence:
├── Tech Radar
│   ├── Dependencies (current + outdated)
│   ├── Vulnerabilities (auto-detected)
│   └── Upgrade suggestions
├── Architecture Map 
│   ├── Modules and their relationships
│   ├── Data flow diagrams (auto-generated)
│   └── Critical paths and bottlenecks
├── Evolution Log
│   ├── Every decision with rationale
│   ├── Auto-generated ADRs
│   ├── Live roadmap (updates on completion)
│   └── Changelog (auto-generated from commits)
├── Quality Metrics
│   ├── Test coverage per module (timeline)
│   ├── Cyclomatic complexity
│   ├── Technical debt estimation
│   └── Code churn (hotspots)
└── Knowledge Base
    ├── Cross-project decisions (learns from past)
    ├── Recurring patterns
    ├── Frequent errors (prevents recurrence)
    └── Team conventions
```

---

## Phase 0: Foundation (Weeks 1–3)

**Goal:** `project-x --help` works. An EchoAgent talks to an LLM and saves to SQLite.

### Sprint 0.1 — Workspace + Actor Model (Week 1)

#### Repository Setup

- [x] **Create workspace Cargo.toml** with all crate members
- [x] **Create rust-toolchain.toml** pinning to specific nightly
- [x] **Create .gitignore** (target, node_modules, .env, *.db, *.enc)
- [x] **Create .editorconfig** (indent: 4 spaces, charset: utf-8, eol: lf)
- [x] **Create .github/workflows/ci.yml** — `cargo test`, `cargo clippy`, `cargo fmt`
- [x] **Create .github/workflows/release.yml** — Build + upload binaries on tag
- [ ] **Verify `cargo build --workspace` compiles clean**
- [ ] **Verify `cargo clippy` passes with zero warnings**

#### Shared Types (`crates/shared`)

- [ ] **Define core type aliases:** `AgentId`, `SessionId`, `ProjectId`, `GoalId`, `TaskId`, `PhaseId`, `ConversationId`, `OrganizationId`, `TeamId`, `UserId`
- [ ] **Define `Priority` enum:** `Low`, `Normal`, `High`, `Critical`
- [ ] **Define `AgentMessage` struct:** `id`, `conversation_id`, `source`, `target`, `priority`, `timestamp`, `ttl`, `kind`
- [ ] **Define `MessageKind` enum:** all variants (task, result, info, health, phase, checkpoint, drift, token, tool, gate)
- [ ] **Define `SystemEvent` enum**
- [ ] **Define `Phase` enum:** `Idle`, `Planning`, `Researching`, `Designing`, `Implementing`, `Reviewing`, `Fixing`, `Testing`, `SecurityScan`, `Finalizing`, `Completed`, `Failed`, `Cancelled`
- [ ] **Define `Transition` struct:** `from`, `to`, `gate`, `condition`
- [ ] **Define `GateResult` struct:** `gate_name`, `passed`, `details`, `evaluator`
- [ ] **Define `Error` enum** with variants for each subsystem
- [ ] **🧠 Define `ContextSection` types** — system_prompt, goal, task, tool_results, history, memory, project_context, agent_state, feedback
- [ ] **🧠 Define `ContextBudget` struct** — model_max, hard_limit, allocated, sections
- [ ] **Implement `Serialize` + `Deserialize` for all types**

#### Actor Model (`crates/core/src/actor`)

- [ ] **Add ractor dependency**
- [ ] **Implement `Supervisor` actor** with hierarchical supervision (spawn, kill, restart, list)
- [ ] **Implement `EchoAgent` actor** for testing
- [ ] **Implement `ActorRef` wrapper** — send, call (with timeout), broadcast
- [ ] **Implement `GracefulShutdown`** — SIGINT/SIGTERM → complete iteration → checkpoint → flush → exit
- [ ] **Write integration test:** spawn, send, receive, kill, verify cleanup
- [ ] **Write stress test:** 1000 messages, no leak, no deadlock

#### Event Bus (`crates/core/src/bus`)

- [ ] **Implement `EventBus`** wrapping `tokio::sync::broadcast::Sender`
- [ ] **Implement `EventBusLog`** — subscribes and writes to tracing
- [ ] **Implement `EventBusMetrics`** — counts events by type

#### Validation Milestone 🧪

- [ ] **Demo:** EchoAgent receives message, echoes, logs event, graceful shutdown on Ctrl+C

---

### Sprint 0.2 — LLM Provider Trait + OpenAI (Week 1–2)

#### Provider Trait

- [ ] **Define `LLMProvider` trait:** `chat()`, `stream()`, `embed()`, `count_tokens()`, `model_info()`
- [ ] **🧠 Add `context_window()` to ModelInfo** — max tokens for this model
- [ ] **Define `ChatConfig`:** model, temperature, max_tokens, top_p, stop, penalties
- [ ] **Define `ChatMessage`:** role (System/User/Assistant/Tool), content, tool_calls, tool_call_id
- [ ] **Define `ChatResponse`:** content, finish_reason, usage, model
- [ ] **Define `StreamReceiver`:** `mpsc::Receiver<StreamChunk>`
- [ ] **Define `StreamChunk`:** `Delta(String)`, `Done(TokenUsage)`, `Error(Error)`
- [ ] **Define `TokenUsage`:** input_tokens, output_tokens, total_tokens

#### Provider Implementations

- [ ] **Implement `OpenAIProvider`** — chat, stream, embed, count_tokens, model_info
- [ ] **Implement `AnthropicProvider`** — Messages API, streaming
- [ ] **Implement `GeminiProvider`** — Gemini API, streaming
- [ ] **Implement `OllamaProvider`** — local models via REST
- [ ] **Implement `OpenAICompatibleProvider`** — configurable base_url, works with any OpenAI-compatible endpoint
- [ ] **Implement `MockProvider`** — deterministic, for testing

#### Provider Infrastructure

- [ ] **🧠 Implement per-model context registry** — model name → context_window, hard_limit_pct, budget profile
- [ ] **Implement `ProviderRouter`** — model name → correct provider
- [ ] **Implement `ModelTier`** — Fast, Balanced, Capable, Cheapest
- [ ] **Implement tier-based routing**
- [ ] **Implement `TokenCounter`** middleware — wraps provider, counts tokens, publishes events
- [ ] **Implement retry logic** — backoff + jitter on 429/500/503
- [ ] **Implement timeout** — configurable per request

#### Validation Milestone 🧪

- [ ] **Demo:** Provider sends "Say hello", receives streamed response, counts tokens, logs model info with context window

---

### Sprint 0.3 — SQLite + Event Store (Week 2)

#### Event Store Trait

- [ ] **Define `EventStore` trait:** append, read_events, get_snapshot, save_snapshot, list_aggregates

#### SQLite Implementation

- [ ] **Create migration V1:** `events` table (append-only, WAL mode)
- [ ] **Create migration V2:** `checkpoints` table
- [ ] **Create migration V3:** `sessions` table
- [ ] **Create migration V4:** `projects` table
- [ ] **Create migration V5:** `drift_history` table
- [ ] **🧠 Create migration V6:** `context_snapshots` table (snapshots of context before compression)
  ```sql
  CREATE TABLE context_snapshots (
      id              BLOB PRIMARY KEY,
      session_id      BLOB NOT NULL,
      iteration       INTEGER NOT NULL,
      budget_snapshot TEXT NOT NULL,    -- Full budget JSON
      compression_log TEXT NOT NULL,    -- What was compressed, ratios
      pressure_before REAL,
      pressure_after  REAL,
      created_at      TEXT NOT NULL DEFAULT (datetime('now'))
  );
  ```
- [ ] **Create migration V7:** `organizations` table (for multi-tenant)
- [ ] **Create migration V8:** `teams` table
- [ ] **Create migration V9:** `users` table (with auth fields)
- [ ] **Create migration V10:** `organization_members` table
- [ ] **Create migration V11:** `billing_records` table
- [ ] **Create migration V12:** `audit_logs` table
- [ ] **Implement `SqliteEventStore`** — WAL mode, busy_timeout, connection pool
- [ ] **Implement migration runner** — auto-run on init
- [ ] **Implement data isolation layer** — all queries scoped by org_id (even in SQLite, filter in WHERE)

#### Project Directory Structure

- [ ] **Define project directory layout:**
  ```
  my-project/
  ├── forge.toml
  ├── .forge/
  │   ├── state.db
  │   ├── memory/        # Qdrant storage
  │   └── sessions/
  └── src/
  ```
- [ ] **Implement `create_project(path)`** — creates structure, inits DB

#### Postgres Adapter (Stub)

- [ ] **Create `PgEventStore` struct** (implements EventStore, stub for now)
- [ ] **Define `PersistenceMode`:** Embedded (SQLite), Remote (Postgres)
- [ ] **Implement factory**

#### Validation Milestone 🧪

- [ ] **Demo:** `project-x init my-project` → SQLite created. Append 10 events, snapshot, recover.

---

### Sprint 0.4 — CLI v0.1 (Week 3)

#### CLI Framework

- [ ] **Add dependencies:** `clap` v4 (derive), `tracing`, `tracing-subscriber`, `indicatif`, `self_update`, `colored`
- [ ] **Define CLI structure with clap derive** (all commands)
  - `init`, `run`, `status`, `logs`, `session`, `config`, `provider`, `mcp`, `update`, `version`, `help`
  - 🏢 `org`, `team`, `user`, `billing`, `audit`
  - 🧠 `context` (inspect context budget, pressure, compression history)

#### Command: init

- [ ] **Implement `init`** — creates project, default forge.toml, initializes DB

#### Command: run

- [ ] **🧠 Implement context budget calculation on startup** — read model config, calculate limits
- [ ] **Implement `run`** — loads project, creates session, spawns core, streams output
- [ ] **Implement `--resume`** — reloads last session from checkpoint
- [ ] **Implement `--dry-run`** — shows workflow plan without executing
- [ ] **Implement `--headless`** — JSON output for CI/CD
- [ ] **Implement progress bar** for token streaming

#### Command: status / logs / session

- [ ] **Implement `status`, `logs`, `session list`, `session show`**

#### Command: config

- [ ] **Implement `config show`, `set`, `get`, `edit`, `import`, `export`**

#### 🧠 Command: context

- [ ] **Implement `context inspect <session>`** — shows current budget, pressure, compression stats
- [ ] **Implement `context history <session>`** — timeline of compressions, ratios
- [ ] **Implement `context force-compress <session>`** — manual EMC trigger

#### 🏢 Command: org / team / user

- [ ] **Implement `org create`, `org list`, `org show`**
- [ ] **Implement `team create`, `team list`, `team add-user`**
- [ ] **Implement `user invite`, `user list`, `user remove`**

#### Logging

- [ ] **Configure `tracing-subscriber`** — human-readable + JSON mode
- [ ] **🧠 Add context metrics to logs** — pressure, compression ratio at each LLM call

#### Self-Update

- [ ] **Implement `self_update`** — GitHub releases, checksum, signature, rollback

#### Validation Milestone 🧪

- [ ] **Demo:** Full CLI flow: init, run, resume, logs, context inspect, config edit

---

## Phase 0 — Completion Checklist

- [ ] All Sprint 0.1–0.4 tasks complete
- [ ] `cargo build --workspace` compiles on nightly, zero warnings
- [ ] `cargo test --workspace` passes
- [ ] End-to-end: init → run --goal "hello" → streaming → checkpoint → resume
- [ ] 🧠 Context budgeting stubs exist, model registry has context_window values

---

## Phase 1: Smart Loop (Weeks 4–8)

**Goal:** `project-x run --goal "..."` executes an agent with state machine, MCP tools, hot memory, **context budget enforcement**, limits, and divergence detection.

### Sprint 1.1 — State Machine + Loop Controller (Week 4)

#### State Machine

- [ ] **Implement `StateMachine`** — phase transitions, history tracking, cycle detection
- [ ] **Define all valid phase transitions**
- [ ] **Implement `PhaseHistory`** — detects A→B→A→B cycles

#### Gate System

- [ ] **Define `Gate` struct:** name, evaluator, max_retries
- [ ] **Define `GateEvaluator`:** AllAgentsPass, MajorityPass, NoCritical, NoFailures
- [ ] **Implement `GateEvaluator::evaluate()`**

#### Loop Controller

- [ ] **Implement `LoopController`** — main loop with phases, gates, checkpoints, events
- [ ] **🧠 Integrate ContextBudget check before each LLM call:**
  ```
  Before each agent.chat():
    1. budget = context_manager.calculate_budget(agent, model, phase)
    2. if budget.is_over_limit():
         context_manager.compress(budget)
         log("Context compressed: {before} → {after} tokens")
    3. if budget.pressure > 0.8:
         context_manager.schedule_compression()
    4. log context_pressure metric
    5. proceed with LLM call
  ```

#### Limits

- [ ] **Define `Limits` struct:** max_iterations, session_ttl, phase_timeout, tool_timeout
- [ ] **🧠 Add context limits:** `context_pressure_warning`, `context_pressure_critical`, `context_max_compression_ratio`
- [ ] **Implement `LimitsChecker`** — checks all limits before each iteration

#### Divergence Detector

- [ ] **Implement `DivergenceDetector`** — rolling hash window, oscillation detection, repetition detection

#### Validation Milestone 🧪

- [ ] **Demo:** State machine progresses through phases. Gate blocks on failure. Limits enforced. Context budget calculated.

---

### Sprint 1.2 — MCP Host + FileSystem Tool (Week 4–5)

#### JSON-RPC Protocol

- [ ] **Implement `JsonRpcMessage`** — Request, Response, Error, Notification
- [ ] **Implement serialize/deserialize** (spec-compliant)
- [ ] **Define MCP messages:** initialize, tools/list, tools/call, resources/list
- [ ] **Implement auto-incrementing request IDs**

#### Transport Layer

- [ ] **Implement `StdioTransport`** — spawn process, read stdout, write stdin, disconnect with SIGTERM
- [ ] **Define `Transport` trait:** send, receive, connect, disconnect, is_connected
- [ ] **Implement `SseTransport`** (stub for remote MCPs)

#### MCP Host

- [ ] **Implement `McpHost`** — add/remove servers, start_all, shutdown, negotiate, tool registry
- [ ] **Implement `McpServerHandle`** — connection, capabilities, tools cache, health, reconnection

#### Tool Registry

- [ ] **Define `McpTool`:** name, description, input_schema, server_name
- [ ] **Define `ToolRegistry`** — thread-safe map, register, unregister, get, list
- [ ] **Define unified `Tool` trait** — name, description, input_schema, execute, cost_estimate
- [ ] **Implement `McpToolWrapper`** — adapts McpTool to Tool trait
- [ ] **Implement `AggregateToolRegistry`** — merges MCP + WASM + Native tools

#### FileSystem MCP Server

- [ ] **Create MCP server binary** (separate crate)
- [ ] **Implement tools:** read_file, write_file, edit_file, list_files, create_directory, file_info, search_files
- [ ] **Implement path sandboxing:** only project directory, reject traversal, reject symlinks outside
- [ ] **Implement security:** block `.env`, `.ssh`, hidden system files

#### Validation Milestone 🧪

- [ ] **Demo:** MCP host starts Filesystem server, negotiates, agent calls tools. Path traversal blocked.

---

### Sprint 1.3 — Hot Memory + Cache (Week 5–6)

#### Hot Memory

- [ ] **Implement `HotMemory`** — DashMap for sessions, contexts, variables
- [ ] **Define `SessionState`:** id, project_id, status, goal, phase, iteration
- [ ] **Define `Interaction`:** timestamp, role, content, token_count, tool_calls
- [ ] **Implement methods:** create_session, get/update, push_interaction, get_context, variables

#### Sliding Window

- [ ] **🧠 Implement context-aware sliding window:**
  - Default: 50 interactions or 70% of model context — whichever is smaller
  - Dynamic window: adjust max interactions based on average token cost per interaction
  - Auto-compress: when window exceeds 50% budget, compress oldest interactions
- [ ] **Implement window stats:** current_size, max_size, total_tokens

#### LLM Cache

- [ ] **Implement `LLMCache` with moka** — LRU, TTL, max size
- [ ] **Implement cache middleware** — wraps provider, checks cache before LLM call
- [ ] **Write tests:** 2nd same prompt = cache hit, different prompts = miss

#### Event Bus (Refine)

- [ ] **🧠 Add context events:** `context/budget_updated`, `context/compression_applied`, `context/pressure_alert`
- [ ] **Implement `LogSubscriber`, `MetricsSubscriber`, `WebSocketBridge` (stub)**

#### Validation Milestone 🧪

- [ ] **Demo:** 100 interactions, sliding window auto-compresses at 50% budget. Cache saves tokens.

---

### Sprint 1.4 — Context Window Management (Week 6)

**🧠 This is the most important sprint in Phase 1.**

#### Token Counter Service

- [ ] **Implement `TokenCounterService`:**
  - `count_message(msg) -> usize` — counts tokens for a single message
  - `count_messages(msgs) -> usize` — counts for a batch
  - `count_tool_result(result) -> usize` — counts tool output tokens
  - Uses `tiktoken-rs` or native tokenizer per model
- [ ] **Implement budget calculation:**
  - `calculate_budget(model, sections) -> ContextBudget`
  - Allocates token budget per section type using model's hard_limit
  - Returns budget with pressure percentage

#### Budget Profiles

- [ ] **Define budget profiles:**
  ```rust
  pub struct BudgetProfile {
      pub system_prompt_pct: f32,     // 5%
      pub goal_definition_pct: f32,   // 5%  
      pub active_task_pct: f32,       // 15%
      pub tool_results_pct: f32,      // 10%
      pub recent_history_pct: f32,    // 35%
      pub memory_rag_pct: f32,        // 25%
      pub project_context_pct: f32,   // 5%
  }
  ```
- [ ] **Implement profiles:** `balanced` (default), `generous` (more memory), `aggressive` (tight), `research` (max RAG)

#### Compression Pipeline

- [ ] **Implement compression steps (in order):**
  - `truncate_tool_results(results, budget)` — keep only summaries of recent tool calls
  - `compress_history(history, budget)` — summarizer on the fly per group of 5 interactions
  - `reduce_rag(k, budget)` — K=10 → K=5 → K=3 → K=1 → drop
  - `prune_project_context(context, budget)` — keep only current-goal-relevant sections
  - `emergency_consolidation(full_context)` — SummarizerAgent produces structured summary, clear everything
- [ ] **Each compression step returns:** `CompressionResult { before_tokens, after_tokens, ratio, technique }`
- [ ] **Implement compression chain:** run steps in order until `pressure < 0.7`
- [ ] **Implement `CompressionLogger`** — every compression is logged with full details

#### Context Manager

- [ ] **Implement `ContextManager` orchestrator:**
  ```rust
  pub struct ContextManager {
      pub token_counter: TokenCounterService,
      pub budget_profile: BudgetProfile,
      pub max_pressure: f32,           // 0.7 (70%)
      pub compression_history: Vec<CompressionEvent>,
  }
  
  impl ContextManager {
      /// Called before each LLM call
      pub fn prepare_context(&mut self, agent: &AgentState, model: &ModelInfo) -> PreparedContext {
          let budget = self.calculate_budget(model);
          let pressure = self.measure_pressure(&budget);
          
          if pressure > self.max_pressure {
              let result = self.compress(&mut budget, pressure);
              self.log_compression(&result);
          }
          
          self.build_prompt(&budget, agent)
      }
      
      /// Emergency: called when pressure > 0.95
      pub fn force_emergency_consolidation(&mut self) -> ConsolidationResult {
          // Full EMC: summarize everything, clear context, inject summary
      }
  }
  ```

#### Context Health Metrics

- [ ] **Implement metrics collection:** pressure, compression_ratio, compression_frequency, rag_relevance, history_fidelity
- [ ] **Publish context events on every compression**
- [ ] **Define alert thresholds:** pressure > 0.8 = warning, > 0.95 = critical

#### Context Snapshotting

- [ ] **Implement context snapshots** — before each compression, save full state to SQLite
- [ ] **Implement context diff** — compare before/after compression, show what was removed

#### Validation Milestone 🧪

- [ ] **Demo:** Agent runs with 64k context budget. After 30 interactions, pressure hits 80%. Compression pipeline activates: truncates tool results → compress history → reduce RAG. Pressure drops to 50%. LLM call proceeds with compact context. Never exceeds 70% of model max.

---

### Sprint 1.5 — Drift Detection + Limits Enforcement (Week 6–7)

#### Drift Guard Base

- [ ] **Implement `DriftGuard`** — collects metrics per iteration window
- [ ] **Implement `MetricsCollector`:** latency, errors, tool calls, output length, gate results
- [ ] **Calculate baseline** from first 10 iterations
- [ ] **Calculate current vs baseline** — z-score for each metric
- [ ] **🧠 Add context metrics to drift:** pressure trend, compression frequency, history_fidelity
- [ ] **Publish `DriftAlert` events**

#### Limits Enforcer

- [ ] **Implement `LimitsEnforcer`** — integrated into LoopController
- [ ] **🧠 Add context limits enforcement:** if pressure > 0.95, force EMC. If compression frequency > 5 in last 10 calls, escalate to supervisor.

#### Phase Timeout

- [ ] **Implement phase timeout tracking and rollback**

#### Recovery Actions

- [ ] **Define `RecoveryAction`:** LogOnly, ForceConsolidation, ContextReset, ModelUpgrade, PauseAgent, KillSession
- [ ] **🧠 Add emergency recovery:** when context_pressure > 0.95, trigger ForceConsolidation. If that doesn't work, ModelUpgrade to larger context.

#### Validation Milestone 🧪

- [ ] **Demo:** Agent degrades → drift detected → recovery action triggered. Context pressure hits 95% → emergency EMC → pressure drops.

---

### Sprint 1.6 — CLI v0.2 (Week 7–8)

#### Enhanced Commands

- [ ] **Implement `run --resume`** — find last session, restore state from checkpoint, continue loop
- [ ] **Implement `run --dry-run`** — show workflow plan (agents, models, phases, estimated tokens, context budget)
- [ ] **🧠 Add context budget to dry-run:** "Estimated token usage per phase: Design 12k, Code 45k, Review 20k, Security 15k. Peak pressure: 65% of 128k (model: gpt-5). Budget profile: balanced."
- [ ] **Implement `run --headless`** — JSON lines output
- [ ] **Implement `logs --tail`** — subscribe to EventBus, filterable
- [ ] **🧠 Implement `context inspect`** — show current budget allocation per section, pressure, compression history
  ```
  $ project-x context inspect --session abc
  ┌─────────────────────────────────────────────┐
  │  CONTEXT BUDGET — Session abc               │
  │  Model: gpt-5 (128k max, 89,600 hard limit) │
  │  Profile: balanced                          │
  │                                              │
  │  Section              Tokens    %Budget     │
  │  System Prompt          4,200      4.7%      │
  │  Goal Definition        2,100      2.3%      │
  │  Active Task            6,800      7.6%      │
  │  Tool Results          12,400     13.8%      │
  │  Recent History        34,200     38.2%  ←   │
  │  Memory (RAG)          18,900     21.1%      │
  │  Project Context        3,400      3.8%      │
  │  ─────────────────────────────────         │
  │  TOTAL                 81,500     91.0%      │
  │  PRESSURE: 🔴 CRITICAL (91%)                │
  │                                              │
  │  Compression History:                        │
  │  10m ago: truncate_tool_results 45k→12k     │
  │  15m ago: compress_history 68k→34k          │
  │  25m ago: reduce_rag K=10→K=5              │
  │  30m ago: emergency_consolidation 85k→12k   │
  └─────────────────────────────────────────────┘
  ```
- [ ] **🧠 Implement `context force-compress <session>`** — trigger EMC manually

#### TUI Monitor

- [ ] **Implement `monitor` command** with ratatui multi-panel
- [ ] **🧠 Add context panel:** pressure gauge, section breakdown, compression frequency
- [ ] **Add panels:** status, agents, logs, tokens, phases

#### Validation Milestone 🧪

- [ ] **Demo:** Full CLI: run with context management visible. `context inspect` shows budget. `context force-compress` triggers EMC. Monitor shows live context pressure.

---

## Phase 1 — Completion Checklist

- [ ] All Sprint 1.1–1.6 tasks complete
- [ ] 🧠 Context budget enforcement works: never exceed 70% of model max
- [ ] 🧠 Compression pipeline: truncate → compress → reduce → prune → emergency
- [ ] 🧠 Context snapshots saved before each compression
- [ ] State machine, gates, limits all work
- [ ] MCP host + Filesystem tool functional
- [ ] Hot memory + cache functional
- [ ] Drift detection + basic recovery
- [ ] CLI v0.2 with context commands

---

## Phase 2: Multi-Agent (Weeks 9–13)

**Goal:** Multiple agents with different models, **context-aware per agent**, cross-model verification.

### Sprint 2.1 — Orchestrator + Role System (Week 9)

#### Orchestrator Actor

- [ ] **Implement `Orchestrator` actor:** receives goals, spawns agents, manages phases, collects results, evaluates gates
- [ ] **Implement agent lifecycle:** spawn_role, kill_role, restart_role, list_roles
- [ ] **Implement task dispatch:** assign_task, collect_results, handle_timeout

#### Role System

- [ ] **Define `RoleConfig`:** name, description, model, temperature, max_tokens, system_prompt, tools
- [ ] **🧠 Per-agent context configuration:**
  ```toml
  [roles.coder]
  model = "gpt-5"
  context_profile = "balanced"       # Each role can have different budget profile
  context_priority = "normal"        # "low" | "normal" | "high" (affects compression order)
  
  [roles.architect]
  model = "claude-4-opus"
  context_profile = "generous"       # More memory for architectural reasoning
  context_priority = "high"
  
  [roles.security]
  model = "claude-4-haiku"
  context_profile = "aggressive"     # Security doesn't need full history
  context_priority = "normal"
  ```
- [ ] **Implement role validation** — required fields, model exists
- [ ] **Implement per-role ContextManager** — each agent type has its own budget profile

#### Goal → Workflow Mapping

- [ ] **Define `GoalConfig`:** name, agents, gates, max_iterations, overrides, workflow
- [ ] **Define `RoleOverride`:** model, temperature, system_prompt, context_profile
- [ ] **Implement goal resolution** — lookup → merge with defaults → validate

#### Agent Spawner

- [ ] **Implement `AgentSpawner`** — creates actor with correct config, binds tools
- [ ] **🧠 Per-agent context initialization** — each agent gets its own ContextManager with its profile

#### Message Priority System

- [ ] **Implement priority queue** — Critical > High > Normal > Low
- [ ] **🧠 Context injection has CRITICAL priority** — mid-loop instructions bypass compression

#### Validation Milestone 🧪

- [ ] **Demo:** Orchestrator spawns 3 agents with different models and context profiles. Each agent manages its own context budget independently.

---

### Sprint 2.2 — Base Agents (Week 9–10)

#### Agent Trait

- [ ] **Define `Agent` trait:** role(), system_prompt(), execute(), handle_feedback(), reset_context()
- [ ] **Define `Task`, `TaskOutput`**

#### Architect Agent

- [ ] **Implement `ArchitectAgent`** — generates ADR, uses filesystem + web_search
- [ ] **🧠 Context profile: `generous`** — needs full project context + memory
- [ ] **Implement ADR validation** (JSON Schema)

#### Coder Agent

- [ ] **Implement `CoderAgent`** — generates code, compiles, iterates on errors
- [ ] **🧠 Context profile: `balanced`** — needs recent history + tool results
- [ ] **Implement compilation check** — `cargo check`, parse errors

#### Reviewer Agent

- [ ] **Implement `ReviewerAgent`** — analyzes code, reports pass/fail with comments
- [ ] **🧠 Context profile: `aggressive`** — only needs diff + current file
- [ ] **Define `ReviewComment`, `ReviewReport`**

#### System Prompt Injection

- [ ] **Implement prompt assembly** — system_prompt + role defaults + project context
- [ ] **🧠 Inject context budget info into system prompt** — tell the agent its context limits:
  ```
  "Your context budget is 62,720 tokens (70% of 89,600 hard limit).
   Current usage: 45,200 tokens (72%).
   If budget is exceeded, recent history will be compressed.
   Keep responses concise to avoid compression."
  ```

#### Validation Milestone 🧪

- [ ] **Demo:** Goal → Architect designs → Coder implements → Reviewer approves. Each agent operates within its context budget.

---

### Sprint 2.3 — Cross-Model Verification (Week 10–11)

#### Parallel Agent Execution

- [ ] **Implement `JoinSet`-based parallel execution**
- [ ] **🧠 Synchronize context across parallel reviews** — each reviewer gets same code diff, independently analyzed
- [ ] **Implement `parallel_reviewers` config** — spawn N instances

#### Consensus Consolidation

- [ ] **Implement `ConsensusConsolidator`** — all_pass, majority, weighted, escalate
- [ ] **Write tests:** 2 pass 1 fail = majority pass, 1 pass 2 fail = fail

#### Cross-Model Feedback Loop

- [ ] **Implement feedback generation** — consolidate reviewer comments, send to Coder
- [ ] **Track feedback cycles** per gate, per agent, per phase

#### Agent-Specific Drift

- [ ] **🧠 Per-agent context metrics** — track context_pressure per agent, not just global
- [ ] **If context_pressure critical for one agent** — trigger per-agent EMC (doesn't affect others)

#### Validation Milestone 🪜

- [ ] **Demo:** 2 reviewers with different models, parallel analysis, consensus, feedback loop.

---

### Sprint 2.4 — Mid-Loop Injection (Week 11)

**🧠 This enables changing agent behavior WHILE the loop is running.**

#### Injection Channel

- [ ] **Implement `InjectionChannel`** — broadcast channel for mid-loop instructions
- [ ] **Implement message types:** `Instruction`, `Context`, `Correction`, `Halt`, `Approval`
- [ ] **Each agent checks for pending injections before each LLM call:**
  ```rust
  // In agent's execute():
  async fn execute(&mut self, task: Task) -> Result<TaskOutput> {
      // 1. Check for pending injections
      let injections = self.injection_channel.receive_pending().await;
      for injection in injections {
          self.context.inject(injection.message, injection.priority);
      }
      
      // 2. Context budget check (normal)
      let context = self.context_manager.prepare_context(&self.state, &self.model);
      
      // 3. Build prompt with injections at top
      // 4. Call LLM
      // 5. Return result
  }
  ```
- [ ] **Injections have CRITICAL priority** — they are NEVER compressed or dropped
- [ ] **Injections persist** in event store for audit

#### CLI Injection

- [ ] **Implement `inject` command:**
  ```bash
  project-x inject --session abc --agent coder --type instruction \
    --message "Usa thiserror en vez de anyhow para los errores de dominio"
  
  project-x inject --session abc --agent all --type correction \
    --message "Paren, hay un bug crítico en producción. Cambien prioridad a: fix issue #142"
  
  project-x inject --session abc --agent orchestrator --type halt \
    --message "Cambia el goal a: implementar rate limiting en los endpoints POST"
  ```

#### Dashboard Injection

- [ ] **Implement Prompt Injection panel** (Vue component)
  - Target selector: orchestrator or specific agent
  - Type selector: instruction, context, correction, halt, approval
  - Priority override (default: HIGH)
  - Message input + send
  - History of all injections for the session

#### Injection Audit

- [ ] **Log all injections** with: timestamp, user, session, agent, type, message
- [ ] **Show injection history** in session details

#### Validation Milestone 🧪

- [ ] **Demo:** Agent coding → inject "usa thiserror" → agent receives instruction in next iteration → changes approach. Inject "stop, change goal" → agent pauses current task, starts new goal.

---

### Sprint 2.5 — Security + Tester Agents (Week 12)

#### Security Agent

- [ ] **Implement `SecurityAgent`** — pattern detection (secrets, unsafe, injection, dangerous imports)
- [ ] **🧠 Context profile: `aggressive`** — only needs code diff, no history
- [ ] **Implement dependency scanning** — Cargo.toml vs vulnerability DB
- [ ] **Define `SecurityFinding`, `SecurityReport`**

#### Tester Agent

- [ ] **Implement `TesterAgent`** — test generation, execution, coverage estimation
- [ ] **🧠 Context profile: `balanced`** — needs function implementation + related code
- [ ] **Test execution:** `cargo test`, parse output, retry on failure

#### Gates

- [ ] **Implement `SecurityGate`** — `NoCriticalFailures`: zero critical findings = pass
- [ ] **Implement `TestGate`** — `AllTestsPass`: all pass + coverage > threshold = pass

#### Validation Milestone 🧪

- [ ] **Demo:** Coder writes code with hardcoded password → Security blocks → fix → tests pass → gate passes.

---

### Sprint 2.6 — Git Integration (Week 12–13)

#### Git MCP Server

- [ ] **Create MCP server binary** — init, add, commit, branch, checkout, diff, log, push, pull
- [ ] **Implement commit message generation** via LLM (conventional commits)
- [ ] **Implement branching strategy:** `feature/session-{id}`

#### Git Agent

- [ ] **Implement `GitAgent`** — orchestrates commits, branches, PRs
- [ ] **Pre-commit security check** — Security scans diff before commit

#### GitHub MCP Server

- [ ] **Create MCP server binary** — PR create, list, comment; issue create; check runs
- [ ] **Implement PR description generation** from agent reports

#### Validation Milestone 🧪

- [ ] **Demo:** Full session → Coder writes → Git commits → Security passes → push → PR with description.

---

### Sprint 2.7 — CLI v0.3 (Week 13)

#### Command Enhancements

- [ ] **Implement `run --goal <name>`** — lookup by name from forge.toml
- [ ] **Implement `run --agents coder,reviewer`** — inline override
- [ ] **Implement `run --agent.coder.model claude-4-opus`** — inline model override
- [ ] **🧠 Implement `--context-profile <profile>`** — override context budget profile
- [ ] **Implement `inject` command**
- [ ] **Implement `project list`, `project show`**

#### TUI Multi-Panel

- [ ] **Enhanced TUI:** pipeline, agents, tokens, logs, gates
- [ ] **🧠 Add context panel:** pressure gauge per agent, compression history

#### Validation Milestone 🧪

- [ ] **Demo:** `run --goal full-feature --context-profile generous --agent.coder.context_profile aggressive`

---

## Phase 2 — Completion Checklist

- [ ] All Sprint 2.1–2.7 tasks complete
- [ ] Multi-agent orchestration with per-context management
- [ ] Cross-model verification + consensus
- [ ] 🧠 Mid-loop injection functional (CLI + dashboard)
- [ ] Security + Tester agents with gates
- [ ] Git + GitHub integration
- [ ] CL v0.3 with context profile overrides

---

## Phase 3: Desktop + Dashboard (Weeks 14–19)

**Goal:** `project-x desktop` opens Tauri app. Dashboard shows everything in real time, including context health.

### Sprint 3.1 — Backend API (Week 14)

#### HTTP Server

- [ ] **Add dependencies:** `axum`, `tower-http`, `tokio-tungstenite`
- [ ] **Implement `ApiServer`** — start on configurable port, graceful shutdown

#### REST Endpoints

- [ ] `GET /api/health`
- [ ] `GET/POST /api/projects`
- [ ] `GET/DELETE /api/projects/:id`
- [ ] `POST /api/projects/:id/run`
- [ ] `POST /api/projects/:id/stop`
- [ ] `GET /api/projects/:id/sessions`
- [ ] `GET /api/sessions/:id`
- [ ] `GET /api/sessions/:id/events`
- [ ] `GET /api/sessions/:id/logs`
- [ ] `GET /api/sessions/:id/context` — **🧠 context budget + pressure + compression history**
- [ ] `POST /api/sessions/:id/inject` — **🧠 mid-loop injection via API**
- [ ] `POST /api/sessions/:id/compress` — **🧠 force compression**
- [ ] `GET /api/metrics/tokens`
- [ ] `GET /api/metrics/drift`
- [ ] `GET /api/metrics/context` — **🧠 aggregate context metrics across sessions**
- [ ] `GET /api/metrics/summary`

#### 🧠 Context-specific Endpoints

- [ ] `GET /api/sessions/:id/context` — full budget snapshot
- [ ] `GET /api/sessions/:id/context/history` — compression timeline
- [ ] `GET /api/sessions/:id/context/agents` — per-agent context breakdown
- [ ] `POST /api/sessions/:id/context/force-compress` — manual EMC trigger

#### WebSocket Endpoint

- [ ] `WS /ws/projects/:id` — real-time events
- [ ] `WS /ws/global` — all events
- [ ] **🧠 Add context events to WS stream** — `context/pressure_update`, `context/compression`, `context/alert`

#### Authentication

- [ ] **Implement local auth** — first-run password, JWT tokens, 24h expiry

#### 🏢 Enterprise Auth (Stub)

- [ ] **Define `OrgAuthMiddleware`** — scopes data access by organization
- [ ] **Implement token with org_id claim**

#### Validation Milestone 🧪

- [ ] **Demo:** REST endpoints return context data. WS streams context events. Auth works.

---

### Sprint 3.2 — Vue 3 Dashboard (Week 15–17)

#### Scaffold

- [ ] **Create Vite + Vue 3 + TypeScript project**
- [ ] **Add dependencies:** pinia, vue-router, tailwindcss, vue-flow, apexcharts
- [ ] **Set up project structure:** router, stores, composables, components, views

#### Core Components

- [ ] **PipelineView.vue** — Vue Flow graph, phase nodes with colors, animated edges
- [ ] **TokenChart.vue** — real-time area chart, breakdown by agent/model, budget bar
- [ ] **AgentHealth.vue** — table with model, status, ASI, latency, error rate, reset button
- [ ] **MemoryExplorer.vue** — semantic search, results, graph view
- [ ] **PromptInjection.vue** — target selector, type selector, message input, send, history
- [ ] **ProjectConfig.vue** — TOML editor with syntax highlighting, validation
- [ ] **SessionList.vue** — session history with status, duration, tokens
- [ ] **EventLog.vue** — filterable event stream
- [ ] **SystemStatus.vue** — overall health, uptime, version

#### 🧠 Context Dashboard Components

- [ ] **ContextPressureGauge.vue** — circular gauge, 0–100%, color zones, for each agent
  ```
  ┌──────────────────────────────────────┐
  │  CONTEXT PRESSURE                    │
  │                                      │
  │        ┌──── 82% ────┐              │
  │        │    🔴       │              │
  │        │   HIGH      │              │
  │        └────────────┘              │
  │                                      │
  │  Agent         Pressure  Compress   │
  │  Coder (GPT-5)   82%      3x/10m   │
  │  Reviewer (Cl)   34%      0x/10m   │
  │  Security (Haiku) 12%     0x/10m   │
  └──────────────────────────────────────┘
  ```
- [ ] **CompressionTimeline.vue** — time-series chart of compression events
  ```
  Compression Events (last 30 min):
  ● = truncate_tool_results  │  ▲ = compress_history
  ■ = reduce_rag             │  ⚡ = emergency_consolidation
  
  30m    25m    20m    15m    10m    5m    now
  │      │      │      │      │      │     │
  ⚡─────●──────▲──────■──────●──────▲─────●
  ```
- [ ] **ContextBudgetBreakdown.vue** — stacked bar per section
  ```
  CONTEXT BUDGET (GPT-5: 128k max, 89,600 limit)
  
  System   ████░░░░░░░░░░░░░░░░░░░░  4,200
  Goal     ██░░░░░░░░░░░░░░░░░░░░░░  2,100
  Task     ██████░░░░░░░░░░░░░░░░░░  6,800
  Tools    ████████████░░░░░░░░░░░░ 12,400
  History  █████████████████████████ 34,200  ◄
  Memory   █████████████████░░░░░░░ 18,900
  Project  ███░░░░░░░░░░░░░░░░░░░░░  3,400
  ─────────────────────────────────────────
  TOTAL    █████████████████████████░░ 82% 🔴
  ```
  
#### 🏢 Enterprise Dashboard Components

- [ ] **OrgSwitcher.vue** — dropdown to switch organizations (for multi-tenant)
- [ ] **TeamOverview.vue** — team members, projects, token usage
- [ ] **BillingDashboard.vue** — current spend, monthly trend, per-project breakdown
- [ ] **AuditLogViewer.vue** — searchable event log per org
- [ ] **UserManagement.vue** — invite, roles, remove users

#### WebSocket Integration

- [ ] **Implement `useWebSocket`** — auto-reconnect, exponential backoff, event buffering
- [ ] **Connect all stores to WS events**

#### Responsive Layout

- [ ] **Grid layout:** 3-col desktop, 2-col tablet, 1-col mobile

#### Validation Milestone 🧪

- [ ] **Demo:** Dashboard shows pipeline, tokens, agents, context pressure, compression timeline. Inject message via panel. See it arrive at agent.

---

### Sprint 3.3 — Tauri v2 Desktop (Week 17–19)

#### Scaffold

- [ ] **Initialize Tauri v2 project** — window 1280x800, title "Project-X"
- [ ] **Move core to library crate** — `core/src/lib.rs` exports public API

#### Core Integration

- [ ] **Implement `AppState`** — holds core runtime, event bus, hot memory, event store, provider registry
- [ ] **Initialize core in Tauri setup**
- [ ] **Start API server on random port, pass port to frontend**

#### Tauri Commands

- [ ] `run_agent`, `stop_session`, `get_projects`, `get_sessions`, `get_session`
- [ ] `get_metrics`, `send_prompt`, `reset_agent`, `get_config`, `save_config`
- [ ] **🧠 `get_context(session_id)`** — return current context budget for session
- [ ] **🧠 `force_compress(session_id)`** — trigger EMC
- [ ] **🧠 `inject_message(session, agent, type, message)`** — mid-loop injection

#### Tauri Events

- [ ] **Bridge EventBus to Tauri events** — emit all system events to frontend
- [ ] **🧠 Emit context events** — pressure updates, compression events

#### Tauri Plugins

- [ ] **Configure `tauri-plugin-store`** — credential storage (encrypted)
- [ ] **Configure `tauri-plugin-updater`** — auto-update from GitHub releases
- [ ] **Configure `tauri-plugin-dialog`** — file dialogs
- [ ] **Configure `tauri-plugin-shell`** — spawn MCP processes

#### Offline-First + VPS Sync

- [ ] **Implement embedded mode** — SQLite, no external services
- [ ] **Implement VPS sync toggle** — push/pull projects to/from remote

#### System Tray

- [ ] **Implement tray icon** — show/hide, new session, settings, quit
- [ ] **Dynamic icon** — changes color based on active session status

#### Menus

- [ ] **File:** New Project, Open Project, Import/Export Config, Quit
- [ ] **Edit:** Preferences
- [ ] **View:** Dashboard, Pipeline, Console, Memory, Context
- [ ] **🏢 Organization:** Switch Org, Team Settings, Billing, User Management
- [ ] **Help:** About, Documentation, Check for Updates

#### Validation Milestone 🧪

- [ ] **Demo:** Tauri desktop opens. Create project, run goal, see pipeline. Inspect context budget. Inject message. See compression events live.

---

### Sprint 3.4 — CLI v1.0 (Week 19)

#### Commands

- [ ] **Implement `desktop`** — launches Tauri app
- [ ] **Implement `dashboard`** — starts HTTP server, opens browser
- [ ] **Implement `context inspect`, `context history`, `context force-compress`**
- [ ] **Implement `inject`**

#### Deploy Commands

- [ ] **Implement `deploy setup`** — SSH + Docker Compose on VPS
- [ ] **Implement `deploy push`** — sync project to VPS
- [ ] **Implement `deploy status`** — remote health check
- [ ] **Implement `deploy logs`** — remote log streaming

#### 🏢 Organization Commands

- [ ] **Implement `org create`, `org list`, `org show`, `org delete`**
- [ ] **Implement `team create`, `team add-user`, `team remove-user`, `team list`**
- [ ] **Implement `user invite`, `user list`, `user remove`**
- [ ] **Implement `billing show`, `billing invoices`**

#### Full Documentation

- [ ] **Document all commands in clap's long_help**

#### Validation Milestone 🧪

- [ ] **Demo:** Full CLI v1.0 with org commands, context commands, deploy, inject.

---

## Phase 3 — Completion Checklist

- [ ] All Sprint 3.1–3.4 tasks complete
- [ ] REST API + WebSocket with context endpoints
- [ ] Vue 3 dashboard with context pressure, compression timeline, budget breakdown
- [ ] 🏢 Enterprise UI components (org switcher, billing, audit)
- [ ] Tauri desktop with core embedded
- [ ] All CLI commands including context and org management
- [ ] Mid-loop injection from both CLI and dashboard

---

## Phase 4: Memory & Self-Healing (Weeks 20–24)

**Goal:** The system remembers across sessions, detects degradation, **auto-manages context**, auto-recovers.

### Sprint 4.1 — Qdrant Embedded + RAG (Week 20)

#### Qdrant Integration

- [ ] **Add dependency:** `qdrant-client` (embedded mode)
- [ ] **Configure Qdrant local mode** — `Qdrant::local(path)`
- [ ] **Initialize collection** — `embeddings`, cosine distance, 1536 dimensions
- [ ] **Implement methods:** upsert_point, search, delete_points, collection_info, optimize

#### Embedding Service

- [ ] **Implement `EmbeddingService`** — wraps LLM provider embed(), batch processing, caching
- [ ] **Implement chunking strategies:** by size (512 tokens), by structure (paragraphs), by conversation turn

#### RAG Integration

- [ ] **Implement `RetrievalAugmentedContext`**:
  - Before each agent call: search memory for relevant chunks
  - Inject top-K into context as "Relevant Context" section
  - K is **dynamically adjusted based on context budget**: more budget = more RAG
- [ ] **🧠 Dynamic K calculation:**
  ```rust
  fn calculate_k(&self, available_budget: usize, chunk_size: usize) -> usize {
      // Each chunk is ~chunk_size tokens + overhead
      // Reserve max 25% of budget for RAG
      let max_rag_budget = available_budget * 0.25;
      let max_k = (max_rag_budget / (chunk_size + OVERHEAD)) as usize;
      max_k.clamp(1, 20)  // Between 1 and 20 chunks
  }
  ```

#### MemoryKeeper Actor

- [ ] **Implement `MemoryKeeper`** — background indexing on configurable interval
- [ ] **🧠 Index context snapshots too** — compression results are indexed for future reference

#### Validation Milestone 🧪

- [ ] **Demo:** Session 1 → indexed. Session 2 asks "what did we decide?" → RAG retrieves → agent references past decisions.

---

### Sprint 4.2 — DriftGuard v2 (Full ASI) (Week 21)

#### 8-Dimension ASI

- [ ] **Implement all 8 metric collectors:**
  - GoalAlignment, ToolUsageConsistency, ErrorRate, OutputLengthStability
  - ResponseTimeStability, OutputUniqueness, GatePassRate, TokenEfficiency
- [ ] **🧠 Add context health as dimension #9:** context_pressure_trend, compression_frequency
- [ ] **Each metric implements `Metric` trait**

#### ASI Calculator

- [ ] **Implement `ASICalculator`** — weighted sum, breakdown, configurable weights
- [ ] **🧠 Context dimension weight: 10%** — high compression frequency penalizes ASI

#### Baseline Profile

- [ ] **Establish baseline from first 10 iterations**
- [ ] **🧠 Context baseline:** normal pressure (~40-50%), normal compression (~1x per 20 iterations)

#### Historical Tracking

- [ ] **Create migration V5:** `drift_history` table
- [ ] **🧠 Store context metrics with each ASI snapshot**

#### Dashboard: ASI Gauge

- [ ] **Implement circular gauge** — 0-100, color zones, animated needle
- [ ] **🧠 Show context sub-score** — how much of the ASI degradation is context-related

#### Validation Milestone 🧪

- [ ] **Demo:** Agent degrades → ASI drops → dashboard shows which dimensions caused it (including context).

---

### Sprint 4.3 — Auto-Recovery + Model Switching (Week 22–23)

#### Episodic Memory Consolidation (EMC)

- [ ] **Implement `EMCController`:** collect, summarize, replace, index
- [ ] **🧠 EMC is ALSO triggered by context pressure:** if pressure > 85%, trigger EMC preemptively
- [ ] **Implement summary format** — executive_summary, key_decisions, errors_learned, current_state, action_items

#### Context Reset

- [ ] **Implement `ContextReset`:** clear window, inject system prompt + consolidated summary + goal
- [ ] **Trigger conditions:** ASI < 60, oscillation detected, N iterations without progress
- [ ] **🧠 New trigger:** `context_pressure > 90%` for 3 consecutive checks

#### Model Switching

- [ ] **Implement `ModelRouter` with tier-based selection:** Fast, Balanced, Capable
- [ ] **Dynamic tier upgrade on drift** — if ASI drops, upgrade model
- [ ] **Dynamic tier downgrade on stability** — if ASI > 80 for 20+ iterations, downgrade to save cost
- [ ] **🧠 Context-aware model switching:**
  - If context_pressure stays > 80% despite compression → switch to model with larger context window
  - If compression_ratio < 1.5x (can't compress further) → switch to model that handles complexity better
  - Log: "Switched Coder from GPT-5 (128k) to Claude 4 Opus (200k) due to persistent context pressure"

#### Session Handoff

- [ ] **Implement `SessionHandoff`** — new session with same goal, all learnings copied
- [ ] **🧠 Trigger: ASI < 30 OR context_pressure > 95% for 5 consecutive checks**

#### Diagnostic Snapshot

- [ ] **Implement diagnostic capture:** full state + context history + drift history
- [ ] **🧠 Diagnostic includes all context snapshots** — can replay context management decisions

#### Validation Milestone 🧪

- [ ] **Demo:** Context pressure > 85% → EMC triggered → pressure drops. Pressure > 90% persistent → model upgrade to larger context. ASI < 30 → handoff.

---

### Sprint 4.4 — Summarizer Agent + Long-Term Memory (Week 23–24)

#### Summarizer Agent

- [ ] **Implement `SummarizerAgent`** — structured summary from N interactions
- [ ] **🧠 Summarizer is the core of context management** — used by EMC, context reset, and background consolidation
- [ ] **Implement summary validation** — JSON Schema

#### Long-Term Memory

- [ ] **Implement consolidated memory table** — summaries indexed in Qdrant
- [ ] **Implement cross-project memory** — search across all projects
- [ ] **🧠 Memory includes context management decisions** — "We had to compress 5 times because history was too detailed"

#### Memory Cleanup

- [ ] **Implement TTL-based cleanup** — raw chunks: 30 days, summaries: forever
- [ ] **Background purge task** — runs daily

#### Context Optimization Learning

- [ ] **Implement context profile optimizer:**
  - After each session: analyze context management effectiveness
  - Metrics: how many compressions, average pressure, how often emergency triggered
  - Learn optimal budget profile for each project type
  - Suggest: "For Rust API projects, the 'aggressive' profile saves 30% tokens with no quality loss"

#### Validation Milestone 🧪

- [ ] **Demo:** 100+ interactions across 3 sessions. Context optimized automatically. Cross-session memory works.

---

## Phase 4 — Completion Checklist

- [ ] All Sprint 4.1–4.4 tasks complete
- [ ] 🧠 RAG with dynamic K based on context budget
- [ ] 🧠 Context health is dimension #9 in ASI
- [ ] 🧠 EMC triggered by context pressure, not just iterations
- [ ] 🧠 Model switching based on context window needs
- [ ] 🧠 Context optimization learns from past sessions
- [ ] Full auto-recovery pipeline: drift → diagnose → recover → verify

---

## Phase 5: Production (Weeks 25–27)

**Goal:** Documentation, testing, packaging, VPS mode, installer.

### Sprint 5.1 — Documentation (Week 25)

- [ ] **User guides:** installation, quickstart, CLI reference, configuration
- [ ] **🧠 Context management guide:** how budgeting works, how to tune profiles, how to monitor
- [ ] **🏢 Enterprise guide:** multi-tenant setup, RBAC, billing, SSO
- [ ] **Architecture ADRs:** all major decisions documented
- [ ] **OpenAPI spec** for REST API
- [ ] **WebSocket protocol documentation**

### Sprint 5.2 — Testing + Benchmarks (Week 25–26)

- [ ] **Unit tests > 80% coverage**
- [ ] **Integration tests:** full multi-agent workflow with MockProvider
- [ ] **🧠 Context stress test:** 10k interactions, 100 compression events, verify no memory leak
- [ ] **🧠 Context fuzz test:** random context sizes, random pressure spikes, verify never exceeds hard limit
- [ ] **Crash recovery test:** kill mid-execution, verify resume
- [ ] **Performance benchmarks:** token throughput, latency per phase, memory usage
- [ ] **Security audit:** `cargo audit`, manual code review, penetration test

### Sprint 5.3 — VPS Mode + Docker (Week 26–27)

- [ ] **Docker Compose:** core, postgres, redis, qdrant, caddy
- [ ] **Postgres adapter** (full implementation)
- [ ] **Redis adapter** (full implementation)
- [ ] **Sync engine:** local ↔ remote
- [ ] **Backup scripts:** dump, snapshot, S3, restore

### Sprint 5.4 — Installer + Release v1.0 (Week 27)

- [ ] **Install script:** `curl ... | bash`, OS detection, checksum, GPG signature
- [ ] **Homebrew formula**
- [ ] **Windows installer** (NSIS)
- [ ] **GitHub Actions release:** build all targets, test, release, upload
- [ ] **Changelog** from conventional commits

---

## Phase 5 — Completion Checklist

- [ ] Documentation complete
- [ ] Tests > 80% coverage
- [ ] 🧠 Context stress test passed
- [ ] VPS mode functional
- [ ] Installer works on all platforms
- [ ] Release v1.0 published

---

## Phase 6: Enterprise (Weeks 28–30)

**Goal:** Multi-tenant, teams, RBAC, billing, SSO, webhooks. Everything needed for a SaaS business.

### Sprint 6.1 — Multi-Tenant Architecture (Week 28)

#### Data Isolation

- [ ] **Implement org-scoped SQLite:** each org gets its own database file (or schema in Postgres)
- [ ] **Implement org-scoped Qdrant:** collections prefixed with org_id
- [ ] **Implement org-scoped file storage:** project files isolated by org directory
- [ ] **All queries are org-scoped:** verify in every repository method

#### Organization CRUD

- [ ] **Implement `Organization` model:** id, name, slug, plan, settings, created_at
- [ ] **Implement `OrgService`:** create, read, update, delete, list
- [ ] **Implement `OrgSettings`:** allowed_providers, max_seats, max_projects, max_agents_per_goal, context_budget_overrides

#### Team CRUD

- [ ] **Implement `Team` model:** id, org_id, name, description
- [ ] **Implement `TeamService`:** create, add_user, remove_user, list, delete

#### User Management

- [ ] **Implement `User` model:** id, email, name, password_hash, avatar_url, status
- [ ] **Implement `UserService`:** register, login, invite, list, deactivate
- [ ] **Implement invitation flow:** invite email → accept → join org

#### Validation Milestone 🧪

- [ ] **Demo:** Create org, create team, invite user, user joins. Org A data is invisible to Org B.

---

### Sprint 6.2 — RBAC + Permissions (Week 28–29)

#### Roles

- [ ] **Define built-in roles:**
  ```rust
  pub enum OrgRole {
      Owner,        // Full access, billing, delete org
      Admin,        // Manage users, teams, projects, settings
      Developer,    // Full project access, can run agents
      Reviewer,     // Can review agent output, approve gates
      Viewer,       // Read-only: dashboard, logs, metrics
      Billing,      // Billing only: invoices, payment methods
  }
  ```

#### Permissions

- [ ] **Define granular permissions:**
  ```rust
  // Project permissions
  project:create, project:delete, project:read, project:write
  // Session permissions
  session:run, session:stop, session:inject, session:read
  // Team permissions
  team:manage, team:read
  // Billing permissions
  billing:read, billing:write
  // Admin permissions
  admin:users, admin:settings, admin:audit
  ```
- [ ] **Implement `PermissionChecker`** — given user + org + resource, check permission
- [ ] **Implement RBAC middleware** — for both REST API and CLI commands

#### Validation Milestone 🧪

- [ ] **Demo:** Admin can manage users. Developer can run agents. Viewer can only see dashboard. Reviewer can approve gates.

---

### Sprint 6.3 — Billing (Week 29)

#### Billing Model

- [ ] **Define `Plan` enum:** Free, Pro, Team, Enterprise
- [ ] **Define per-plan limits:** seats, projects, sessions, token budget, context_profiles
- [ ] **Define `BillingRecord`:** org_id, period_start, period_end, tokens_used, cost, plan, status

#### Usage Tracking

- [ ] **Track token usage per org** — aggregate from TokenCounter events
- [ ] **Track seat usage** — active users per org
- [ ] **Track storage usage** — SQLite + Qdrant file sizes

#### Invoicing

- [ ] **Implement invoice generation** — monthly, based on usage + plan
- [ ] **Implement payment integration** (Stripe stub — mock for now, real integration later)
- [ ] **Implement usage alerts** — "Org X has used 80% of monthly token budget"

#### Token Budget Enforcement

- [ ] **🧠 Org-level context budget:** org can set max tokens per session per project
- [ ] **🧠 Override context profiles per org:** "Enterprise plans get 'generous' profile, Free plans get 'aggressive'"

#### Validation Milestone 🧪

- [ ] **Demo:** Free plan → limited seats, aggressive context profile. Upgrade to Pro → more seats, generous context. Usage tracked per org.

---

### Sprint 6.4 — SSO + Audit + Webhooks (Week 29–30)

#### SSO / OIDC

- [ ] **Implement `SsoProvider` trait:** authenticate, get_user_info, map_groups_to_teams
- [ ] **Implement `OidcProvider`** — Generic OIDC (works with Google, Azure AD, Okta, Keycloak)
- [ ] **Implement `SamlProvider`** (stub) — for enterprise SAML
- [ ] **Implement SSO flow:** redirect → authenticate → callback → create/login user → JWT

#### Audit Log

- [ ] **Implement `AuditLog`** — append-only log per org
- [ ] **Events to log:**
  - User actions: login, logout, invite, role change
  - Session actions: run, stop, inject, compress
  - Config changes: project config edit, role override
  - Billing: plan change, payment, invoice
  - Security: failed login, permission denied, API key usage
- [ ] **Implement `AuditLogViewer`** — searchable, filterable by event type, user, date range

#### Webhooks

- [ ] **Implement `WebhookDispatcher`:**
  - Register webhooks per org: `POST /api/webhooks`
  - Events: session.completed, session.failed, gate.blocked, security.alert, billing.alert
  - Delivery: POST to URL, retry with backoff (3 attempts), log failures
  - Signature: HMAC-SHA256 signature header for verification
- [ ] **Implement `WebhookConfig`:** url, events, secret, active, retry_count

#### API Keys

- [ ] **Implement `ApiKeyService`:**
  - Generate keys per team: `project-x_xxxxxxxxxxxx`
  - Keys have permissions (scoped to org)
  - Keys can expire
  - Audit log: key used for what, by whom

#### Validation Milestone 🧪

- [ ] **Demo:** SSO login with Google. Audit log shows every action. Webhook fires on session complete. API key authenticates CLI.

---

### Sprint 6.5 — CLI + Dashboard Enterprise Features (Week 30)

#### CLI Enterprise Commands

- [ ] **Implement `org switch <org>`** — switch active organization
- [ ] **Implement `org settings`** — view/edit org-level settings
- [ ] **Implement `team`** — full team management
- [ ] **Implement `billing`** — view plan, usage, invoices
- [ ] **Implement `user`** — invite, list, remove, set role
- [ ] **Implement `audit`** — view audit log, filter, export
- [ ] **Implement `webhook`** — create, list, delete, test
- [ ] **Implement `api-key`** — create, list, revoke

#### Dashboard Enterprise Pages

- [ ] **Organization Settings page:** name, slug, plan, brand colors, allowed providers
- [ ] **Team Management page:** teams list, members per team, add/remove
- [ ] **User Management page:** users list, invite form, role selector
- [ ] **Billing page:** current plan, usage charts, invoices table, payment method
- [ ] **Audit Log page:** searchable log with filters and export
- [ ] **Webhook Settings page:** manage webhooks, test delivery, view logs
- [ ] **API Keys page:** generate, copy, revoke keys

#### 🧠 Enterprise Context Management

- [ ] **Org-level context profiles:** admins can set custom budget profiles per team
- [ ] **Context budget pooling:** unused budget from one team can be allocated to another
- [ ] **🧠 Context analytics:** per-org reports on compression effectiveness, average pressure, token savings from compression

#### Validation Milestone 🧪

- [ ] **Demo:** Full enterprise flow: create org → invite users → set roles → configure SSO → set webhooks → generate API key → run agents → view audit log → see billing usage.

---

## Phase 6 — Completion Checklist

- [ ] Multi-tenant: data isolation per org
- [ ] RBAC: roles + permissions, enforced everywhere
- [ ] Billing: plans, usage tracking, invoices
- [ ] SSO: OIDC integration
- [ ] Audit: append-only log per org
- [ ] Webhooks: event-driven integration
- [ ] API keys: team-scoped authentication
- [ ] 🧠 Org-level context profiles and analytics
- [ ] All enterprise features available via CLI and dashboard

---

## Full Project Checklist — All Phases

- [ ] **Phase 0:** Foundation — workspace, actor model, LLM providers, SQLite, CLI v0.1
- [ ] **Phase 1:** Smart Loop — state machine, MCP, hot memory, **context management**, limits, CLI v0.2
- [ ] **Phase 2:** Multi-Agent — orchestrator, roles, cross-model verification, **mid-loop injection**, security, git, CLI v0.3
- [ ] **Phase 3:** Desktop + Dashboard — REST API, Vue 3, Tauri, **context dashboard**, CLI v1.0
- [ ] **Phase 4:** Memory & Self-Healing — Qdrant, RAG, ASI v2, auto-recovery, **context optimization**
- [ ] **Phase 5:** Production — docs, tests, VPS, installer, release v1.0
- [ ] **Phase 6:** Enterprise — multi-tenant, RBAC, billing, SSO, audit, webhooks

---

## Risk Register

| # | Risk | Probability | Impact | Mitigation |
|---|------|-------------|--------|------------|
| R0 | Ractor has sparse docs | Medium | High | Fallback to Tokio tasks + mpsc. Max 1 week on Ractor. |
| R1 | Loop without brakes burns tokens | High | High | Hard limits from Sprint 1.1. Context budget prevents overuse. |
| R2 | Multi-agent coordination bugs | High | High | Start with 3 agents, add 1 per week. Integration tests. |
| R3 | UI built on unstable API | Medium | High | Freeze API before Phase 3. No breaking changes after Phase 2. |
| R4 | MCP ecosystem maturing | Medium | Medium | Abstract behind Tool trait. Can switch to internal protocol. |
| R5 | LLM costs uncontrolled | High | High | Context compression, caching, model tiering, hard budgets. |
| R6 | **🧠 Context budget too aggressive** | High | Medium | Configurable profiles. Dashboard alert when compression > 5x/hour. |
| R7 | **🧠 Context pressure oscillation** | Medium | High | Compression causes re-request → more compression. Mitigation: compress less aggressively, use model upgrade instead. |
| R8 | Nightly Rust breakage | Medium | Medium | Pin nightly. Test weekly. Only use stable-approved features. |
| R9 | SQLite concurrency limits | Low | Medium | WAL mode, busy timeout, connection pool. Postgres fallback for VPS. |
| R10 | Credential leak | Medium | Critical | Vault from Sprint 0.2. Audit log. Never in logs. |
| R11 | Context compression loses info | High | High | SummarizerAgent is validated. Compression snapshots preserved. Can replay. |
| R12 | Feature creep | Very High | High | Strict roadmap. No Phase 6 features until Phase 0-5 are solid. |

---

## Technology Stack

| Layer | Technology | Version | Rationale |
|-------|-----------|---------|-----------|
| Language | Rust | nightly-2026 | async traits, TAIT, ADT const params |
| Async runtime | Tokio | 1.x | Industry standard, JoinSet, CancellationToken |
| Actor framework | Ractor | 0.x | Lightweight, supervision, location-transparent |
| HTTP/WS | Axum | 0.8 | First-class WebSocket, tower middleware |
| SQLite | rusqlite + r2d2 | 0.32 | Embedded, WAL mode, battle-tested |
| Vector DB | Qdrant | 1.x | Embedded mode, fast, Rust-native |
| Cache | moka | 0.12 | LRU, TTL, concurrent, Rust-native |
| Concurrency | DashMap | 6.x | Lock-free concurrent hashmap |
| Tokenizer | tiktoken-rs | 0.6 | Accurate token counting per model |
| CLI | clap | 4.x | Derive API, comprehensive |
| TUI | ratatui | 0.29 | Terminal UI, widgets, async |
| Desktop | Tauri | 2.x | Native, small binary, Rust backend |
| Frontend | Vue 3 + Vite + TS | 3.x | Reactive, typed, fast HMR |
| Styling | Tailwind CSS | 4.x | Utility-first, rapid prototyping |
| Pipeline viz | Vue Flow | 1.x | Node-based graph visualization |
| Charts | ApexCharts | 4.x | Real-time streaming charts |
| MCP | JSON-RPC 2.0 | — | Industry standard for agent tools |
| WASM | wasmtime | 24.x | Sandboxed execution, fuel metering |
| Auth | jsonwebtoken | 9.x | Stateless JWT authentication |
| OpenID | openidconnect | 4.x | SSO / OIDC integration |
| Payments | Stripe (stub) | — | Billing integration (mock first) |
| Updates | self_update | 0.40 | GitHub releases, checksum verify |
| CI/CD | GitHub Actions | — | Free for public/private repos |

---

## CLI Reference (Final)

```
project-x <command> [options]

CORE COMMANDS:
  init <name>                   Create a new project
  run                           Execute a goal
    --goal <text>               Goal description
    --resume                    Resume last session
    --session <id>              Resume specific session
    --dry-run                   Show plan without executing
    --headless                  JSON output for CI/CD
    --goal <name>               Use named goal from config
    --agents <list>             Override agents
    --agent.<role>.<key> <val>  Per-agent override
    
    🧠 --context-profile <p>    Override budget profile
    🧠 --context-k <n>         Override RAG chunk count
    
  inject                        Inject mid-loop instruction
    --session <id>              Target session
    --agent <name>              Target agent (or "all")
    --type <type>               instruction|context|correction|halt
    --message <text>            The instruction
    --priority <p>              low|normal|high|critical
  
  stop                          Stop running session
    --session <id>              Session to stop

  🧠 CONTEXT COMMANDS:
  context inspect [--session <id>]   Show budget, pressure, sections
  context history [--session <id>]   Compression timeline
  context force-compress [--session <id>]  Trigger EMC now
  context profiles                     List available profiles

PROJECT COMMANDS:
  project list                   List all projects
  project show <id>              Project details
  project archive <id>           Archive project

SESSION COMMANDS:
  session list [--project <id>]  List sessions
  session show <id>              Session details
  session stop <id>              Stop session
  session logs <id> [--tail]     View logs
    --json                       JSON format
    --level <level>              Filter by level

CONFIG COMMANDS:
  config show                    Show configuration
  config get <key>               Get value
  config set <key> <value>       Set value
  config edit                    Open in editor
  config import/export           File I/O

PROVIDER COMMANDS:
  provider list                  List providers
  provider test <name>           Test connection

MCP COMMANDS:
  mcp list                       List servers
  mcp add <name> <cmd> [args]    Add server
  mcp remove <name>              Remove server
  mcp test <name>                Test connection

MEMORY COMMANDS:
  memory search <query>          Search across sessions
  memory stats                   Usage statistics

DEPLOY COMMANDS:
  deploy setup <host>            Configure VPS
  deploy push                    Sync to VPS
  deploy pull                    Sync from VPS
  deploy status                  VPS health
  deploy logs [--tail]           Remote logs

🏢 ENTERPRISE COMMANDS:
  org create <name>              Create organization
  org list                       List organizations
  org switch <id>                Switch active org
  org show                       Current org details
  org settings                   View/edit org settings
  
  team create <name>             Create team
  team list                      List teams
  team add-user <email> <role>   Add user to team
  team remove-user <email>       Remove user from team
  
  user invite <email>            Invite user to org
  user list                      List users
  user remove <email>            Remove user
  user set-role <email> <role>   Change user role
  
  billing show                   Current plan + usage
  billing invoices               Invoice history
  
  audit [--event <type>]         View audit log
    --user <email>               Filter by user
    --from <date>                Date range
    --export <format>             csv|json
  
  webhook create <url>           Create webhook
  webhook list                   List webhooks
  webhook delete <id>            Delete webhook
  webhook test <id>              Test delivery
  
  api-key create                  Generate key
  api-key list                    List keys
  api-key revoke <id>             Revoke key

DESKTOP / DASHBOARD:
  desktop                        Open desktop app
  dashboard                      Open web dashboard
  monitor                        Open terminal UI

SYSTEM:
  update [--channel <ch>]        Update to latest
  version                        Show version
  diagnose <session>             Diagnostic report
  help                           Show help
```

---

*End of ROADMAP — 30 weeks, 6 Phases, 16 Sprints, ~220 tasks.*
*Core innovations: Context Window Management (never reach max context), Mid-Loop Injection, Enterprise Multi-Tenant.*