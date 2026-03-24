-- Sentinel: human-in-the-loop pool surveillance assist
-- SAFETY DISCLAIMER: Sentinel is an assistive safety layer only.
-- It is NOT a replacement for active lifeguard surveillance.

CREATE TABLE IF NOT EXISTS cameras (
    id          TEXT PRIMARY KEY,
    site_id     TEXT NOT NULL REFERENCES sites(id),
    name        TEXT NOT NULL,
    location    TEXT NOT NULL DEFAULT '',
    stream_url  TEXT NOT NULL DEFAULT '',
    active      INTEGER NOT NULL DEFAULT 1,
    created_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS pool_zones (
    id          TEXT PRIMARY KEY,
    pool_id     TEXT NOT NULL REFERENCES pools(id),
    camera_id   TEXT REFERENCES cameras(id),
    name        TEXT NOT NULL,
    zone_type   TEXT NOT NULL DEFAULT 'general',  -- general, deep_end, shallow, lap_lane, diving
    immobility_threshold_secs INTEGER NOT NULL DEFAULT 15,
    active      INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE IF NOT EXISTS sentinel_events (
    id              TEXT PRIMARY KEY,
    camera_id       TEXT REFERENCES cameras(id),
    zone_id         TEXT NOT NULL REFERENCES pool_zones(id),
    event_type      TEXT NOT NULL,  -- immobility, unresponsive, motion_timeout
    confidence      REAL NOT NULL DEFAULT 0.0,
    duration_secs   REAL NOT NULL DEFAULT 0.0,
    description     TEXT NOT NULL DEFAULT '',
    raw_data_json   TEXT NOT NULL DEFAULT '{}',
    detected_at     TEXT NOT NULL DEFAULT (datetime('now')),
    dismissed       INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS sentinel_alerts (
    id              TEXT PRIMARY KEY,
    event_id        TEXT NOT NULL REFERENCES sentinel_events(id),
    severity        TEXT NOT NULL DEFAULT 'low',  -- low, medium, high
    status          TEXT NOT NULL DEFAULT 'active',  -- active, acknowledged, resolved, false_positive, escalated
    explanation     TEXT NOT NULL DEFAULT '',
    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    resolved_at     TEXT,
    escalation_count INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS sentinel_alert_recipients (
    id          TEXT PRIMARY KEY,
    alert_id    TEXT NOT NULL REFERENCES sentinel_alerts(id),
    guard_id    TEXT NOT NULL REFERENCES guards(id),
    role        TEXT NOT NULL DEFAULT 'supervisor',  -- supervisor, backup
    notified_at TEXT NOT NULL DEFAULT (datetime('now')),
    channel     TEXT NOT NULL DEFAULT 'in_app'  -- in_app, sms, groupme
);

CREATE TABLE IF NOT EXISTS sentinel_acknowledgments (
    id          TEXT PRIMARY KEY,
    alert_id    TEXT NOT NULL REFERENCES sentinel_alerts(id),
    guard_id    TEXT NOT NULL REFERENCES guards(id),
    action      TEXT NOT NULL,  -- acknowledged, dismissed, false_positive, escalated, resolved
    notes       TEXT NOT NULL DEFAULT '',
    acted_at    TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS incident_outcomes (
    id          TEXT PRIMARY KEY,
    alert_id    TEXT NOT NULL REFERENCES sentinel_alerts(id),
    outcome     TEXT NOT NULL,  -- false_alarm, resolved_safe, intervention_needed, real_emergency
    summary     TEXT NOT NULL DEFAULT '',
    recorded_by TEXT,
    recorded_at TEXT NOT NULL DEFAULT (datetime('now'))
);
