CREATE TABLE IF NOT EXISTS guards (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  date_of_birth TEXT NOT NULL,
  phone TEXT NOT NULL,
  email TEXT NOT NULL,
  notes TEXT NOT NULL,
  preferred_shifts TEXT NOT NULL,
  active INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE IF NOT EXISTS certifications (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL UNIQUE
);

CREATE TABLE IF NOT EXISTS guard_certifications (
  guard_id TEXT NOT NULL,
  certification_id TEXT NOT NULL,
  expires_on TEXT NOT NULL,
  PRIMARY KEY (guard_id, certification_id),
  FOREIGN KEY (guard_id) REFERENCES guards(id) ON DELETE CASCADE,
  FOREIGN KEY (certification_id) REFERENCES certifications(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS roles (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL UNIQUE
);

CREATE TABLE IF NOT EXISTS guard_roles (
  guard_id TEXT NOT NULL,
  role_id TEXT NOT NULL,
  PRIMARY KEY (guard_id, role_id),
  FOREIGN KEY (guard_id) REFERENCES guards(id) ON DELETE CASCADE,
  FOREIGN KEY (role_id) REFERENCES roles(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS sites (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  region TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS pools (
  id TEXT PRIMARY KEY,
  site_id TEXT NOT NULL,
  name TEXT NOT NULL,
  FOREIGN KEY (site_id) REFERENCES sites(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS scheduling_cycles (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  starts_on TEXT NOT NULL,
  ends_on TEXT NOT NULL,
  rollover_deadline TEXT NOT NULL,
  status TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS shift_templates (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  site_id TEXT NOT NULL,
  pool_id TEXT NOT NULL,
  role_id TEXT NOT NULL,
  day_of_week TEXT NOT NULL,
  start_time TEXT NOT NULL,
  end_time TEXT NOT NULL,
  required_certifications TEXT NOT NULL,
  active INTEGER NOT NULL DEFAULT 1,
  FOREIGN KEY (site_id) REFERENCES sites(id),
  FOREIGN KEY (pool_id) REFERENCES pools(id),
  FOREIGN KEY (role_id) REFERENCES roles(id)
);

CREATE TABLE IF NOT EXISTS shifts (
  id TEXT PRIMARY KEY,
  template_id TEXT NOT NULL,
  cycle_id TEXT NOT NULL,
  shift_date TEXT NOT NULL,
  status TEXT NOT NULL,
  FOREIGN KEY (template_id) REFERENCES shift_templates(id),
  FOREIGN KEY (cycle_id) REFERENCES scheduling_cycles(id)
);

CREATE TABLE IF NOT EXISTS shift_assignments (
  id TEXT PRIMARY KEY,
  shift_id TEXT NOT NULL,
  guard_id TEXT NOT NULL,
  status TEXT NOT NULL,
  assigned_at TEXT NOT NULL,
  FOREIGN KEY (shift_id) REFERENCES shifts(id) ON DELETE CASCADE,
  FOREIGN KEY (guard_id) REFERENCES guards(id)
);

CREATE TABLE IF NOT EXISTS shift_requests (
  id TEXT PRIMARY KEY,
  shift_template_id TEXT NOT NULL,
  cycle_id TEXT NOT NULL,
  guard_id TEXT NOT NULL,
  requested_at TEXT NOT NULL,
  status TEXT NOT NULL,
  reason TEXT,
  FOREIGN KEY (shift_template_id) REFERENCES shift_templates(id),
  FOREIGN KEY (cycle_id) REFERENCES scheduling_cycles(id),
  FOREIGN KEY (guard_id) REFERENCES guards(id)
);

CREATE TABLE IF NOT EXISTS rollover_requests (
  id TEXT PRIMARY KEY,
  shift_template_id TEXT NOT NULL,
  cycle_id TEXT NOT NULL,
  guard_id TEXT NOT NULL,
  requested_at TEXT NOT NULL,
  status TEXT NOT NULL,
  FOREIGN KEY (shift_template_id) REFERENCES shift_templates(id),
  FOREIGN KEY (cycle_id) REFERENCES scheduling_cycles(id),
  FOREIGN KEY (guard_id) REFERENCES guards(id)
);

CREATE TABLE IF NOT EXISTS policy_rules (
  id TEXT PRIMARY KEY,
  rule_type TEXT NOT NULL,
  description TEXT NOT NULL,
  config_json TEXT NOT NULL,
  active INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE IF NOT EXISTS policy_scopes (
  id TEXT PRIMARY KEY,
  policy_rule_id TEXT NOT NULL,
  site_id TEXT,
  region TEXT,
  role_id TEXT,
  min_age INTEGER,
  max_age INTEGER,
  FOREIGN KEY (policy_rule_id) REFERENCES policy_rules(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS decision_traces (
  id TEXT PRIMARY KEY,
  cycle_id TEXT NOT NULL,
  shift_id TEXT NOT NULL,
  decision_type TEXT NOT NULL,
  summary TEXT NOT NULL,
  payload_json TEXT NOT NULL,
  decided_at TEXT NOT NULL,
  FOREIGN KEY (cycle_id) REFERENCES scheduling_cycles(id),
  FOREIGN KEY (shift_id) REFERENCES shifts(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS message_log (
  id TEXT PRIMARY KEY,
  provider_name TEXT NOT NULL,
  recipient TEXT NOT NULL,
  body TEXT NOT NULL,
  created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS import_jobs (
  id TEXT PRIMARY KEY,
  provider_name TEXT NOT NULL,
  status TEXT NOT NULL,
  details_json TEXT NOT NULL,
  created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS provider_sync_state (
  id TEXT PRIMARY KEY,
  provider_name TEXT NOT NULL,
  status TEXT NOT NULL,
  last_synced_at TEXT NOT NULL,
  details_json TEXT NOT NULL
);
