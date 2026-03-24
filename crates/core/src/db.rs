use std::path::{Path, PathBuf};

use anyhow::Result;
use rusqlite::Connection;

const MIGRATION_001: &str = include_str!("../migrations/001_initial.sql");
const MIGRATION_002: &str = include_str!("../migrations/002_app_settings.sql");
const MIGRATION_003: &str = include_str!("../migrations/003_sentinel.sql");

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
        conn.execute_batch(MIGRATION_001)?;
        conn.execute_batch(MIGRATION_002)?;
        conn.execute_batch(MIGRATION_003)?;
        Ok(())
    }
}
