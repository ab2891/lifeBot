use std::path::{Path, PathBuf};

use anyhow::Result;
use rusqlite::Connection;

const MIGRATION_001: &str = include_str!("../migrations/001_initial.sql");
const MIGRATION_002: &str = include_str!("../migrations/002_app_settings.sql");
const MIGRATION_003: &str = include_str!("../migrations/003_sentinel.sql");
const MIGRATION_004: &str = include_str!("../migrations/004_sling_integration.sql");

#[derive(Debug, Clone)]
pub struct LifebotDb {
    path: PathBuf,
}

impl LifebotDb {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn connect(&self) -> Result<Connection> {
        let conn = Connection::open(&self.path)?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        Ok(conn)
    }

    pub fn migrate(&self) -> Result<()> {
        let conn = self.connect()?;
        migrate_conn(&conn)?;
        Ok(())
    }
}

/// Run all migrations on an existing connection.
///
/// This is useful for in-memory databases (e.g. in tests) where every
/// `Connection::open(":memory:")` call creates a *separate* database, so
/// `migrate()` and `connect()` would otherwise operate on different empty
/// databases.
pub fn migrate_conn(conn: &Connection) -> Result<()> {
    conn.execute_batch(MIGRATION_001)?;
    conn.execute_batch(MIGRATION_002)?;
    conn.execute_batch(MIGRATION_003)?;

    // Migration 004: add Sling ID columns (idempotent via ensure_column)
    ensure_column(conn, "guards", "sling_id", "INTEGER")?;
    ensure_column(conn, "sites", "sling_id", "INTEGER")?;
    ensure_column(conn, "roles", "sling_id", "INTEGER")?;
    ensure_column(conn, "shifts", "sling_shift_id", "TEXT")?;
    conn.execute_batch(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_guards_sling_id ON guards(sling_id) WHERE sling_id IS NOT NULL;
         CREATE UNIQUE INDEX IF NOT EXISTS idx_sites_sling_id ON sites(sling_id) WHERE sling_id IS NOT NULL;
         CREATE UNIQUE INDEX IF NOT EXISTS idx_roles_sling_id ON roles(sling_id) WHERE sling_id IS NOT NULL;
         CREATE UNIQUE INDEX IF NOT EXISTS idx_shifts_sling_id ON shifts(sling_shift_id) WHERE sling_shift_id IS NOT NULL;",
    )?;
    conn.execute_batch(MIGRATION_004)?;

    Ok(())
}

/// Add a column to a table only if it does not already exist.
/// SQLite does not support `ALTER TABLE … ADD COLUMN IF NOT EXISTS`, so we
/// inspect PRAGMA table_info first.
fn ensure_column(conn: &Connection, table: &str, column: &str, col_type: &str) -> Result<()> {
    let has_column: bool = conn
        .prepare(&format!("PRAGMA table_info({table})"))?
        .query_map([], |row| row.get::<_, String>(1))?
        .any(|name| name.as_deref() == Ok(column));

    if !has_column {
        conn.execute_batch(&format!(
            "ALTER TABLE {table} ADD COLUMN {column} {col_type};"
        ))?;
    }
    Ok(())
}
