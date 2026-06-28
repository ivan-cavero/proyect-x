# Project-X

**Autonomous Multi-Agent System — Infinite Loop, Cross-Model Verification, Desktop-Native**

> **Status:** v1.0 — Production Ready  
> **Stack:** Rust (nightly) + Tauri v2 + Vue 3 + SQLite + Qdrant (embedded) + Ractor  
> **Distribution:** Single static binary (~15–20 MB), zero external dependencies  
> **License:** Proprietary (until further notice)

---

## The Idea

Most AI coding assistants are **single-agent, single-model, single-threaded**. You send a prompt, one model responds, you iterate manually. OpenCode, Cursor, Claude Code, Aider — they are all fundamentally the same architecture: a user in the loop, one agent, one context window, no parallel verification, no memory between sessions.

**Project-X is different.**

It is a **multi-agent factory** where each agent is a specialist with its own model, its own tools, its own system prompt, and its own perspective. They work in parallel, review each other's output, verify from different angles, and never stop iterating until the goal is met.

### Core Principles

| Principle | Why it matters |
|-----------|----------------|
| **One binary, any environment** | Install once. No Docker, no Postgres, no Redis. SQLite + Qdrant embedded run inside the process. |
| **Every goal is an orchestra** | You define which agents participate, with which model, at which temperature, with which role. Nothing is hardcoded. |
| **Cross-model verification** | Different models for different agents. GPT-5 writes, Claude 4 Opus reviews, Gemini 2.5 Pro audits security. One model's blind spot is another's strength. |
| **Loop with brakes** | State machine, quality gates, divergence detection, hard limits. The loop never runs wild. |
| **Three-tier memory** | Hot (in-memory), Episodic (vector DB), Consolidated (compressed summaries). The agent remembers between sessions without context rot. |
| **CLI + Desktop are the same binary** | Tauri embeds the core. The CLI is the same binary in headless mode. Deploy to VPS and connect remotely. |

---

### What It Does

```bash
# Install (one command)
curl -fsSL https://project-x.dev/install.sh | bash

# Create a project
project-x init my-api

# Define your agents and models in TOML
vim forge.toml
#   [roles.coder]
#   model = "gpt-5"
#   [roles.reviewer]
#   model = "claude-4-opus"

# Run with a goal — the system loops until it's done
project-x run --goal "Create a REST API in Rust with JWT auth"

# Or open the desktop app
project-x desktop

# Or deploy to a VPS
project-x deploy setup user@vps
project-x deploy push
```

The system spawns an **Orchestrator** that:
1. Reads the goal configuration
2. Activates the specified agents (Architect, Coder, Reviewer, Security, Tester...)
3. Each agent runs with its configured model (GPT-5, Claude, Gemini, local...)
4. Agents execute phases: Design → Implement → Review → Secure → Test → Commit
5. **Quality gates** block progress until criteria are met
6. **Divergence detection** prevents context rot and oscillation
7. **Checkpointing** persists state on every transition — crash recovery is built-in
8. **Memory indexing** stores everything in vector DB for cross-session recall
9. The loop continues until the goal is complete or hard limits are reached

---

### Agent Roles (Predefined + Custom)

| Role | Default Model | Responsibility |
|------|---------------|----------------|
| **Orchestrator** | — | Supervisor, routing, consensus, state machine |
| **Architect** | Claude 4 Opus | System design, ADRs, technology decisions |
| **Coder** | GPT-5 | Code generation, implementation, compilation |
| **Reviewer** | Gemini 2.5 Pro | Code review, quality, edge case analysis |
| **Security** | Claude 4 Haiku | Vulnerability scanning, secrets detection |
| **Tester** | GPT-5 | Test generation, execution, coverage |
| **Git** | — | Commit, branch, push, PR management |
| **Researcher** | GPT-5 | Web search, documentation, synthesis |
| **MemoryKeeper** | — | Background indexing, consolidation |
| **DriftGuard** | — | Stability metrics, auto-recovery |

Every role is **customizable** in `forge.toml`: model, temperature, system prompt, allowed tools. You can also define **new roles** from scratch.

---

### Architecture Overview

```
┌──────────────────────────────────────────────────────┐
│                    BINARY (project-x)                  │
│                                                       │
│  ┌──────────────┐  ┌──────────────┐  ┌────────────┐  │
│  │   Core        │  │   MCP Host   │  │  HTTP/WS   │  │
│  │  (ractor)     │  │  (tokio)     │  │  (axum)    │  │
│  │               │  │              │  │            │  │
│  │ Orchestrator  │  │ FileSystem   │  │ REST API   │  │
│  │ Architect     │  │ Git          │  │ WebSocket  │  │
│  │ Coder         │  │ WebSearch    │  │ Auth       │  │
│  │ Reviewer      │  │ GitHub       │  │            │  │
│  │ Security      │  │ Custom MCPs  │  │            │  │
│  │ Tester        │  │              │  │            │  │
│  │ Researcher    │  │              │  │            │  │
│  │ DriftGuard    │  │              │  │            │  │
│  │ MemoryKeeper  │  │              │  │            │  │
│  └──────┬───────┘  └──────┬───────┘  └─────┬──────┘  │
│         │                 │                 │         │
│  ┌──────▼─────────────────▼─────────────────▼──────┐  │
│  │              PERSISTENCE LAYER                   │  │
│  │  ┌──────────┐  ┌──────────┐  ┌───────────────┐  │  │
│  │  │  SQLite  │  │  Qdrant  │  │  moka (cache) │  │  │
│  │  │ (event   │  │ (vector  │  │  + DashMap    │  │  │
│  │  │  store)  │  │  memory) │  │  (hot state)  │  │  │
│  │  └──────────┘  └──────────┘  └───────────────┘  │  │
│  └─────────────────────────────────────────────────┘  │
│                                                       │
│  ┌──────────────────────────────────────────────────┐  │
│  │            SYNC ENGINE (optional)                 │  │
│  │  Local project ←→ Remote VPS (via SSH/API)       │  │
│  └──────────────────────────────────────────────────┘  │
│                                                       │
│  ┌──────────────────────────────────────────────────┐  │
│  │          Tauri Desktop (Vue 3 Dashboard)          │  │
│  │  Pipeline View · Token Chart · Agent Health      │  │
│  │  Memory Explorer · Prompt Injection · Config     │  │
│  └──────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────┘
```

---

### Memory Architecture (Three Tiers)

```
                    ┌──────────────────────────────────┐
                    │      MEMORY KEEPER (background)    │
                    │  - Indexes every N interactions    │
                    │  - Generates embeddings            │
                    │  - Consolidates summaries          │
                    └──────────┬───────────────────────┘
                               │
        ┌──────────────────────┼──────────────────────┐
        ▼                      ▼                       ▼
┌───────────────┐    ┌──────────────────┐    ┌────────────────┐
│  HOT MEMORY   │    │  EPISODIC MEMORY │    │  CONSOLIDATED  │
│  (DashMap)    │    │  (Qdrant)        │    │  (Summaries)   │
│               │    │                  │    │                │
│  • Session    │    │  • Embeddings    │    │  • Compressed  │
│    state      │    │  • Chunks        │    │    summaries   │
│  • Context    │    │  • Search        │    │  • Decisions   │
│    window     │    │  • RAG           │    │  • Learnings   │
│  • Cache      │    │  • Cross-session │    │  • Cross-project│
└───────────────┘    └──────────────────┘    └────────────────┘
```

---

### Drift Detection — Agent Stability Index (ASI)

The system monitors **8 dimensions** of agent health in real time:

| Dimension | Weight | What It Measures |
|-----------|--------|------------------|
| Goal Alignment | 20% | Cosine similarity between goal embedding and output |
| Tool Usage Consistency | 15% | Distribution of tool calls vs baseline |
| Error Rate | 15% | % of failed tool calls |
| Output Length Stability | 10% | Standard deviation of response length |
| Response Time Stability | 10% | Z-score of latency |
| Output Uniqueness | 10% | 1 - cosine similarity between consecutive outputs |
| Gate Pass Rate | 10% | % of gates passed on first attempt |
| Token Efficiency | 10% | Tokens per unit of progress |

**ASI Score = weighted average (0–100). Actions:**

| Score | Color | Action |
|-------|-------|--------|
| 80–100 | 🟢 Healthy | None |
| 60–80 | 🟡 Attention | Log, dashboard notification |
| 40–60 | 🟠 Drift | Force consolidation + context reset |
| 20–40 | 🔴 Critical | Pause agent, notify user, upgrade model |
| 0–20 | ⚫ Severe | Kill session, save diagnostic, don't auto-resume |

---

### Configuration-Driven Design

Everything is in `forge.toml`:

```toml
[project]
name = "my-api"

[roles.coder]
model = "gpt-5"
temperature = 0.3
system_prompt = "You are an expert Rust engineer..."

[roles.reviewer]
model = "claude-4-opus"
temperature = 0.2
system_prompt = "Review this code critically..."

[[goals]]
name = "full-feature"
agents = ["architect", "coder", "reviewer", "security", "tester"]
gates = ["review.pass", "security.no_critical", "test.pass"]
max_iterations = 10
parallel_reviewers = 2

[[goals]]
name = "quick-fix"
agents = ["coder", "reviewer"]
coder.model = "claude-4-haiku"  # Per-goal override
max_iterations = 3
```

Define your own roles, your own goals, your own workflow. Nothing is hardcoded.

---

## Why Rust Nightly?

| Feature | Why We Need It |
|---------|----------------|
| `async_fn_in_trait` | Agent traits with async methods without boxing |
| `return_type_notation` | Precise async return types in trait definitions |
| `type_alias_impl_trait` | Complex nested type aliases for actor mailboxes |
| `associated_type_defaults` | Default implementations in agent traits |
| `adt_const_params` | Compile-time phase enum validation |
| `async_gen` | Eventually: streaming generators for agent output |
| `never_type` | Infallible error handling in actor supervisors |

Rust nightly gives us cleaner abstractions without runtime overhead. We can stabilize to specific nightly versions via `rust-toolchain.toml`.

---

## Directory Structure

```
project-x/
├── Cargo.toml                       # Workspace root (nightly)
├── rust-toolchain.toml              # Nightly pin
├── crates/
│   ├── core/                        # Runtime: actor model, state machine, orchestrator
│   ├── agent-traits/                # Public traits: LLMProvider, Tool, Memory, Persistence
│   ├── providers/                   # LLM implementations: OpenAI, Anthropic, Gemini, Ollama
│   ├── mcp-host/                    # MCP client: discovery, protocol, tool registry
│   ├── memory/                      # Memory implementations: hot, episodic, consolidated
│   ├── persistence/                 # Event store: SQLite (default), Postgres (VPS)
│   ├── vault/                       # Credential management: keyring, tauri, env
│   ├── cli/                         # CLI interface: clap + ratatui
│   ├── desktop/                     # Tauri v2 binary
│   └── shared/                      # Shared types, protocol, config
├── dashboard/                       # Vue 3 frontend (Vite + TypeScript + Tailwind)
├── mcp-servers/                     # First-party MCP servers (separate processes)
├── tools/                           # Example WASM tools
├── docs/
│   ├── adr/                         # Architecture Decision Records
│   └── guides/                      # User documentation
├── scripts/                         # CI/CD, release, install
├── examples/                        # Example projects with forge.toml
└── tests/                           # Integration tests
```

---

## License

Proprietary. All rights reserved. Open-source planned for the future.

---

## Status

Currently in **Phase 0: Foundation**. See [ROADMAP.md](./ROADMAP.md) for detailed progress.