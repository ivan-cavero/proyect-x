---
name: clean-architecture
description: Design clean crate boundaries, module structure, and trait abstractions in the praxis Rust workspace. Use whenever creating a new crate, module, or trait, reviewing cross-crate dependencies, designing the actor model architecture, organizing source files, or refactoring code structure. Also use when the user mentions architecture, crate boundaries, dependency direction, module organization, trait design, separation of concerns, or asks "where should this code go".
---

# Clean Code & Architecture — praxis Workspace

Architecture in a Rust workspace is about **crate boundaries and dependency
direction.** Get those right and the borrow checker enforces your design at
compile time. Get them wrong and you get circular dependencies, god-crates,
and tests that require the entire workspace to compile.

---

## Dependency Rule — inward only

Dependencies point **inward only**. The innermost crates know nothing about
the outer ones.

```
[providers, mcp-host, memory, persistence, vault]   ← implementations
                        ↓ depend on
                  [agent-traits]                     ← contracts
                        ↓ depends on
                    [shared]                         ← foundation
                        ↑ depends on all
                    [core]                           ← composition root
                        ↑ depends on core
                [cli, desktop]                       ← entry points
```

### Layer responsibilities

- **`shared`** — types, protocol messages, config structs, error enum. Pure
  data. Zero framework imports, zero async, zero I/O. If it needs `tokio` or
  `reqwest`, it doesn't belong here.
- **`agent-traits`** — the contracts: `LLMProvider`, `Tool`, `Memory`,
  `Persistence`. Trait definitions + associated types only. Depends on
  `shared` only. No implementations.
- **`providers`** — concrete LLM implementations (OpenAI, Anthropic, Gemini,
  Ollama). Implements `agent-traits::LLMProvider`. Depends on traits + shared.
- **`mcp-host`** — MCP client: discovery, protocol, tool registry. Depends on
  traits + shared.
- **`memory`** — hot (DashMap), episodic (Qdrant), consolidated. Depends on
  traits + shared.
- **`persistence`** — event store: SQLite (default), Postgres (VPS). Depends
  on traits + shared.
- **`vault`** — credential management: keyring, tauri, env. Depends on shared.
- **`core`** — the **composition root**. Wires everything together: ractor
  actors, state machine, orchestrator, axum API. The only crate that depends
  on all others.
- **`cli` / `desktop`** — entry points. Thin binaries that call into `core`.
  Depend on `core` only.

### The golden rule

> **`shared` and `agent-traits` never depend on `core`.**

If you find yourself adding `praxis-core` as a dependency of `shared` or
`agent-traits`, you've made a design error. The contracts don't know about
the runtime.

---

## Crate boundary checklist

Before adding code to a crate, ask:

| Question | If yes → |
|----------|----------|
| Is it a type, protocol message, or config struct? | `shared` |
| Is it a trait definition (contract)? | `agent-traits` |
| Is it a concrete implementation of a trait? | The matching impl crate (`providers`, `memory`, etc.) |
| Does it wire implementations together? | `core` |
| Is it a binary entry point? | `cli` or `desktop` |
| Does it need `tokio`, `reqwest`, `rusqlite`? | Not `shared` — move to an impl crate |

---

## Module structure — file-based, not `mod.rs`

Modern Rust (Edition 2024) uses `foo.rs` + `foo/` directory, not `foo/mod.rs`:

```
crates/core/src/
├── lib.rs              // crate root: pub mod declarations
├── orchestrator/
│   ├── mod.rs          // ❌ legacy — don't use
│   ├── injection.rs    // ...actually this project has mod.rs — migrate when touching
│   └── task.rs
├── actor/
│   ├── mod.rs
│   ├── agent.rs
│   ├── supervisor.rs
│   └── roles/
│       ├── mod.rs
│       ├── coder.rs
│       └── reviewer.rs
```

> **Note:** praxis currently uses `mod.rs` in some places. When you touch a
> module, migrate it to `foo.rs` + `foo/` as part of the change. Don't do a
> mass migration in a single commit — it pollutes the diff.

### Module rules
- One responsibility per module. If a file exceeds ~300 lines, split by
  concern.
- `pub use` re-exports at the crate root (`lib.rs`) for a clean public API:

```rust
// lib.rs
pub mod orchestrator;
pub mod actor;
pub mod api;

// Flatten the API surface — callers write `praxis_core::Orchestrator`
pub use orchestrator::Orchestrator;
pub use actor::AgentActor;
```

- `pub mod prelude` for the most commonly used types:

```rust
pub mod prelude {
    pub use crate::config::*;
    pub use crate::error::*;
    pub use crate::protocol::*;
    pub use crate::types::*;
}
```

---

## Trait design — contracts in `agent-traits`

Traits are the **seam** between `core` (which uses them) and impl crates
(which implement them). Design them carefully.

### Rules
- **`Send + Sync` bounds** on all traits that cross actor/thread boundaries:

```rust
#[async_trait]
pub trait LLMProvider: Send + Sync {
    async fn chat(&self, messages: &[ChatMessage], config: &ChatConfig) -> Result<ChatResponse>;
}
```

- **Associated types over generics** when the type is implementation-specific:

```rust
// ✅ Associated type — each impl chooses its own
trait Memory {
    type Entry;
    async fn store(&self, entry: Self::Entry) -> Result<()>;
}

// ❌ Generic — forces the caller to specify, leaks impl details
trait Memory<T> {
    async fn store(&self, entry: T) -> Result<()>;
}
```

- **`#[async_trait]` for dyn dispatch** — praxis uses `#[async_trait]`
  because it needs `dyn LLMProvider` (object-safe traits for runtime
  provider selection). Native `async fn in trait` (AFIT) is not object-safe.
- **Don't put `impl Trait` in public trait signatures** — callers can't add
  bounds (e.g., `Send`). Use `#[async_trait]` or explicit return types.
- **Keep traits small** — one responsibility. A trait with 15 methods is a
  god-trait. Split it.

### Trait variant for `Send` async traits

For public async traits that need a `Send` variant, use `trait-variant`:

```rust
#[trait_variant::make(LLMProvider: Send)]
pub trait LocalLLMProvider {
    async fn chat(&self, messages: &[ChatMessage]) -> Result<ChatResponse>;
}
// Produces both LocalLLMProvider and a Send LLMProvider
```

---

## Error strategy — unified enum in `shared`

praxis uses a single `ProjectXError` enum in `shared` with `#[from]`
conversions. This is the right call for a workspace where errors propagate
across crate boundaries.

```rust
// shared/src/error.rs
#[derive(thiserror::Error, Debug)]
pub enum ProjectXError {
    #[error("LLM provider error: {0}")]
    ProviderError(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),
    // ...
}

pub type Result<T> = std::result::Result<T, ProjectXError>;
```

### Rules
- **Library crates return `Result<T, ProjectXError>`** (the shared type).
- **Application code (`core`, `cli`) can use `anyhow::Result`** for
  ergonomic context chaining at the top level.
- **`#[from]` for automatic conversion** — so `?` just works:

```rust
impl From<serde_json::Error> for ProjectXError {
    fn from(e: serde_json::Error) -> Self {
        ProjectXError::Internal(format!("JSON error: {}", e))
    }
}
```

- **Don't create per-crate error enums** — they force callers to map between
  error types at every boundary. One shared enum is simpler.
- **Error messages are user-facing** — don't leak secrets or internal paths
  in `#[error("...")]` strings.

---

## Actor model architecture (ractor)

praxis uses `ractor` for the actor model. Each agent role is an actor with
its own mailbox.

### Actor hierarchy

```
Supervisor (top-level, restarts children)
├── Orchestrator (state machine, routes work)
│   ├── ArchitectActor
│   ├── CoderActor
│   ├── ReviewerActor
│   ├── SecurityActor
│   ├── TesterActor
│   ├── ResearcherActor
│   ├── GitActor
│   ├── MemoryKeeperActor (background indexing)
│   └── DriftGuardActor (stability monitoring)
```

### Rules
- **One actor per role instance** — don't share an actor across goals.
- **Messages are the only communication** — actors don't share mutable state.
  If they need shared state, use `Arc<DashMap<_>>` or `Arc<Mutex<_>>`.
- **Supervisors handle failures** — let actors return `Result`, let the
  supervisor decide whether to restart, retry, or escalate.
- **Don't block in message handlers** — if a handler does I/O (LLM API call),
  it blocks the actor's mailbox. Use `tokio::spawn` for fire-and-forget, or
  accept that the actor is busy until the I/O completes.
- **Message types in the role's module** — co-locate messages with the actor
  that handles them.

---

## Functions

- Max ~30 lines for Rust (longer than JS because match arms are verbose).
  Longer → extract and name the extracted piece.
- Max 4 parameters. More → builder struct or `Config` struct.
- Guard clauses: return early to avoid nesting.
- No boolean parameters that change behavior — they are two functions:

```rust
// ❌ Boolean param = hidden branching
fn process(prompt: &str, is_streaming: bool) -> Result<Output>

// ✅ Explicit intent
fn process_batch(prompt: &str) -> Result<Output>
fn process_stream(prompt: &str) -> Result<StreamReceiver>
```

---

## What to avoid

- **God crates** — one crate doing everything. Split by responsibility.
- **Circular dependencies** — if crate A needs B and B needs A, extract the
  shared part into C that both depend on.
- **`core` depending on `cli` or `desktop`** — entry points depend on core,
  never the reverse.
- **Leaking implementation details through traits** — if a trait exposes
  `reqwest::Client` or `rusqlite::Connection`, the abstraction is broken.
- **Premature abstractions** — YAGNI. Don't create a trait for one
  implementation. Wait until you have two, then extract.
- **`pub` everything** — export only what consumers need. Internals stay
  private. `pub(crate)` for cross-module-but-internal.
- **Shared mutable state across actors** — actors communicate via messages.
  If you need shared state, make it explicit (`Arc<DashMap>`) and documented.
