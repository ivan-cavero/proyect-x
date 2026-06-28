//! SQLite event store implementation.
//!
//! WAL mode, connection pooling, append-only event log, snapshot support.
//! All queries are parameterized (no SQL injection).

use async_trait::async_trait;
use project_x_agent_traits::persistence::{EventStore, StoredEvent, StoredSnapshot};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::params;
use std::path::Path;
use std::sync::Mutex;
use uuid::Uuid;

/// SQLite-backed event store with WAL mode and connection pooling.
pub struct SqliteEventStore {
    pool: Pool<SqliteConnectionManager>,
    /// Mutex for write operations (SQLite WAL allows concurrent reads but serial writes).
    write_lock: Mutex<()>,
}

impl SqliteEventStore {
    /// Create a new SQLite event store from a file path.
    ///
    /// Creates the file and runs migrations if it doesn't exist.
    pub fn new(path: &Path) -> Result<Self, String> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create directory: {}", e))?;
        }

        let manager = SqliteConnectionManager::file(path)
            .with_init(|conn| {
                // Enable WAL mode for better concurrency
                conn.execute_batch("PRAGMA journal_mode=WAL;")?;
                // Busy timeout: wait 5s if locked
                conn.execute_batch("PRAGMA busy_timeout=5000;")?;
                // Enable foreign keys
                conn.execute_batch("PRAGMA foreign_keys=ON;")?;
                // Normal synchronous (good balance of safety/speed)
                conn.execute_batch("PRAGMA synchronous=NORMAL;")?;
                Ok(())
            });

        let pool = Pool::builder()
            .max_size(10)
            .build(manager)
            .map_err(|e| format!("Failed to create pool: {}", e))?;

        let store = Self {
            pool,
            write_lock: Mutex::new(()),
        };

        // Run migrations
        store.run_migrations()?;

        Ok(store)
    }

    /// Create an in-memory SQLite store (for testing).
    pub fn in_memory() -> Result<Self, String> {
        let manager = SqliteConnectionManager::memory()
            .with_init(|conn| {
                conn.execute_batch("PRAGMA journal_mode=WAL;")?;
                conn.execute_batch("PRAGMA foreign_keys=ON;")?;
                Ok(())
            });

        let pool = Pool::builder()
            .max_size(1)
            .build(manager)
            .map_err(|e| format!("Failed to create in-memory pool: {}", e))?;

        let store = Self {
            pool,
            write_lock: Mutex::new(()),
        };

        store.run_migrations()?;
        Ok(store)
    }

    /// Run SQL migrations to create tables.
    fn run_migrations(&self) -> Result<(), String> {
        let conn = self.pool.get().map_err(|e| format!("Pool error: {}", e))?;

        conn.execute_batch(
            "
            -- Events table (append-only)
            CREATE TABLE IF NOT EXISTS events (
                id              TEXT PRIMARY KEY,
                aggregate_id    TEXT NOT NULL,
                aggregate_type  TEXT NOT NULL,
                event_type      TEXT NOT NULL,
                payload         TEXT NOT NULL,
                metadata        TEXT NOT NULL DEFAULT '{}',
                version         INTEGER NOT NULL,
                created_at      TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_events_aggregate
                ON events(aggregate_id, version);

            CREATE INDEX IF NOT EXISTS idx_events_type
                ON events(event_type);

            CREATE INDEX IF NOT EXISTS idx_events_created
                ON events(created_at);

            -- Snapshots table (latest state per aggregate)
            CREATE TABLE IF NOT EXISTS snapshots (
                aggregate_id    TEXT PRIMARY KEY,
                aggregate_type  TEXT NOT NULL,
                state           TEXT NOT NULL,
                version         INTEGER NOT NULL,
                updated_at      TEXT NOT NULL
            );

            -- Sessions table
            CREATE TABLE IF NOT EXISTS sessions (
                id              TEXT PRIMARY KEY,
                project_id      TEXT,
                status          TEXT NOT NULL DEFAULT 'active',
                goal            TEXT,
                current_phase   TEXT,
                iteration       INTEGER DEFAULT 0,
                created_at      TEXT NOT NULL
            );

            -- Projects table
            CREATE TABLE IF NOT EXISTS projects (
                id              TEXT PRIMARY KEY,
                name            TEXT NOT NULL,
                path            TEXT NOT NULL,
                created_at      TEXT NOT NULL
            );

            -- Drift history table
            CREATE TABLE IF NOT EXISTS drift_history (
                id              TEXT PRIMARY KEY,
                session_id      TEXT NOT NULL,
                agent_id        TEXT,
                asi_score       REAL NOT NULL,
                dimensions      TEXT NOT NULL,
                recorded_at     TEXT NOT NULL
            );

            -- Context snapshots table
            CREATE TABLE IF NOT EXISTS context_snapshots (
                id              TEXT PRIMARY KEY,
                session_id      TEXT NOT NULL,
                iteration       INTEGER NOT NULL,
                budget_snapshot TEXT NOT NULL,
                compression_log TEXT NOT NULL,
                pressure_before REAL,
                pressure_after  REAL,
                created_at      TEXT NOT NULL
            );
            ",
        )
        .map_err(|e| format!("Migration error: {}", e))?;

        Ok(())
    }

    /// Get a connection from the pool.
    fn conn(&self) -> Result<r2d2::PooledConnection<SqliteConnectionManager>, String> {
        self.pool.get().map_err(|e| format!("Pool error: {}", e))
    }
}

#[async_trait]
impl EventStore for SqliteEventStore {
    /// Append a new event (with version conflict detection).
    async fn append(&self, event: StoredEvent) -> project_x_shared::error::Result<()> {
        let _lock = self.write_lock.lock().map_err(|e| {
            project_x_shared::error::ProjectXError::DatabaseError(format!("Lock error: {}", e))
        })?;

        let conn = self.conn().map_err(|e| {
            project_x_shared::error::ProjectXError::DatabaseError(e)
        })?;

        // Check version conflict
        let max_version: i64 = conn
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM events WHERE aggregate_id = ?1",
                params![event.aggregate_id.to_string()],
                |row| row.get(0),
            )
            .map_err(|e| {
                project_x_shared::error::ProjectXError::DatabaseError(format!("Query error: {}", e))
            })?;

        if event.version <= max_version {
            return Err(project_x_shared::error::ProjectXError::DatabaseError(format!(
                "Version conflict: event version {} <= max version {}",
                event.version, max_version
            )));
        }

        conn.execute(
            "INSERT INTO events (id, aggregate_id, aggregate_type, event_type, payload, metadata, version, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                event.id.to_string(),
                event.aggregate_id.to_string(),
                event.aggregate_type,
                event.event_type,
                event.payload.to_string(),
                event.metadata.to_string(),
                event.version,
                event.created_at,
            ],
        )
        .map_err(|e| {
            project_x_shared::error::ProjectXError::DatabaseError(format!("Insert error: {}", e))
        })?;

        Ok(())
    }

    /// Read all events for an aggregate, optionally starting from a version.
    async fn read_events(
        &self,
        aggregate_id: Uuid,
        after_version: Option<i64>,
    ) -> project_x_shared::error::Result<Vec<StoredEvent>> {
        let conn = self.conn().map_err(|e| {
            project_x_shared::error::ProjectXError::DatabaseError(e)
        })?;

        let rows = if let Some(version) = after_version {
            let mut stmt = conn
                .prepare(
                    "SELECT id, aggregate_id, aggregate_type, event_type, payload, metadata, version, created_at
                     FROM events
                     WHERE aggregate_id = ?1 AND version > ?2
                     ORDER BY version ASC",
                )
                .map_err(|e| {
                    project_x_shared::error::ProjectXError::DatabaseError(format!("Prepare error: {}", e))
                })?;

            stmt.query_map(params![aggregate_id.to_string(), version], |row| {
                Ok(StoredEvent {
                    id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap_or_default(),
                    aggregate_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap_or_default(),
                    aggregate_type: row.get(2)?,
                    event_type: row.get(3)?,
                    payload: serde_json::from_str(&row.get::<_, String>(4)?).unwrap_or_default(),
                    metadata: serde_json::from_str(&row.get::<_, String>(5)?).unwrap_or_default(),
                    version: row.get(6)?,
                    created_at: row.get(7)?,
                })
            })
            .map_err(|e| {
                project_x_shared::error::ProjectXError::DatabaseError(format!("Query error: {}", e))
            })?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| {
                project_x_shared::error::ProjectXError::DatabaseError(format!("Row error: {}", e))
            })?
        } else {
            let mut stmt = conn
                .prepare(
                    "SELECT id, aggregate_id, aggregate_type, event_type, payload, metadata, version, created_at
                     FROM events
                     WHERE aggregate_id = ?1
                     ORDER BY version ASC",
                )
                .map_err(|e| {
                    project_x_shared::error::ProjectXError::DatabaseError(format!("Prepare error: {}", e))
                })?;

            stmt.query_map(params![aggregate_id.to_string()], |row| {
                Ok(StoredEvent {
                    id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap_or_default(),
                    aggregate_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap_or_default(),
                    aggregate_type: row.get(2)?,
                    event_type: row.get(3)?,
                    payload: serde_json::from_str(&row.get::<_, String>(4)?).unwrap_or_default(),
                    metadata: serde_json::from_str(&row.get::<_, String>(5)?).unwrap_or_default(),
                    version: row.get(6)?,
                    created_at: row.get(7)?,
                })
            })
            .map_err(|e| {
                project_x_shared::error::ProjectXError::DatabaseError(format!("Query error: {}", e))
            })?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| {
                project_x_shared::error::ProjectXError::DatabaseError(format!("Row error: {}", e))
            })?
        };

        Ok(rows)
    }

    /// Get the latest snapshot for an aggregate.
    async fn get_snapshot(&self, aggregate_id: Uuid) -> project_x_shared::error::Result<Option<StoredSnapshot>> {
        let conn = self.conn().map_err(|e| {
            project_x_shared::error::ProjectXError::DatabaseError(e)
        })?;

        let result = conn
            .query_row(
                "SELECT aggregate_id, aggregate_type, state, version, updated_at
                 FROM snapshots WHERE aggregate_id = ?1",
                params![aggregate_id.to_string()],
                |row| {
                    Ok(StoredSnapshot {
                        aggregate_id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap_or_default(),
                        aggregate_type: row.get(1)?,
                        state: serde_json::from_str(&row.get::<_, String>(2)?).unwrap_or_default(),
                        version: row.get(3)?,
                        updated_at: row.get(4)?,
                    })
                },
            )
            .optional()
            .map_err(|e| {
                project_x_shared::error::ProjectXError::DatabaseError(format!("Snapshot query error: {}", e))
            })?;

        Ok(result)
    }

    /// Save (upsert) a snapshot for an aggregate.
    async fn save_snapshot(&self, snapshot: StoredSnapshot) -> project_x_shared::error::Result<()> {
        let _lock = self.write_lock.lock().map_err(|e| {
            project_x_shared::error::ProjectXError::DatabaseError(format!("Lock error: {}", e))
        })?;

        let conn = self.conn().map_err(|e| {
            project_x_shared::error::ProjectXError::DatabaseError(e)
        })?;

        conn.execute(
            "INSERT OR REPLACE INTO snapshots (aggregate_id, aggregate_type, state, version, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                snapshot.aggregate_id.to_string(),
                snapshot.aggregate_type,
                snapshot.state.to_string(),
                snapshot.version,
                snapshot.updated_at,
            ],
        )
        .map_err(|e| {
            project_x_shared::error::ProjectXError::DatabaseError(format!("Snapshot save error: {}", e))
        })?;

        Ok(())
    }

    /// List all aggregate IDs of a given type.
    async fn list_aggregates(&self, aggregate_type: &str) -> project_x_shared::error::Result<Vec<Uuid>> {
        let conn = self.conn().map_err(|e| {
            project_x_shared::error::ProjectXError::DatabaseError(e)
        })?;

        let ids = conn
            .prepare("SELECT DISTINCT aggregate_id FROM events WHERE aggregate_type = ?1")
            .map_err(|e| {
                project_x_shared::error::ProjectXError::DatabaseError(format!("Prepare error: {}", e))
            })?
            .query_map(params![aggregate_type], |row| {
                let id_str: String = row.get(0)?;
                Ok(Uuid::parse_str(&id_str).unwrap_or_default())
            })
            .map_err(|e| {
                project_x_shared::error::ProjectXError::DatabaseError(format!("Query error: {}", e))
            })?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| {
                project_x_shared::error::ProjectXError::DatabaseError(format!("Row error: {}", e))
            })?;

        Ok(ids)
    }
}

// ─── Extension trait for rusqlite optional ─────────────────────

trait OptionalExt<T> {
    fn optional(self) -> Result<Option<T>, rusqlite::Error>;
}

impl<T> OptionalExt<T> for Result<T, rusqlite::Error> {
    fn optional(self) -> Result<Option<T>, rusqlite::Error> {
        match self {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }
}

// ─── Tests ────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn test_event(aggregate_id: Uuid, version: i64, event_type: &str) -> StoredEvent {
        StoredEvent {
            id: Uuid::new_v4(),
            aggregate_id,
            aggregate_type: "session".to_string(),
            event_type: event_type.to_string(),
            payload: json!({"data": "test"}),
            metadata: json!({"source": "test"}),
            version,
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    #[tokio::test]
    async fn test_append_and_read() {
        let store = SqliteEventStore::in_memory().expect("Failed to create store");
        let agg_id = Uuid::new_v4();

        let event1 = test_event(agg_id, 1, "session.created");
        let event2 = test_event(agg_id, 2, "session.phase_changed");
        let event3 = test_event(agg_id, 3, "session.checkpoint");

        store.append(event1).await.expect("append failed");
        store.append(event2).await.expect("append failed");
        store.append(event3).await.expect("append failed");

        let events = store.read_events(agg_id, None).await.expect("read failed");
        assert_eq!(events.len(), 3);
        assert_eq!(events[0].event_type, "session.created");
        assert_eq!(events[1].event_type, "session.phase_changed");
        assert_eq!(events[2].event_type, "session.checkpoint");
    }

    #[tokio::test]
    async fn test_read_with_version_filter() {
        let store = SqliteEventStore::in_memory().expect("Failed to create store");
        let agg_id = Uuid::new_v4();

        store.append(test_event(agg_id, 1, "v1")).await.unwrap();
        store.append(test_event(agg_id, 2, "v2")).await.unwrap();
        store.append(test_event(agg_id, 3, "v3")).await.unwrap();

        // after_version=1 means version > 1, so versions 2 and 3
        let events = store.read_events(agg_id, Some(1)).await.unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event_type, "v2");
        assert_eq!(events[1].event_type, "v3");

        // after_version=2 means version > 2, so only version 3
        let events = store.read_events(agg_id, Some(2)).await.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "v3");
    }

    #[tokio::test]
    async fn test_version_conflict() {
        let store = SqliteEventStore::in_memory().expect("Failed to create store");
        let agg_id = Uuid::new_v4();

        store.append(test_event(agg_id, 1, "v1")).await.unwrap();

        // Try to append version 1 again → should fail
        let result = store.append(test_event(agg_id, 1, "v1_dup")).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_snapshot_roundtrip() {
        let store = SqliteEventStore::in_memory().expect("Failed to create store");
        let agg_id = Uuid::new_v4();

        let snapshot = StoredSnapshot {
            aggregate_id: agg_id,
            aggregate_type: "session".to_string(),
            state: json!({"phase": "implementing", "iteration": 5}),
            version: 5,
            updated_at: chrono::Utc::now().to_rfc3339(),
        };

        store.save_snapshot(snapshot).await.unwrap();

        let loaded = store.get_snapshot(agg_id).await.unwrap();
        assert!(loaded.is_some());

        let loaded = loaded.unwrap();
        assert_eq!(loaded.version, 5);
        assert_eq!(loaded.state["phase"], "implementing");
        assert_eq!(loaded.state["iteration"], 5);
    }

    #[tokio::test]
    async fn test_snapshot_upsert() {
        let store = SqliteEventStore::in_memory().expect("Failed to create store");
        let agg_id = Uuid::new_v4();

        // Save initial snapshot
        let snap1 = StoredSnapshot {
            aggregate_id: agg_id,
            aggregate_type: "session".to_string(),
            state: json!({"v": 1}),
            version: 1,
            updated_at: chrono::Utc::now().to_rfc3339(),
        };
        store.save_snapshot(snap1).await.unwrap();

        // Upsert with new version
        let snap2 = StoredSnapshot {
            aggregate_id: agg_id,
            aggregate_type: "session".to_string(),
            state: json!({"v": 2}),
            version: 2,
            updated_at: chrono::Utc::now().to_rfc3339(),
        };
        store.save_snapshot(snap2).await.unwrap();

        let loaded = store.get_snapshot(agg_id).await.unwrap().unwrap();
        assert_eq!(loaded.version, 2);
        assert_eq!(loaded.state["v"], 2);
    }

    #[tokio::test]
    async fn test_list_aggregates() {
        let store = SqliteEventStore::in_memory().expect("Failed to create store");

        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();

        store.append(test_event(id1, 1, "e1")).await.unwrap();
        store.append(test_event(id2, 1, "e2")).await.unwrap();

        let aggregates = store.list_aggregates("session").await.unwrap();
        assert_eq!(aggregates.len(), 2);
        assert!(aggregates.contains(&id1));
        assert!(aggregates.contains(&id2));
    }

    #[tokio::test]
    async fn test_file_based_store() {
        let dir = std::env::temp_dir().join(format!("project-x-test-{}", Uuid::new_v4()));
        let db_path = dir.join("test.db");

        let store = SqliteEventStore::new(&db_path).expect("Failed to create file store");

        let agg_id = Uuid::new_v4();
        store.append(test_event(agg_id, 1, "test")).await.unwrap();

        let events = store.read_events(agg_id, None).await.unwrap();
        assert_eq!(events.len(), 1);

        // Clean up
        drop(store);
        let _ = std::fs::remove_dir_all(&dir);
    }
}