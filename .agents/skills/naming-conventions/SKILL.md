---
name: naming-conventions
description: Apply consistent naming conventions across the praxis codebase — Rust crates, modules, functions, types, constants, and Vue/TypeScript components, composables, stores, files. Use whenever creating or renaming any file, function, type, variable, crate, module, component, composable, or store. Also use when the user asks about naming, casing (snake_case, PascalCase, camelCase, SCREAMING_SNAKE), or "what should I call this".
---

# Naming Conventions — Rust + Vue/TypeScript

Consistent naming is the cheapest form of documentation. When names follow
convention, a reader knows what a thing **is** (type? function? constant?)
and what it **does** without reading the definition. This skill covers both
sides of praxis: the Rust workspace and the Vue dashboard.

---

## Rust

### Crates

- **`praxis-<name>`** — workspace crate prefix.
- Lowercase, hyphen-separated.
- Examples: `praxis-core`, `praxis-shared`, `praxis-agent-traits`, `praxis-mcp-host`.

```toml
[package]
name = "praxis-core"      # crate name in Cargo.toml
```

```rust
// In code, hyphens become underscores:
extern crate praxis_core;  // but use `use praxis_core::...` instead
```

### Modules / files

- **`snake_case`** — lowercase with underscores.
- One module per file. File name = module name.

```
src/
├── orchestrator.rs       # module orchestrator
├── orchestrator/         # submodules
│   ├── task.rs           # module orchestrator::task
│   └── verification.rs   # module orchestrator::verification
├── drift/
│   ├── mod.rs            # (legacy — prefer drift.rs)
│   ├── metrics.rs
│   └── recovery.rs
```

- **Module names are singular** when they represent one concept (`actor`,
  `drift`, `workflow`), **plural** when they're a collection of submodules
  (`roles`, `routes`).

### Functions / methods

- **`snake_case`**, verb-first.
- The name should read as a sentence when called: `agent.fetch_messages()`,
  `store.persist_event()`, `vault.get_credential()`.

```rust
// ✅ Verb-first, reads naturally
fn fetch_agent(id: &Uuid) -> Result<Agent>
fn validate_transition(from: Phase, to: Phase) -> Result<()>
fn persist_event(event: &Event) -> Result<EventId>

// ❌ No verb, unclear what it does
fn agent(id: &Uuid) -> Result<Agent>       // fetch? get? create?
fn transition(from: Phase, to: Phase)      // validate? execute?
```

- **Getters**: no `get_` prefix. Rust convention omits it:

```rust
// ❌
fn get_name(&self) -> &str

// ✅
fn name(&self) -> &str
```

- **Setters**: use the field name (for builders) or `set_` prefix:

```rust
fn set_model(&mut self, model: &str)     // setter
fn with_model(mut self, model: &str)     // builder pattern (consuming)
```

- **Boolean predicates**: `is_`, `has_`, `can_`, `should_` prefix:

```rust
fn is_healthy(&self) -> bool
fn has_permission(&self, perm: &str) -> bool
fn can_retry(&self) -> bool
fn should_consolidate(&self) -> bool
```

### Types (structs, enums, traits)

- **`PascalCase`** — uppercase first letter, no underscores.

```rust
struct OrchestratorState { ... }
enum Phase { Design, Implement, Review, Test }
trait LLMProvider { ... }
```

- **Enums**: variants are `PascalCase`, no enum name prefix (Rust namespaces
  them):

```rust
// ✅
enum ChatRole {
    System,
    User,
    Assistant,
    Tool,
}

// ❌ Redundant prefix
enum ChatRole {
    ChatRoleSystem,    // the enum already says "ChatRole"
    ChatRoleUser,
}
```

- **Enums with data**: descriptive variant names, fields named in struct
  syntax for clarity:

```rust
enum StreamChunk {
    Delta(String),
    Done(TokenUsage),
    Error(String),
}

// Struct-variant for multiple fields — self-documenting
enum ProjectXError {
    InvalidTransition { from: String, to: String },
    ContextBudgetExceeded { pressure: f32, limit: f32 },
}
```

### Constants / statics

- **`SCREAMING_SNAKE_CASE`** — all caps with underscores.

```rust
const MAX_ITERATIONS: u32 = 1000;
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);
static LAZY_CONFIG: LazyLock<Config> = LazyLock::new(Config::load);
```

### Type aliases

- **`PascalCase`**, same as types:

```rust
type Result<T> = std::result::Result<T, ProjectXError>;
type StreamReceiver = mpsc::Receiver<StreamChunk>;
```

### Generic type parameters

- **Single uppercase letter** for simple generics (`T`, `K`, `V`, `E`).
- **Descriptive PascalCase** when the meaning matters:

```rust
// ✅ Simple container — T is fine
struct Vec<T> { ... }

// ✅ Meaningful — Agent and Message convey intent
trait Actor<Agent, Message> { ... }

// ❌ Too terse for a complex trait
trait Handler<A, M> { ... }   // A? M? what are they?
```

### Lifetimes

- **Short lowercase**: `'a`, `'b` for simple cases.
- **Descriptive** when the lifetime has meaning: `'src`, `'static`, `'env`.

```rust
fn parse<'src>(input: &'src str) -> Token<'src> { ... }
```

---

## Vue / TypeScript (dashboard)

### Component files

- **`PascalCase.vue`** — one component per file.
- Suffix with `View` for route-level pages, `Card`/`List`/`Form` for
  presentational patterns.

```
src/
├── views/
│   ├── LoginView.vue
│   ├── SettingsView.vue
│   └── OverviewView.vue
├── components/
│   ├── ui/
│   │   ├── Button.vue
│   │   ├── Input.vue
│   │   └── Icon.vue
│   ├── MetricCard.vue
│   └── ProjectList.vue
```

### Composables

- **`useXxx.ts`** — `use` prefix, camelCase, returns reactive state.
- One composable per file. File name = composable name.

```ts
// src/composables/useWebSocket.ts
export function useWebSocket() {
  const connected = ref(false)
  const events = ref<Event[]>([])
  // ...
  return { connected, events }
}
```

### Pinia stores

- **`useXxxStore`** — `use` prefix, `Store` suffix.
- File name: `xxx.ts` (lowercase, matching the store domain).

```ts
// src/stores/app.ts
export const useAppStore = defineStore('app', () => {
  const projects = ref<Project[]>([])
  const loading = ref(false)
  // ...
  return { projects, loading }
})
```

### Functions

- **`camelCase`**, verb-first — same principle as Rust.

```ts
function fetchProjects() { ... }
function formatDate(iso: string) { ... }
function handleLogin(token: string) { ... }
```

- **Event handlers**: `handle` prefix (`handleClick`, `handleLogin`).
- **Boolean getters**: `is`/`has`/`can`/`should` prefix (`isLoading`,
  `hasError`).

### Variables

- **`camelCase`** for locals and refs.
- **`SCREAMING_SNAKE_CASE`** for true constants (not refs):

```ts
const MAX_RETRY_COUNT = 3              // constant
const isLoading = ref(false)           // reactive state
const recentEvents = computed(() => ...)  // derived
```

### TypeScript types / interfaces

- **`PascalCase`**:

```ts
interface Project { id: string; name: string }
type ViewName = 'overview' | 'projects' | 'settings'
interface ChatMessage { role: ChatRole; content: string }
```

- **Interfaces** for object shapes, **types** for unions/intersections.
- Don't prefix with `I` (no `IProject`) — that's a C# convention, not TS.

### CSS classes

- **`kebab-case`** — lowercase with hyphens.

```css
.metric-card { ... }
.nav-item { ... }
.data-row { ... }
```

- Tailwind utility classes are fine inline for one-off styling. For reusable
  patterns, extract to a CSS class in `<style scoped>`.

---

## Database / SQL naming

- **Tables**: `snake_case`, plural (`projects`, `events`, `sessions`).
- **Columns**: `snake_case` (`created_at`, `project_id`, `forge_toml`).
- **Foreign keys**: `<table_singular>_id` (`project_id`, `session_id`).
- **Timestamps**: `created_at`, `updated_at` (ISO 8601 strings).

---

## Quick reference table

| Element | Rust | Vue/TS |
|---------|------|--------|
| Crate / package | `praxis-core` | `praxis-dashboard` |
| File (module) | `snake_case.rs` | `PascalCase.vue`, `camelCase.ts` |
| Function | `snake_case`, verb-first | `camelCase`, verb-first |
| Type / struct / enum | `PascalCase` | `PascalCase` |
| Trait / interface | `PascalCase` | `PascalCase` |
| Constant | `SCREAMING_SNAKE` | `SCREAMING_SNAKE` |
| Variable | `snake_case` | `camelCase` |
| Boolean | `is_`/`has_` prefix | `is`/`has` prefix |
| CSS class | — | `kebab-case` |
| Composable | — | `useXxx` |
| Store | — | `useXxxStore` |
| DB table | `snake_case`, plural | — |
| DB column | `snake_case` | — |
