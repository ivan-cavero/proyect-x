---
name: rust-best-practices
description: Write idiomatic, modern Rust targeting Edition 2024 and the nightly toolchain (mid-2026, Rust 1.96+). Use whenever writing, reviewing, or refactoring any Rust code or .rs file, setting up Cargo.toml, or working with async, unsafe, traits, error handling, concurrency, lifetimes, clippy, or testing. Also use when the user mentions rustc, cargo, clippy, rustup, tokio, serde, anyhow, thiserror, axum, or asks about modern Rust idioms, Edition 2024, or nightly features. Use even when the user just says "write a Rust function" or "fix this Rust code" — modern idioms should always apply.
---

# Modern Rust Best Practices

Baseline: **Edition 2024** (shipped Rust 1.85, Feb 2025) on the **nightly** channel
(latest stable Rust 1.96, May 2026). Every idiom below is stable unless tagged
**[nightly]** — emit nightly-only code only when the project already opts into
nightly (`rust-toolchain.toml` with `channel = "nightly"`), and call it out
explicitly so it doesn't silently fail to compile on stable.

The guiding idea: Rust moves fast, and the "obvious" way from a few years ago is
often superseded by something safer and more ergonomic in the language itself.
Prefer std/built-in features over external crates for the common cases, reach
for the ecosystem where std is genuinely missing, and let the borrow checker
and clippy do their job rather than fighting them.

---

## Toolchain baseline

Every new project starts here. These unlock the modern defaults (let chains,
the new RPIT capture rules, `unsafe` safety) and make linting declarative.

```toml
# Cargo.toml
[package]
edition = "2024"
rust-version = "1.85"          # MSRV — be honest about it

[lints.rust]
unsafe_code = "forbid"         # or "deny" if you genuinely need unsafe

[lints.clippy]
pedantic = { level = "warn", priority = -1 }
nursery  = { level = "warn", priority = -1 }
# Opt out the noisy ones per-project:
needless_pass_by_value = "allow"
must_use_candidate     = "allow"
```

Why `pedantic`/`nursery` at `warn`: they catch real mistakes (redundant clones,
needless returns, questionable APIs) without breaking the build. Setting
`priority = -1` lets individual lints override the group. Pin a toolchain with
`rust-toolchain.toml` so CI and contributors match:

```toml
# rust-toolchain.toml
[toolchain]
channel = "nightly-2026-06-01"
components = ["rustfmt", "clippy", "rust-src", "miri"]
```

---

## Modern syntax — reach for these

These replace older, more verbose patterns. Using them signals current Rust.

**`let` chains** (stable 1.88, Edition 2024 only) flatten nested `if let`:

```rust
// ❌ Nested pyramids
if let Some(cfg) = load() {
    if let Ok(url) = cfg.parse() {
        if url.host() == "localhost" { /* ... */ }
    }
}

// ✅ One flat condition; earlier bindings are in scope later
if let Some(cfg) = load()
    && let Ok(url) = cfg.parse()
    && url.host() == "localhost"
{
    serve(url);
}
```

**`let else`** (stable 1.65) for early-return destructuring:

```rust
let Some(user) = lookup(id) else { return Err(NotFound); };
```

**Inline const** (stable 1.79) — a const expression that can use in-scope
generics, unlike a `const` item:

```rust
fn none_array<T, const N: usize>() -> [Option<T>; N] {
    [const { None }; N]
}
```

**`cfg_select!`** (stable 1.95) replaces the `cfg-if` crate for multi-arm
compile-time branching:

```rust
cfg_select! {
    unix => { fn path_sep() -> &'static str { "/" } }
    windows => { fn path_sep() -> &'static str { "\\" } }
    _ => { fn path_sep() -> &'static str { "/" } }
}
```

**`if let` guards in `match`** (stable 1.95) — combine a pattern guard with a
sub-pattern binding:

```rust
match request {
    Some(r) if let Ok(body) = r.body() => handle(body),
    _ => retry(),
}
```

Other small wins: exclusive ranges in patterns (`1..10`), omitting uninhabited
`Err` arms when the error type is `!`/uninhabited, `Option::is_none_or`,
`iter::repeat_n`, `[T]::is_sorted`.

---

## `unsafe` in Edition 2024

Edition 2024 made `unsafe` explicit everywhere it matters. The philosophy:
`unsafe` is a contract you're upholding, so the compiler makes you spell out
each place you rely on it — this makes audits tractable.

```rust
// Inner unsafe block now REQUIRED inside unsafe fn (unsafe_op_in_unsafe_fn)
unsafe fn write_val(ptr: *mut u32) {
    unsafe { *ptr = 42; }
}

// extern blocks must be `unsafe extern`; items declare their own safety
unsafe extern "C" {
    pub safe fn sqrt(x: f64) -> f64;        // safe to call
    pub unsafe fn strlen(p: *const u8) -> usize;
}

// Attributes that affect linking must be `#[unsafe(...)]`
#[unsafe(no_mangle)]
pub extern "C" fn entry() {}
```

**Raw pointers without forming a reference** — use the native `&raw` syntax
(stable 1.82) for `repr(packed)` and unaligned data. The `addr_of!` macros
still work but `&raw` is the preferred form:

```rust
#[repr(packed)]
struct Header { flags: u32 }
let h = Header { flags: 0 };
let ptr = &raw const h.flags;              // not &h.flags (would be UB)
let flags = unsafe { ptr.read_unaligned() };
```

**Provenance API** (stable 1.84) — pointers carry more than an address. Avoid
`as usize`/`as *const T` round-trips; use provenance-preserving methods for
tagged pointers and mmap tricks:

```rust
let tagged = ptr.map_addr(|a| a | 1);            // set low bit, keep provenance
let clean = tagged.with_addr(tagged.addr() & !1);
```

**Uninitialized memory** — always `MaybeUninit<T>`, never `mem::uninitialized()`
(that's UB and long-deprecated). `Box::new_zeroed` / `Rc::new_zeroed` (1.92)
for heap zeroed alloc.

**No references to `static mut`** — Edition 2024 makes `&static_mut` a hard
error. Use raw pointers (`&raw mut STATIC`), atomics, or `OnceLock`/`LazyLock`
instead. Mutable globals are almost always the wrong tool.

---

## Async Rust

**`async fn` in traits (AFIT)** is stable (1.75). Use it directly; drop
`#[async_trait]` unless you need `dyn` dispatch or support Rust < 1.75.

```rust
trait Fetcher {
    async fn fetch(&self, url: &str) -> Bytes;
}
```

Two real limitations to know:
- **Not object-safe** — no `dyn Fetcher` for traits with `async fn`. Need
  `dyn`? Use `#[async_trait]`.
- **Callers can't add `Send` bounds** on the returned future. For *public*
  traits that must be usable across executors, derive a Send variant with
  `trait-variant`:

```rust
#[trait_variant::make(Fetcher: Send)]
pub trait LocalFetcher {
    async fn fetch(&self, url: &str) -> Bytes;
} // produces both LocalFetcher and a Send Fetcher
```

**Async closures** (stable 1.85) — `async || { ... }` can borrow from captures,
which the older `|| async { ... }` workaround could not:

```rust
let mut buf = Vec::new();
let push = async || { buf.push(ready(0).await); };
```

**Cancellation safety** — dropping a future is always safe, but holding a
non-async-aware lock guard across `.await` is a classic deadlock. Clippy's
`await_holding_lock` catches it; scope the guard instead:

```rust
// ❌ guard lives across the suspension point
async fn bad(m: &Mutex<u32>) {
    let mut g = m.lock().unwrap();
    *g += 1;
    do_io().await;            // g still held — can deadlock
}

// ✅ release before awaiting
async fn good(m: &Mutex<u32>) {
    { let mut g = m.lock().unwrap(); *g += 1; }
    do_io().await;
}
```

**Stack pinning** — use the std `pin!` macro (stable 1.68), not `pin_utils`:

```rust
let fut = std::pin::pin!(some_async());
fut.as_mut().poll(&mut cx);
```

`async fn main` is **not** in std — use a runtime macro (`#[tokio::main]`).

---

## Error handling

The two-tier pattern is still the standard, and it's worth following because it
keeps library APIs typed while letting applications stay ergonomic.

- **Libraries** → `thiserror` with `#[derive(Error)]`. Typed errors in your
  public API; `#[from]` generates the `From` impl so `?` just works.
- **Applications** → `anyhow` with `.context()`. `anyhow::Result<T>`,
  backtraces, downcasting, source chains.

```rust
// Library
#[derive(thiserror::Error, Debug)]
pub enum StoreError {
    #[error("disconnected")]
    Disconnect(#[from] io::Error),
    #[error("bad header (expected {expected}, found {found})")]
    InvalidHeader { expected: u32, found: u32 },
}

// Application
fn load_config(path: &Path) -> anyhow::Result<Config> {
    let raw = fs::read(path).with_context(|| format!("read {path:?}"))?;
    Ok(serde_json::from_str(&raw).context("parse config")?)
}

fn main() -> anyhow::Result<()> { load_config(&path)?; Ok(()) }
```

`fn main() -> Result` uses the `Termination` trait (stable 1.26) — prefer it
over manual `unwrap`/`expect` at the top level. `Result::flatten` (1.89) helps
when nested results share an error type. Prefer `anyhow::Error` over
`Box<dyn Error>` — anyhow gives context and backtraces; `Box<dyn Error>` is
legacy.

`core::error::Error` (stable 1.81) means `#![no_std]` crates share one `Error`
trait with the rest of the ecosystem — no more `std::error::Error` vs
`core::error::Error` split.

---

## Concurrency

**Lazy initialization** — `std::sync::LazyLock` (1.80) and `OnceLock` (1.70)
replace `lazy_static!` and `once_cell` for the common cases. `LazyLock` is
thread-safe (use in `static`); `LazyCell` is single-threaded (good for
`thread_local!`):

```rust
static START: LazyLock<Instant> = LazyLock::new(Instant::now);
static CONFIG: LazyLock<Config> = LazyLock::from(Config::load()); // 1.96 From impl
```

**Scoped threads** (`thread::scope`, stable 1.63) let spawned threads borrow
non-`'static` locals safely — no `Arc` needed for borrowed data:

```rust
std::thread::scope(|s| {
    let v = &mut data;
    s.spawn(|| process(v));   // borrows v
    s.spawn(|| process(v));
}); // joins both before returning
```

**Locks** — std `Mutex`/`RwLock` poison on panic (a deliberate safety signal).
`parking_lot` is faster, smaller, and doesn't poison, at the cost of a
dependency. Reach for `parking_lot` in hot paths; std is fine otherwise.
`RwLockWriteGuard::downgrade` is now in std (1.92).

**Concurrent maps** — `Mutex<HashMap>` is fine at low contention. Under real
contention use `DashMap` (sharded locks). `arc_with_non_send_sync` clippy lint
catches `Arc` wrapping `!Send`/`!Sync` (usually you wanted `Rc`).

**Atomics** — default to `Acquire`/`Release` pairs for publishing data,
`Relaxed` for standalone counters, `SeqCst` only when a global total order
matters (rare). `AtomicUsize::update`/`try_update` (1.95) make CAS loops
ergonomic.

---

## Collections & iteration

**Return `impl Iterator`** — with Edition 2024's RPIT capture rules this is
ergonomic and zero-cost; avoid `Vec` when the caller only iterates:

```rust
fn evens(up_to: usize) -> impl Iterator<Item = usize> + use<'_> {
    (0..up_to).filter(|x| x % 2 == 0)
}
```

**Hashers** — `HashMap`'s default SipHash-1-3 is DoS-resistant but slow. For
trusted keys use `ahash` (good default) or `fxhash`/`rustc-hash` (fastest).
`BuildHasherDefault::new` (1.85) makes construction clean.

**Drain while iterating** — `Vec::extract_if` (1.87) and
`HashMap::extract_if` (1.88) remove matching elements and yield them, without
the borrow-checker fight of `retain` + manual collect.

**Small collections** — `SmallVec`/`tinyvec` (stack-then-spill) when you have
many small vectors; `tinyvec` is `unsafe`-free. `Box<Vec<_>>` is pointless
indirection (`box_collection` lint).

**`itertools`** is the standard `Iterator` extension — `tuple_windows`,
`chunks`, `group_by`, `dedup_by`. Reach for it before hand-rolling.

---

## Lifetimes & the borrow checker

**RPIT capture rules** (Edition 2024) — `-> impl Trait` now captures *all*
in-scope generic types AND lifetimes by default (it used to capture only
types). This is usually what you want. Opt out explicitly with `use<..>` when a
return type shouldn't borrow from an input:

```rust
// Captures only T, not the slice's lifetime — caller knows it's owned-safe
fn indices<T>(slice: &[T]) -> impl Iterator<Item = usize> + use<T> {
    0..slice.len()
}
```

**Elide with `'_`** — the `mismatched_lifetime_syntaxes` lint (1.89) flags
hidden lifetimes in return type paths; write `Iter<'_, u8>` not `Iter<u8>`.

**NLL** is fully stable (since 1.63) — the borrow checker understands control
flow, not just lexical scope. You rarely need to restructure for it.

**Polonius** **[nightly]** — the next-gen borrow checker, enabled with
`-Z polonius`. It accepts more programs (e.g. the classic nested-borrow
pattern NLL rejects). It is **alpha, not the default, not for production** as
of mid-2026. Don't emit code that only compiles under Polonius unless the user
asks for it.

---

## Clippy & lints

Prefer **`#[expect(...)]`** (stable 1.81) over `#[allow(...)]`. `expect`
suppresses the lint but **warns if the lint stops firing** — so when you later
refactor and the suppression becomes dead, you find out. `allow` silently
hides it forever. Add a `reason` so the next reader knows why:

```rust
#[expect(clippy::unnecessary_wraps, reason = "stable public API shape")]
fn helper() -> Option<i32> { Some(42) }
```

Lints worth knowing by name (they catch real bugs):
- `await_holding_lock` / `await_holding_refcell_ref` — locks across `.await`.
- `arc_with_non_send_sync` — `Arc` of non-`Send`/`Sync` (wanted `Rc`).
- `box_collection` — `Box<Vec<_>>` pointless indirection.
- `redundant_clone` / `assigning_clones` — needless `.clone()`.
- `borrow_as_ptr` — suggests `&raw const`/`&raw mut`.
- `ptr_arg` — `&Vec<T>`/`&String` where `&[T]`/`&str` suffices.

Don't blanket-enable `clippy::restriction` — it lints *against* idiomatic Rust
in places. Enable restriction lints individually when you have a reason.

---

## Testing

- **`cargo nextest`** is the preferred runner — up to 3× faster, per-test
  process isolation, retries, sharding, JUnit XML. Doctests still run via
  `cargo test --doc`.
- **`rstest`** for parametrized/table tests (`#[rstest]` + `#[case]`).
- **`proptest`** for property-based testing.
- **`insta`** for snapshot testing (serialization, CLI output).
- **`mockall`** to mock trait impls.
- **`pretty_assertions`** for colored `assert_eq!` diffs.
- **`assert_matches!`** (std, 1.96) — not in the prelude, import explicitly:
  `use core::assert_matches;`
- **`#[track_caller]`** on helper functions that panic — points the blame at
  the caller, not the helper.

---

## Project structure

**File-based modules** — use `foo.rs`, not `foo/mod.rs`. The `mod.rs` style is
legacy since Edition 2018 and still works, but `foo.rs` is the modern default:

```
src/
  lib.rs
  models.rs          // declares `pub mod user; pub mod order;`
  models/
    user.rs          // NOT models/user/mod.rs
    order.rs
```

**Re-export at the crate root** for a clean public API:

```rust
// lib.rs
pub mod models;
pub use models::{User, Order};   // callers write `crate::User`, not `crate::models::User`
```

**Workspaces** — share deps and lints via `[workspace.dependencies]` and
`[workspace.lints]`; inherit with `dep.workspace = true`.

**Feature flags** — make them additive (gate features *in*, never *out*), use
`dep:crate` to avoid implicit feature names, and declare custom cfgs with
`cargo::rustc-check-cfg` so the `unexpected_cfgs` lint (1.80) doesn't false-
positive. Avoid mutually-exclusive features.

---

## Performance idioms

- **`&str` over `String`** in function params; return `String` only when you
  must allocate. `impl AsRef<str>` for flexibility.
- **`Cow<'_, T>`** for clone-on-write — zero-copy when no mutation is needed.
- **`Vec::with_capacity(n)`** when the size is known — avoids reallocation
  churn.
- **`Arc` vs `Rc`** — `Rc` single-threaded, `Arc` multi-threaded. The compiler
  (via `Send`/`Sync`) and clippy keep you honest.
- **`#[inline]`** sparingly — only for small hot functions in libraries
  consumed across crates. Let the optimizer handle the rest.
- **`std::hint`** — `black_box` (prevent over-optimization in benches),
  `select_unpredictable` (1.88), `cold_path` (1.95),
  `assert_unchecked` (1.81, unsafe "assume true").
- **Zero-copy parsing** — slice into `&[u8]`/`&str`; `nom`/`winnow` for parsers.

---

## Nightly-only features — mark clearly

Only emit these when the project is on nightly AND the user wants them. Always
note they're nightly so the code isn't assumed stable.

| Feature | Status | How to enable |
|---------|--------|---------------|
| **Polonius** borrow checker | nightly alpha, not default | `-Z polonius` |
| **`cargo -Zscript`** single-file `.rs` | nightly | `cargo +nightly -Zscript file.rs` |
| **`gen` blocks** | keyword reserved (Edition 2024), blocks unstable | `#![feature(gen_blocks)]` |
| **`allocator_api`** custom allocators | nightly | `#![feature(allocator_api)]` |
| **Never type `!`** | not fully stable; deny-by-default lints paving the way | — |
| **Async drop** | no stable language feature | patterns only |

---

## Anti-patterns to avoid

| Anti-pattern | Do this instead | Caught by |
|---|---|---|
| `unwrap()`/`expect()` in non-test code | `?`, `Result`, `.context()` | `clippy::unwrap_used` |
| Needless `.clone()` | borrow, `Cow`, `clone_from` | `redundant_clone`, `assigning_clones` |
| `Box<dyn Error>` | `anyhow::Error` or typed errors | — |
| `String`/`Vec<T>` param where `&str`/`&[T]` works | borrow slices | `ptr_arg` |
| `Vec` return where caller only iterates | `impl Iterator` | — |
| `Mutex<HashMap>` under contention | `DashMap` | review |
| `Box<Vec<_>>` / `Box<collection>` | unboxed collection | `box_collection` |
| `&expr as *const T` | `&raw const expr` | `borrow_as_ptr` |
| `#[allow(...)]` (silent) | `#[expect(..., reason = "..")]` | `allow_attributes` |
| `MutexGuard` held across `.await` | scope it before awaiting | `await_holding_lock` |
| `Arc<RefCell<_>>` | `Arc<Mutex<_>>` or `Rc<RefCell<_>>` | `arc_with_non_send_sync` |
| `mod.rs` | `foo.rs` | style convention |
| `extern {}` without ABI | `extern "C" {}` | `missing_abi` |
| `lazy_static!` / `once_cell` (simple cases) | `LazyLock` / `OnceLock` (std) | — |
| `cfg-if` crate (simple cases) | `cfg_select!` (1.95) | — |
| `mem::uninitialized()` | `MaybeUninit<T>` | UB; long-deprecated |
| `&static_mut` references | raw pointers / `&raw mut` / atomics | Edition 2024 hard error |

---

## Tooling checklist (CI standard)

- `cargo fmt --check` — formatting (style editions in 2024).
- `cargo clippy --all-targets -- -D warnings` — lint, fail on warnings.
- `cargo nextest run` — fast test runner (doctests: `cargo test --doc`).
- `cargo deny check` — license + advisories + bans in one pass.
- `cargo machete` — find unused dependencies.
- `cargo binstall` — install pre-built binaries (skip compile).
- `cargo mutants` (optional) — mutation testing for test quality.
- `miri` (nightly) — UB detection for `unsafe` code via `cargo +nightly miri test`.

---

## Quick decision guide

- **New project?** `edition = "2024"`, set `rust-version`, add the `[lints]`
  block, pick `tokio` only if you actually need async.
- **Public async trait?** `#[trait_variant::make(..: Send)]`; need `dyn`? `#[async_trait]`.
- **Errors?** Library → `thiserror`; app → `anyhow` + `fn main() -> Result`.
- **Global init?** `LazyLock`/`OnceLock` (not `lazy_static`).
- **Raw pointer to packed field?** `&raw const`/`&raw mut` (not `&`).
- **Suppress a lint?** `#[expect(.., reason = "..")]` (not `#[allow]`).
- **Nightly feature?** Tag it `[nightly]` and confirm the project is on nightly.
