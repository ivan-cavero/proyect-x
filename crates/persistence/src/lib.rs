//! Persistence layer — event store implementations.
//!
//! Default: SQLite (embedded, WAL mode, zero configuration)
//! Optional: PostgreSQL (for VPS / production deployments)

pub mod sqlite;

pub use sqlite::SqliteEventStore;