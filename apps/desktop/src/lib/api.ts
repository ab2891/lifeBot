import type {
  AssistantResponse,
  Camera,
  CertificationExpiryView,
  DashboardData,
  DecisionTraceDetail,
  DecisionTraceSummary,
  GuardProfile,
  ImportRunResult,
  IntegrationStatus,
  PolicyViolationView,
  PoolZone,
  SentinelAlert,
  SentinelDashboard,
  SentinelEvent,
  SetupStatus,
  ShiftAssignmentView,
  ShiftQueueEntry,
  SlingExportResult
} from "@lifebot/shared-types";

// Detect Tauri environment
const isTauri =
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

// Lazy-load Tauri invoke to avoid import errors in non-Tauri environments
async function tauriInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<T>(cmd, args);
}

// HTTP fetch for the Axum web-server mode
async function fetchJson<T>(url: string, opts?: RequestInit): Promise<T> {
  const res = await fetch(url, {
    headers: { "Content-Type": "application/json" },
    ...opts,
  });
  if (!res.ok) {
    const body = await res.text();
    throw new Error(`API ${res.status}: ${body}`);
  }
  const text = await res.text();
  if (!text) return null as unknown as T;
  return JSON.parse(text) as T;
}

export const api = {
  bootstrap: () =>
    isTauri
      ? tauriInvoke<void>("bootstrap_app")
      : fetchJson<void>("/api/bootstrap", { method: "POST" }),

  dashboard: () =>
    isTauri
      ? tauriInvoke<DashboardData>("get_dashboard")
      : fetchJson<DashboardData>("/api/dashboard"),

  guards: () =>
    isTauri
      ? tauriInvoke<GuardProfile[]>("list_guard_profiles")
      : fetchJson<GuardProfile[]>("/api/guards"),

  shifts: () =>
    isTauri
      ? tauriInvoke<ShiftAssignmentView[]>("list_schedule_view")
      : fetchJson<ShiftAssignmentView[]>("/api/schedule"),

  queue: () =>
    isTauri
      ? tauriInvoke<ShiftQueueEntry[]>("list_request_queue")
      : fetchJson<ShiftQueueEntry[]>("/api/queue"),

  violations: () =>
    isTauri
      ? tauriInvoke<PolicyViolationView[]>("list_policy_violations")
      : fetchJson<PolicyViolationView[]>("/api/violations"),

  expirations: () =>
    isTauri
      ? tauriInvoke<CertificationExpiryView[]>("list_cert_expirations")
      : fetchJson<CertificationExpiryView[]>("/api/expirations"),

  generateDraft: () =>
    isTauri
      ? tauriInvoke<DecisionTraceSummary[]>("generate_next_cycle_draft")
      : fetchJson<DecisionTraceSummary[]>("/api/draft/generate", { method: "POST" }),

  approveDraft: () =>
    isTauri
      ? tauriInvoke<void>("approve_draft_schedule")
      : fetchJson<void>("/api/draft/approve", { method: "POST" }),

  traces: () =>
    isTauri
      ? tauriInvoke<DecisionTraceSummary[]>("list_decision_traces")
      : fetchJson<DecisionTraceSummary[]>("/api/traces"),

  traceDetail: (traceId: string) =>
    isTauri
      ? tauriInvoke<DecisionTraceDetail | null>("get_decision_trace_detail", { traceId })
      : fetchJson<DecisionTraceDetail | null>(`/api/traces/${encodeURIComponent(traceId)}`),

  examples: () =>
    isTauri
      ? tauriInvoke<string[]>("get_assistant_examples")
      : fetchJson<string[]>("/api/assistant/examples"),

  assistantQuery: (query: string) =>
    isTauri
      ? tauriInvoke<AssistantResponse>("run_assistant_query", { query })
      : fetchJson<AssistantResponse>("/api/assistant/query", {
          method: "POST",
          body: JSON.stringify({ query }),
        }),

  integrations: () =>
    isTauri
      ? tauriInvoke<IntegrationStatus[]>("get_integrations")
      : fetchJson<IntegrationStatus[]>("/api/integrations"),

  saveIntegration: (key: string, value: string) =>
    isTauri
      ? tauriInvoke<void>("save_integration", { key, value })
      : fetchJson<void>("/api/integrations", {
          method: "POST",
          body: JSON.stringify({ key, value }),
        }),

  disconnectIntegration: (key: string) =>
    isTauri
      ? tauriInvoke<void>("disconnect_integration", { key })
      : fetchJson<void>(`/api/integrations/${encodeURIComponent(key)}`, {
          method: "DELETE",
        }),

  // --- Sentinel ---

  sentinelDashboard: () =>
    isTauri
      ? tauriInvoke<SentinelDashboard>("sentinel_dashboard")
      : fetchJson<SentinelDashboard>("/api/sentinel/dashboard"),

  sentinelActiveAlerts: () =>
    isTauri
      ? tauriInvoke<SentinelAlert[]>("sentinel_active_alerts")
      : fetchJson<SentinelAlert[]>("/api/sentinel/alerts"),

  sentinelEvents: () =>
    isTauri
      ? tauriInvoke<SentinelEvent[]>("sentinel_event_history")
      : fetchJson<SentinelEvent[]>("/api/sentinel/events"),

  sentinelZones: () =>
    isTauri
      ? tauriInvoke<PoolZone[]>("sentinel_zones")
      : fetchJson<PoolZone[]>("/api/sentinel/zones"),

  sentinelSimulate: (zoneId: string, eventType: string, confidence: number, durationSecs: number) =>
    isTauri
      ? tauriInvoke<SentinelAlert>("sentinel_simulate", { zoneId, eventType, confidence, durationSecs })
      : fetchJson<SentinelAlert>("/api/sentinel/simulate", {
          method: "POST",
          body: JSON.stringify({ zone_id: zoneId, event_type: eventType, confidence, duration_secs: durationSecs }),
        }),

  sentinelAcknowledge: (alertId: string, guardId: string, action: string, notes: string) =>
    isTauri
      ? tauriInvoke<void>("sentinel_acknowledge", { alertId, guardId, action, notes })
      : fetchJson<void>("/api/sentinel/acknowledge", {
          method: "POST",
          body: JSON.stringify({ alert_id: alertId, guard_id: guardId, action, notes }),
        }),

  // --- Camera management ---

  sentinelCameras: () =>
    isTauri
      ? tauriInvoke<Camera[]>("sentinel_cameras")
      : fetchJson<Camera[]>("/api/sentinel/cameras"),

  sentinelAddCamera: (siteId: string, name: string, location: string, streamUrl: string) =>
    isTauri
      ? tauriInvoke<Camera>("sentinel_add_camera", { siteId, name, location, streamUrl })
      : fetchJson<Camera>("/api/sentinel/cameras", {
          method: "POST",
          body: JSON.stringify({ site_id: siteId, name, location, stream_url: streamUrl }),
        }),

  sentinelUpdateCamera: (cameraId: string, name: string, location: string, streamUrl: string, active: boolean) =>
    isTauri
      ? tauriInvoke<void>("sentinel_update_camera", { cameraId, name, location, streamUrl, active })
      : fetchJson<void>(`/api/sentinel/cameras/${encodeURIComponent(cameraId)}`, {
          method: "PUT",
          body: JSON.stringify({ name, location, stream_url: streamUrl, active }),
        }),

  sentinelDeleteCamera: (cameraId: string) =>
    isTauri
      ? tauriInvoke<void>("sentinel_delete_camera", { cameraId })
      : fetchJson<void>(`/api/sentinel/cameras/${encodeURIComponent(cameraId)}`, {
          method: "DELETE",
        }),

  sentinelAssignCamera: (zoneId: string, cameraId: string | null) =>
    isTauri
      ? tauriInvoke<void>("sentinel_assign_camera_to_zone", { zoneId, cameraId })
      : fetchJson<void>("/api/sentinel/zones/assign", {
          method: "POST",
          body: JSON.stringify({ zone_id: zoneId, camera_id: cameraId }),
        }),

  sentinelCvHealth: () =>
    isTauri
      ? tauriInvoke<boolean>("sentinel_cv_health")
      : fetchJson<{ healthy: boolean }>("/api/sentinel/cv/health").then(r => r.healthy),

  sentinelRunDetection: () =>
    isTauri
      ? tauriInvoke<SentinelAlert[]>("sentinel_run_detection")
      : fetchJson<SentinelAlert[]>("/api/sentinel/cv/detect", { method: "POST" }),

  // --- Setup / Sling onboarding ---

  getSetupStatus: () =>
    isTauri
      ? tauriInvoke<SetupStatus>("get_setup_status")
      : fetchJson<SetupStatus>("/api/setup/status"),

  initAppMode: (mode: string) =>
    isTauri
      ? tauriInvoke<void>("init_app_mode", { mode })
      : fetchJson<void>("/api/setup/mode", { method: "POST", body: JSON.stringify({ mode }) }),

  slingConnect: (email: string, password: string) =>
    isTauri
      ? tauriInvoke<string>("sling_connect", { email, password })
      : fetchJson<string>("/api/sling/connect", { method: "POST", body: JSON.stringify({ email, password }) }),

  slingImport: (dateFrom: string, dateTo: string, cycleName: string) =>
    isTauri
      ? tauriInvoke<ImportRunResult>("sling_import", { dateFrom, dateTo, cycleName })
      : fetchJson<ImportRunResult>("/api/sling/import", { method: "POST", body: JSON.stringify({ dateFrom, dateTo, cycleName }) }),

  slingExport: (cycleId: string) =>
    isTauri
      ? tauriInvoke<SlingExportResult>("sling_export", { cycleId })
      : fetchJson<SlingExportResult>("/api/sling/export", { method: "POST", body: JSON.stringify({ cycleId }) }),

  reseedDemo: () =>
    isTauri
      ? tauriInvoke<void>("reseed_demo")
      : fetchJson<void>("/api/setup/demo", { method: "POST" }),
};
