---
name: rust-performance
description: Write fast, efficient Rust code and diagnose performance bottlenecks in the praxis workspace (ractor actors, tokio async, SQLite, Qdrant, moka, DashMap). Use whenever writing or reviewing Rust code where performance matters, profiling hot paths, optimizing allocations, tuning async concurrency, configuring release builds, benchmarking with criterion, or investigating latency/memory issues. Also use when the user mentions slow Rust code, CPU usage, memory leaks, allocation overhead, actor throughput, tokio task scheduling, connection pooling, or asks "how do I make this faster".
---

# Rust Performance — praxis Stack

Performance in Rust is about two things: **don't allocate when you can borrow,
and don't block when you can await.** The borrow checker handles the first by
default; the second requires discipline around async boundaries.

This skill covers general Rust performance plus the specific patterns relevant
to praxis: `ractor` actors, `tokio` async, SQLite via `rusqlite`+`r2d2`, Qdrant
vector search, `moka` cache, and `DashMap` hot state.

---

## Release profile — set it once

The workspace already has a good release profile. Understand why each flag
matters before changing it:

```toml
[profile.release]
opt-level = 3       # max optimization — non-negotiable for release
lto = true           # link-time optimization across crates — smaller + faster
codegen-units = 1    # single codegen unit — better inlining, slower compile
strip = true         # strip debug symbols — smaller binary (~15-20 MB target)
```

Trade-off: `lto = true` + `codegen-units = 1` makes compile times much slower.
For dev iteration, `[profile.dev]` stays at `opt-level = 0` + `debug = true`.

For profiling builds (faster than release, with debug info):

```toml
[profile.release-debug]
inherits = "release"
debug = true
strip = false        # keep symbols for profiler
```

---

## Allocation — the #1 lever

Every `String`, `Vec`, `Box`, `Arc` is a heap allocation. In hot loops, these
dominate. The strategy: **borrow in, pre-allocate, reuse buffers.**

### Borrow instead of allocate

```rust
// ❌ Allocates a String on every call
fn process(input: &str) -> String {
    input.to_uppercase()
}

// ✅ Writes into a caller-provided buffer — zero allocation
fn process_into(input: &str, out: &mut String) {
    out.clear();
    out.push_str(input);
    out.make_ascii_uppercase();
}
```

### Pre-allocate when size is known

```rust
// ❌ Grows + reallocates as elements are pushed
let mut results = Vec::new();
for item in source { results.push(transform(item)); }

// ✅ One allocation, no reallocation churn
let mut results = Vec::with_capacity(source.len());
for item in source { results.push(transform(item)); }
```

### Reuse buffers across iterations

For actor message processing or streaming, keep a buffer in the actor state
and reuse it instead of allocating per message:

```rust
struct CoderActor {
    prompt_buf: String,   // reused across messages
}

impl Handler<Prompt> for CoderActor {
    fn handle(&mut self, msg: Prompt) -> String {
        self.prompt_buf.clear();
        self.prompt_buf.push_str(&msg.system);
        self.prompt_buf.push_str(&msg.user);
        // ... use prompt_buf, no new allocation
    }
}
```

### `Cow` for zero-copy when mutation is rare

```rust
use std::borrow::Cow;

fn normalize(input: &str) -> Cow<'_, str> {
    if input.is_ascii() { Cow::Borrowed(input) }     // zero alloc
    else { Cow::Owned(input.to_uppercase()) }         // alloc only when needed
}
```

### `SmallVec` / `tinyvec` for small collections

When a collection is usually small (1-8 elements) but occasionally larger,
stack-allocate the common case and spill to heap only when needed:

```rust
use smallvec::SmallVec;
fn children(node: &Node) -> SmallVec<[&Node; 4]> { ... }
```

---

## Async & tokio — don't block the runtime

tokio uses a thread pool with work-stealing. **Blocking work freezes the
runtime** — one blocked task starves others on the same worker.

### Never block in async

```rust
// ❌ std::sync::Mutex held across .await — can deadlock, definitely stalls
async fn bad(state: &Mutex<Shared>) {
    let guard = state.lock().unwrap();
    do_async_work().await;  // guard held across suspension
}

// ✅ Scope the guard, or use tokio::sync::Mutex if you must hold across .await
async fn good(state: &Mutex<Shared>) {
    let data = { state.lock().unwrap().clone() };  // quick scope
    do_async_work(&data).await;
}
```

For CPU-bound work (parsing, hashing, compression), use `spawn_blocking`:

```rust
// ❌ Blocks the async runtime thread
async fn hash_large(data: Vec<u8>) -> String {
    sha2::Sha256::digest(&data).to_string()  // CPU-bound on async thread
}

// ✅ Offloads to the blocking pool
async fn hash_large(data: Vec<u8>) -> String {
    tokio::task::spawn_blocking(move || {
        sha2::Sha256::digest(&data).to_string()
    }).await.unwrap()
}
```

### Bounded channels — backpressure is a feature

`mpsc::channel` is unbounded by default — the sender can outpace the receiver
and exhaust memory. Use bounded channels for backpressure:

```rust
// ✅ Bounded: receiver applies backpressure when slow
let (tx, mut rx) = tokio::sync::mpsc::channel::<StreamChunk>(32);

// The sender awaits on tx.send() when the buffer is full — natural backpressure
```

praxis uses `StreamReceiver = mpsc::Receiver<StreamChunk>` for LLM streaming.
Keep the bound small (16-64) so a slow consumer doesn't buffer unbounded.

### `join!` / `try_join!` for concurrent operations

```rust
// ✅ Both requests run concurrently, not sequentially
let (architect, coder) = tokio::join!(
    architect_agent.design(&goal),
    coder_agent.implement(&goal),
);
```

### Avoid `tokio::spawn` when you don't need detached tasks

`spawn` creates a detached task — you lose the ability to await it, and
panics are silently swallowed. Prefer `join!`/`try_join!` for scoped
concurrency. Use `spawn` only for fire-and-forget background work (like
`MemoryKeeper` indexing).

---

## ractor — actor throughput

praxis uses `ractor` for the actor model. Each agent (Architect, Coder,
Reviewer, etc.) is an actor with its own mailbox.

### Mailbox capacity

Actors process messages one at a time. If a slow agent (e.g., waiting on an
LLM API call) can't keep up, messages queue up. For long-running agents,
consider:
- Bounded mailboxes with backpressure (drop or reject when full).
- Splitting slow work into `spawn_blocking` so the actor stays responsive.

### Don't clone large state in messages

Actor messages are moved, not borrowed. If a message carries large data
(`Vec<String>`, full conversation history), consider `Arc<T>` to share
without copying:

```rust
// ❌ Clones the entire conversation on every message
struct ProcessPrompt { history: Vec<ChatMessage> }

// ✅ Reference-counted — no deep copy
struct ProcessPrompt { history: Arc<Vec<ChatMessage>> }
```

### Actor supervision — don't panic the system

`ractor` supervisors restart failed actors. But a panic in a handler is still
disruptive. Use `Result` returns and let the supervisor decide:

```rust
// ✅ Return errors, don't panic
fn handle(&mut self, msg: Task) -> Result<Output, ActorError> {
    let result = self.provider.chat(&msg.messages, &msg.config)?;
    Ok(result)
}
```

---

## SQLite — batch and pool

praxis uses `rusqlite` + `r2d2` for connection pooling + `refinery` for
migrations.

### Connection pooling

SQLite is single-writer. `r2d2` pools connections but only one can write at a
time. For read-heavy workloads, use WAL mode for concurrent readers:

```sql
PRAGMA journal_mode = WAL;     -- concurrent readers + one writer
PRAGMA synchronous = NORMAL;   -- WAL-safe, faster than FULL
PRAGMA mmap_size = 268435456;  -- 256 MB memory-mapped I/O
```

### Batch writes

```rust
// ❌ One transaction per insert — slow (fsync per commit)
for event in events {
    conn.execute("INSERT INTO events (...) VALUES (...)", params![...])?;
}

// ✅ One transaction for all inserts — single fsync
let tx = conn.transaction()?;
for event in &events {
    tx.execute("INSERT INTO events (...) VALUES (...)", params![...])?;
}
tx.commit()?;
```

### Prepared statements

`rusqlite` caches prepared statements, but for hot queries, prepare once:

```rust
let mut stmt = conn.prepare_cached("SELECT id, name FROM projects WHERE id = ?1")?;
let rows = stmt.query_map([id], |row| {
    Ok(Project { id: row.get(0)?, name: row.get(1)? })
})?;
```

### Don't hold transactions across `.await`

SQLite transactions are not async-aware. Do all DB work synchronously, then
return to async. If you need async + DB, use `spawn_blocking` for the DB
operation.

---

## Qdrant — vector search

praxis uses Qdrant for episodic memory (embeddings). Performance tips:

- **Batch upserts** — don't insert one vector at a time. Collect a batch and
  upsert together (reduces network round-trips).
- **Use filters** — narrow the search space before vector similarity. A
  `must` filter on `project_id` + `session_id` before the vector search is
  much faster than searching everything and filtering after.
- **Limit `top_k`** — only retrieve what you need. `top_k = 5` is usually
  enough for RAG context; don't retrieve 100 and discard 95.
- **Collection sharding** — for large deployments, shard by project_id so
  searches only hit one shard.

---

## moka cache — eviction and sizing

`moka` is a concurrent cache with time/size-based eviction. Used for hot
memory in praxis.

```rust
use moka::sync::Cache;

let cache: Cache<String, AgentState> = Cache::builder()
    .max_capacity(10_000)              // evict when exceeding 10K entries
    .time_to_live(Duration::from_secs(300))  // TTL: 5 min
    .build();
```

- **Size the cache to your working set** — too small = cache thrashing; too
  large = memory pressure. Monitor `entry_count()` and `weighted_size()`.
- **Use `get_with`** for coalescing concurrent loads — multiple requests for
  the same key share one computation:

```rust
let value = cache.get_with("key", async { expensive_load().await }).await;
```

---

## DashMap — concurrent hot state

`DashMap` uses sharded locks — much better throughput than `Mutex<HashMap>`
under contention.

```rust
use dashmap::DashMap;

let sessions: DashMap<Uuid, Session> = DashMap::new();
sessions.insert(id, session);         // write — locks one shard only
let session = sessions.get(&id);      // read — locks one shard only
```

- **Don't hold `DashMap` entries across `.await`** — the `Ref` guard holds a
  shard lock. Clone the value out if you need it across an await:

```rust
// ❌ Holds shard lock across await
let session = sessions.get(&id).unwrap();
do_async(&session).await;  // shard locked during await

// ✅ Clone out, release the lock
let session = sessions.get(&id).unwrap().clone();
drop(session);  // explicit release
do_async(&session).await;
```

- **`entry()` for atomic get-or-insert** — avoids TOCTOU races:

```rust
let session = sessions.entry(id).or_insert_with(|| Session::new());
```

---

## Clippy perf lints

Enable `clippy::perf` (warn by default in the pedantic group). Key lints:

| Lint | What it catches |
|------|-----------------|
| `redundant_clone` | `.clone()` where a borrow works |
| `box_collection` | `Box<Vec<_>>` — pointless indirection |
| `unnecessary_to_owned` | `.to_owned()` where `&str` works |
| `large_enum_variant` | One enum variant much larger than others — box it |
| `inefficient_to_string` | `.to_string()` on `&str` where `.into()` works |
| `vec_box` | `Vec<Box<T>>` — just `Vec<T>` (Vec is already heap) |

---

## Benchmarking

Use `criterion` (already in workspace deps) for microbenchmarks:

```rust
use criterion::{criterion_group, criterion_main, Criterion};

fn bench_parse(c: &mut Criterion) {
    let input = include_str!("../fixtures/sample.json");
    c.bench_function("parse_config", |b| {
        b.iter(|| toml::from_str::<Config>(criterion::black_box(input)))
    });
}

criterion_group!(benches, bench_parse);
criterion_main!(benches);
```

Run: `cargo bench`. Criterion gives you statistical comparison (regression
detection) across runs.

### Profiling

- **`cargo flamegraph`** — visual flame graphs of CPU time. Install with
  `cargo install flamegraph`, run `cargo flamegraph --bench ...`.
- **`perf record` / `perf report`** (Linux) — sampling profiler.
- **`cargo instruments`** (macOS) — Xcode Instruments integration.
- **`tokio-console`** — live tokio task inspector (spawned tasks, poll times,
  scheduling delays). Essential for async debugging.

### Memory profiling

- **`valgrind --tool=massif`** (Linux) — heap profiling.
- **`dhat`** — Rust memory profiler (`dhat-rs` crate) — tracks allocations
  in-code.
- **`jemalloc`** stats — swap the allocator and read stats:

```rust
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;
```

---

## Quick checklist

- [ ] Release profile: `opt-level = 3`, `lto = true`, `codegen-units = 1`, `strip = true`
- [ ] No `unwrap()`/`expect()` in hot paths (they add panic-check overhead)
- [ ] `Vec::with_capacity(n)` when size is known
- [ ] `&str`/`&[T]` params, not `String`/`Vec<T>`
- [ ] No `MutexGuard` held across `.await`
- [ ] CPU-bound work in `spawn_blocking`
- [ ] Bounded channels for backpressure
- [ ] SQLite: WAL mode, batch writes, prepared statements
- [ ] Qdrant: batch upserts, filter before vector search, small `top_k`
- [ ] DashMap: don't hold `Ref` across `.await`
- [ ] `Arc<T>` for large shared state, not `clone()`
- [ ] `criterion` benchmarks for hot paths
