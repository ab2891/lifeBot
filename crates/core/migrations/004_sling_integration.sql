-- Track Sling IDs so we can map imported data back to Sling for export.
-- NOTE: ALTER TABLE statements for new columns are handled in Rust (db.rs)
-- via ensure_column() to make them idempotent. This file contains only the
-- CREATE TABLE and INSERT statements that are already idempotent.

-- Import run log
CREATE TABLE IF NOT EXISTS import_runs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    source TEXT NOT NULL DEFAULT 'sling',
    guards_imported INTEGER NOT NULL DEFAULT 0,
    guards_updated INTEGER NOT NULL DEFAULT 0,
    sites_imported INTEGER NOT NULL DEFAULT 0,
    positions_imported INTEGER NOT NULL DEFAULT 0,
    shifts_imported INTEGER NOT NULL DEFAULT 0,
    errors_json TEXT,
    started_at TEXT NOT NULL DEFAULT (datetime('now')),
    completed_at TEXT
);

-- App mode setting
INSERT OR IGNORE INTO app_settings (key, value, updated_at) VALUES ('app_mode', 'uninitialized', datetime('now'));
INSERT OR IGNORE INTO app_settings (key, value, updated_at) VALUES ('sling_token', '', datetime('now'));
INSERT OR IGNORE INTO app_settings (key, value, updated_at) VALUES ('sling_org_id', '', datetime('now'));
