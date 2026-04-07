export type GuardCertificationStatus = {
  certification: string;
  expires_on: string;
  status: string;
};

export type GuardProfile = {
  id: string;
  name: string;
  date_of_birth: string;
  age: number;
  phone: string;
  email: string;
  active: boolean;
  notes: string;
  preferred_shifts: string;
  roles: string[];
  certifications: GuardCertificationStatus[];
};

export type ShiftAssignmentView = {
  shift_id: string;
  template_name: string;
  cycle_name: string;
  site_name: string;
  pool_name: string;
  role_name: string;
  day_name: string;
  start_time: string;
  end_time: string;
  assigned_guard_name?: string | null;
  assignment_status: string;
  incumbent_guard_name?: string | null;
  rollover_deadline: string;
};

export type ShiftQueueEntry = {
  request_id: string;
  shift_id: string;
  template_name: string;
  requester_name: string;
  requested_at: string;
  status: string;
  reason?: string | null;
};

export type PolicyViolationView = {
  shift_id: string;
  guard_name: string;
  template_name: string;
  violation: string;
  reason: string;
};

export type CertificationExpiryView = {
  guard_name: string;
  certification: string;
  expires_on: string;
  days_remaining: number;
};

export type DecisionTraceSummary = {
  id: string;
  shift_id: string;
  summary: string;
  decision_type: string;
  decided_at: string;
};

export type DecisionTraceDetail = DecisionTraceSummary & {
  payload: unknown;
};

export type DashboardData = {
  demo_mode: boolean;
  admin_mode: boolean;
  current_cycle_name: string;
  next_cycle_name: string;
  active_guards: number;
  open_shift_count: number;
  pending_request_count: number;
  expiring_cert_count: number;
  draft_status: string;
  recent_decisions: DecisionTraceSummary[];
};

// --- Sentinel ---

export type Camera = {
  id: string;
  site_id: string;
  name: string;
  location: string;
  stream_url: string;
  active: boolean;
};

export type PoolZone = {
  id: string;
  pool_id: string;
  camera_id?: string | null;
  name: string;
  zone_type: string;
  immobility_threshold_secs: number;
  active: boolean;
};

export type SentinelEvent = {
  id: string;
  camera_id?: string | null;
  zone_id: string;
  event_type: string;
  confidence: number;
  duration_secs: number;
  description: string;
  detected_at: string;
  dismissed: boolean;
};

export type SentinelAlert = {
  id: string;
  event_id: string;
  severity: string;
  status: string;
  explanation: string;
  created_at: string;
  resolved_at?: string | null;
  escalation_count: number;
  zone_name?: string | null;
  pool_name?: string | null;
  site_name?: string | null;
  event_type?: string | null;
  confidence?: number | null;
  duration_secs?: number | null;
};

export type SentinelDashboard = {
  active_alerts: SentinelAlert[];
  recent_events: SentinelEvent[];
  cameras: Camera[];
  zones: PoolZone[];
  event_history: SentinelEvent[];
};

// --- Integrations ---

export type IntegrationStatus = {
  key: string;
  label: string;
  connected: boolean;
  value: string;
  description: string;
};

export type AssistantResponse = {
  tool: string;
  title: string;
  explanation: string;
  data: unknown;
};

export interface SetupStatus {
  app_mode: string;
  sling_connected: boolean;
  last_import: string | null;
  guard_count: number;
  site_count: number;
  template_count: number;
}

export interface ImportRunResult {
  guards_imported: number;
  guards_updated: number;
  sites_imported: number;
  positions_imported: number;
  shifts_imported: number;
  errors: string[];
}

export interface SlingExportResult {
  shifts_exported: number;
  errors: string[];
}
