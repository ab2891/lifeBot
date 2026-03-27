<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "./lib/api";
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
    ShiftQueueEntry
  } from "@lifebot/shared-types";

  type Tab =
    | "dashboard"
    | "guards"
    | "schedule"
    | "queue"
    | "violations"
    | "certs"
    | "draft"
    | "assistant"
    | "sentinel"
    | "integrations";

  const tabs: { id: Tab; label: string }[] = [
    { id: "dashboard", label: "Dashboard" },
    { id: "guards", label: "Guards" },
    { id: "schedule", label: "Schedule" },
    { id: "queue", label: "Queue" },
    { id: "violations", label: "Violations" },
    { id: "certs", label: "Certs" },
    { id: "draft", label: "Draft" },
    { id: "assistant", label: "Assistant" },
    { id: "sentinel", label: "Sentinel" },
    { id: "integrations", label: "Integrations" }
  ];

  let activeTab: Tab = "dashboard";
  let loading = true;
  let busy = false;

  // Setup wizard state
  let setupStatus: SetupStatus | null = null;
  let setupStep: 'choose' | 'sling-login' | 'sling-import' | 'done' = 'choose';
  let slingEmail = '';
  let slingPassword = '';
  let importDateFrom = '';
  let importDateTo = '';
  let importCycleName = '';
  let importResult: ImportRunResult | null = null;
  let setupError = '';
  let dashboard: DashboardData | null = null;
  let guards: GuardProfile[] = [];
  let shifts: ShiftAssignmentView[] = [];
  let queue: ShiftQueueEntry[] = [];
  let violations: PolicyViolationView[] = [];
  let expirations: CertificationExpiryView[] = [];
  let traces: DecisionTraceSummary[] = [];
  let selectedTrace: DecisionTraceDetail | null = null;
  let assistantExamples: string[] = [];
  let assistantQuery = "";
  let assistantResponse: AssistantResponse | null = null;
  let integrations: IntegrationStatus[] = [];
  let integrationEditing: Record<string, string> = {};
  let sentinelAlerts: SentinelAlert[] = [];
  let sentinelEvents: SentinelEvent[] = [];
  let sentinelZones: PoolZone[] = [];
  let simZoneId = "";
  let simEventType = "immobility";
  let simConfidence = 0.85;
  let simDuration = 22;
  let cameras: Camera[] = [];
  let cameraForm = { name: "", location: "", streamUrl: "", active: true };
  let editingCameraId: string | null = null;
  let showCameraForm = false;
  let cvHealthy: boolean | null = null;
  let error = "";

  async function loadAll() {
    loading = true;
    error = "";
    try {
      await api.bootstrap();
      setupStatus = await api.getSetupStatus();
      if (setupStatus.app_mode === "uninitialized") {
        // Show setup wizard instead of main app
        loading = false;
        return;
      }
      [
        dashboard,
        guards,
        shifts,
        queue,
        violations,
        expirations,
        traces,
        assistantExamples
      ] = await Promise.all([
        api.dashboard(),
        api.guards(),
        api.shifts(),
        api.queue(),
        api.violations(),
        api.expirations(),
        api.traces(),
        api.examples()
      ]);
      integrations = await api.integrations();
      sentinelAlerts = await api.sentinelActiveAlerts();
      sentinelEvents = await api.sentinelEvents();
      sentinelZones = await api.sentinelZones();
      cameras = await api.sentinelCameras();
      api.sentinelCvHealth().then(h => cvHealthy = h).catch(() => cvHealthy = false);
      if (sentinelZones.length > 0 && !simZoneId) simZoneId = sentinelZones[0].id;
      integrationEditing = {};
      for (const integ of integrations) {
        integrationEditing[integ.key] = integ.value;
      }
      selectedTrace = traces[0] ? await api.traceDetail(traces[0].id) : null;
    } catch (err) {
      error = err instanceof Error ? err.message : "Unable to load data.";
    } finally {
      loading = false;
    }
  }

  async function setupChooseDemo() {
    setupError = '';
    busy = true;
    try {
      await api.initAppMode('demo');
      setupStatus = await api.getSetupStatus();
      await loadAll();
    } catch (err) {
      setupError = err instanceof Error ? err.message : "Failed to initialize demo mode.";
    } finally {
      busy = false;
    }
  }

  async function setupSlingConnect() {
    setupError = '';
    if (!slingEmail.trim() || !slingPassword.trim()) {
      setupError = "Email and password are required.";
      return;
    }
    busy = true;
    try {
      await api.slingConnect(slingEmail, slingPassword);
      setupStep = 'sling-import';
    } catch (err) {
      setupError = err instanceof Error ? err.message : "Failed to connect to Sling.";
    } finally {
      busy = false;
    }
  }

  async function setupSlingImport() {
    setupError = '';
    if (!importCycleName.trim() || !importDateFrom || !importDateTo) {
      setupError = "Cycle name and date range are required.";
      return;
    }
    busy = true;
    try {
      importResult = await api.slingImport(importDateFrom, importDateTo, importCycleName);
      setupStatus = await api.getSetupStatus();
      await loadAll();
    } catch (err) {
      setupError = err instanceof Error ? err.message : "Import failed.";
    } finally {
      busy = false;
    }
  }

  async function generateDraft() {
    busy = true;
    try {
      traces = await api.generateDraft();
      shifts = await api.shifts();
      queue = await api.queue();
      violations = await api.violations();
      dashboard = await api.dashboard();
      if (traces[0]) selectedTrace = await api.traceDetail(traces[0].id);
      activeTab = "draft";
    } finally {
      busy = false;
    }
  }

  async function approveDraft() {
    busy = true;
    try {
      await api.approveDraft();
      dashboard = await api.dashboard();
      shifts = await api.shifts();
    } finally {
      busy = false;
    }
  }

  async function showTrace(traceId: string) {
    selectedTrace = await api.traceDetail(traceId);
  }

  async function runAssistant() {
    if (!assistantQuery.trim()) return;
    busy = true;
    try {
      assistantResponse = await api.assistantQuery(assistantQuery);
      dashboard = await api.dashboard();
      traces = await api.traces();
      shifts = await api.shifts();
      queue = await api.queue();
      violations = await api.violations();
      expirations = await api.expirations();
    } finally {
      busy = false;
    }
  }

  async function refreshSentinel() {
    sentinelAlerts = await api.sentinelActiveAlerts();
    sentinelEvents = await api.sentinelEvents();
  }

  async function simulateSentinelEvent() {
    if (!simZoneId) return;
    busy = true;
    try {
      await api.sentinelSimulate(simZoneId, simEventType, simConfidence, simDuration);
      await refreshSentinel();
    } finally {
      busy = false;
    }
  }

  async function sentinelAction(alertId: string, action: string) {
    busy = true;
    try {
      // Use first guard as actor for demo
      const actorId = guards.length > 0 ? guards[0].id : "demo-user";
      const notes = action === "false_positive" ? "Marked as false positive" : action === "escalated" ? "Escalating to additional staff" : "";
      await api.sentinelAcknowledge(alertId, actorId, action, notes);
      await refreshSentinel();
    } finally {
      busy = false;
    }
  }

  function severityColor(severity: string): string {
    if (severity === "high") return "#dc2626";
    if (severity === "medium") return "#d97706";
    return "#6b7a8a";
  }

  async function saveIntegration(key: string) {
    busy = true;
    try {
      await api.saveIntegration(key, integrationEditing[key] ?? "");
      integrations = await api.integrations();
    } finally {
      busy = false;
    }
  }

  async function disconnectIntegration(key: string) {
    busy = true;
    try {
      await api.disconnectIntegration(key);
      integrations = await api.integrations();
      integrationEditing[key] = "";
    } finally {
      busy = false;
    }
  }

  function resetCameraForm() {
    cameraForm = { name: "", location: "", streamUrl: "", active: true };
    editingCameraId = null;
    showCameraForm = false;
  }

  function startEditCamera(cam: Camera) {
    editingCameraId = cam.id;
    cameraForm = { name: cam.name, location: cam.location, streamUrl: cam.stream_url, active: cam.active };
    showCameraForm = true;
  }

  async function saveCamera() {
    if (!cameraForm.name.trim()) return;
    busy = true;
    try {
      if (editingCameraId) {
        await api.sentinelUpdateCamera(editingCameraId, cameraForm.name, cameraForm.location, cameraForm.streamUrl, cameraForm.active);
      } else {
        // Use first site id from dashboard or fallback
        const siteId = "site-default";
        await api.sentinelAddCamera(siteId, cameraForm.name, cameraForm.location, cameraForm.streamUrl);
      }
      cameras = await api.sentinelCameras();
      resetCameraForm();
    } finally {
      busy = false;
    }
  }

  async function deleteCamera(cameraId: string) {
    busy = true;
    try {
      await api.sentinelDeleteCamera(cameraId);
      cameras = await api.sentinelCameras();
    } finally {
      busy = false;
    }
  }

  async function toggleCamera(cam: Camera) {
    busy = true;
    try {
      await api.sentinelUpdateCamera(cam.id, cam.name, cam.location, cam.stream_url, !cam.active);
      cameras = await api.sentinelCameras();
    } finally {
      busy = false;
    }
  }

  async function assignCameraToZone(zoneId: string, cameraId: string | null) {
    busy = true;
    try {
      await api.sentinelAssignCamera(zoneId, cameraId);
      sentinelZones = await api.sentinelZones();
    } finally {
      busy = false;
    }
  }

  onMount(loadAll);
</script>

<svelte:head>
  <title>Lifebot MVP</title>
</svelte:head>

{#if loading}
  <div class="splash">Loading Lifebot…</div>
{:else if error}
  <div class="splash">
    <div class="error-box">
      <h2>Lifebot could not start</h2>
      <p>{error}</p>
      <button on:click={loadAll}>Try again</button>
    </div>
  </div>
{:else if setupStatus?.app_mode === 'uninitialized'}
  <div class="splash">
    <div class="setup-card">
      <div class="setup-header">
        <div class="eyebrow" style="color: #8fa8be;">Aquatics assistant</div>
        <h1 style="font-size: 1.6rem; font-weight: 700; color: #1a2530; margin-top: 4px;">Welcome to Lifebot</h1>
        <p style="margin-top: 8px; color: #6b7a8a; font-size: 0.9rem;">Choose how you'd like to get started.</p>
      </div>

      {#if setupError}
        <div class="setup-error">{setupError}</div>
      {/if}

      {#if setupStep === 'choose'}
        <div class="setup-choices">
          <button class="setup-choice-btn setup-choice-primary" disabled={busy} on:click={() => { setupStep = 'sling-login'; setupError = ''; }}>
            <strong>Connect to Sling</strong>
            <span>Import your real guard schedules and staff data</span>
          </button>
          <button class="setup-choice-btn setup-choice-secondary" disabled={busy} on:click={setupChooseDemo}>
            <strong>Try Demo Mode</strong>
            <span>Explore with safe seeded data — no Sling account needed</span>
          </button>
        </div>
      {/if}

      {#if setupStep === 'sling-login'}
        <div class="setup-form">
          <p style="margin-bottom: 16px; color: #6b7a8a; font-size: 0.85rem;">Enter your Sling credentials. Lifebot will use these to fetch your schedule data.</p>
          <label class="setup-label">Email
            <input type="email" placeholder="your@email.com" bind:value={slingEmail} disabled={busy} />
          </label>
          <label class="setup-label">Password
            <input type="password" placeholder="Sling password" bind:value={slingPassword} disabled={busy} />
          </label>
          <div class="setup-form-actions">
            <button class="btn-primary" disabled={busy || !slingEmail.trim() || !slingPassword.trim()} on:click={setupSlingConnect}>
              {busy ? 'Connecting…' : 'Connect'}
            </button>
            <button class="btn-secondary" disabled={busy} on:click={() => { setupStep = 'choose'; setupError = ''; }}>Back</button>
          </div>
        </div>
      {/if}

      {#if setupStep === 'sling-import'}
        <div class="setup-form">
          <p style="margin-bottom: 16px; color: #6b7a8a; font-size: 0.85rem;">Sling connected. Now import your schedule data for a date range.</p>
          <label class="setup-label">Cycle name
            <input type="text" placeholder="e.g. Summer 2026 Week 1" bind:value={importCycleName} disabled={busy} />
          </label>
          <div class="setup-date-row">
            <label class="setup-label">From
              <input type="date" bind:value={importDateFrom} disabled={busy} />
            </label>
            <label class="setup-label">To
              <input type="date" bind:value={importDateTo} disabled={busy} />
            </label>
          </div>
          {#if importResult}
            <div class="setup-import-result">
              <strong>Import complete</strong>
              <ul>
                <li>{importResult.guards_imported} guards imported, {importResult.guards_updated} updated</li>
                <li>{importResult.sites_imported} sites, {importResult.positions_imported} positions, {importResult.shifts_imported} shifts</li>
                {#if importResult.errors.length > 0}
                  <li style="color: #dc2626;">{importResult.errors.length} error(s): {importResult.errors[0]}</li>
                {/if}
              </ul>
            </div>
          {/if}
          <div class="setup-form-actions">
            <button class="btn-primary" disabled={busy || !importCycleName.trim() || !importDateFrom || !importDateTo} on:click={setupSlingImport}>
              {busy ? 'Importing…' : 'Import from Sling'}
            </button>
            <button class="btn-secondary" disabled={busy} on:click={() => { setupStep = 'sling-login'; setupError = ''; }}>Back</button>
          </div>
        </div>
      {/if}
    </div>
  </div>
{:else}
  <div class="shell">
    <div class="sidebar">
      <div class="brand">
        <div class="eyebrow">Aquatics assistant</div>
        <h1>Lifebot</h1>
      </div>
      <nav>
        {#each tabs as tab}
          <button
            class="nav-btn"
            class:active={activeTab === tab.id}
            on:click={() => (activeTab = tab.id)}
          >{tab.label}</button>
        {/each}
      </nav>
      {#if setupStatus?.app_mode === 'demo'}
        <div class="sidebar-note">
          <strong>Demo mode</strong>
          <p>Safe seeded data. No real Sling account required.</p>
        </div>
      {/if}
    </div>

    <div class="main">
      <div class="topbar">
        <div>
          <div class="eyebrow">Current status</div>
          <h2>{dashboard?.next_cycle_name} — {dashboard?.draft_status?.replaceAll("_", " ")}</h2>
        </div>
        <div class="topbar-actions">
          <button class="btn-primary" disabled={busy} on:click={generateDraft}>Generate draft</button>
          <button class="btn-secondary" disabled={busy} on:click={approveDraft}>Approve draft</button>
        </div>
      </div>

      <div class="content">
        {#if activeTab === "dashboard"}
          <div class="stat-row">
            <div class="stat-card"><div class="stat-label">Active guards</div><div class="stat-val">{dashboard?.active_guards}</div></div>
            <div class="stat-card"><div class="stat-label">Open shifts</div><div class="stat-val">{dashboard?.open_shift_count}</div></div>
            <div class="stat-card"><div class="stat-label">Queued requests</div><div class="stat-val">{dashboard?.pending_request_count}</div></div>
            <div class="stat-card"><div class="stat-label">Expiring certs</div><div class="stat-val">{dashboard?.expiring_cert_count}</div></div>
          </div>
          <div class="two-col">
            <div class="card">
              <h3>What the assistant can do</h3>
              <ul>{#each assistantExamples as ex}<li>{ex}</li>{/each}</ul>
            </div>
            <div class="card">
              <h3>Recent decisions</h3>
              {#if traces.length === 0}
                <p>No decisions yet. Generate a draft first.</p>
              {:else}
                {#each traces.slice(0, 5) as trace}
                  <button class="list-row" on:click={() => showTrace(trace.id)}>
                    <span>{trace.summary}</span>
                    <small>{trace.decided_at}</small>
                  </button>
                {/each}
              {/if}
            </div>
          </div>
        {/if}

        {#if activeTab === "guards"}
          <div class="card">
            <h3>Guard profiles</h3>
            {#each guards as g}
              <div class="row">
                <div><strong>{g.name}</strong><br><small>{g.age} yrs · {g.roles.join(", ")}</small><br><small>{g.phone} · {g.email}</small></div>
                <div><strong>Preferences:</strong> {g.preferred_shifts}<br><strong>Notes:</strong> {g.notes}</div>
                <div><strong>Certs:</strong>{#each g.certifications as c}<br><small>{c.certification}: {c.expires_on} ({c.status})</small>{/each}</div>
              </div>
            {/each}
          </div>
        {/if}

        {#if activeTab === "schedule"}
          <div class="card">
            <h3>Shifts</h3>
            {#each shifts as s}
              <div class="row">
                <div><strong>{s.template_name}</strong><br><small>{s.day_name} · {s.start_time}–{s.end_time}</small></div>
                <div>{s.site_name} · {s.pool_name}<br><small>{s.role_name}</small></div>
                <div>Cycle: {s.cycle_name}<br><small>Incumbent: {s.incumbent_guard_name ?? "None"}</small></div>
                <div>Assigned: {s.assigned_guard_name ?? "Open"}<br><small>{s.assignment_status}</small></div>
              </div>
            {/each}
          </div>
        {/if}

        {#if activeTab === "queue"}
          <div class="card">
            <h3>Request queue</h3>
            {#each queue as r}
              <div class="row">
                <div><strong>{r.template_name}</strong><br>{r.requester_name}</div>
                <div>Requested: {r.requested_at}<br><small>Status: {r.status}</small></div>
                <div>{r.reason ?? "No issue recorded."}</div>
              </div>
            {/each}
          </div>
        {/if}

        {#if activeTab === "violations"}
          <div class="card">
            <h3>Policy violations</h3>
            {#each violations as v}
              <div class="row">
                <div><strong>{v.guard_name}</strong><br>{v.template_name}</div>
                <div>{v.violation}</div>
                <div>{v.reason}</div>
              </div>
            {/each}
          </div>
        {/if}

        {#if activeTab === "certs"}
          <div class="card">
            <h3>Certification renewals</h3>
            {#each expirations as c}
              <div class="row">
                <div><strong>{c.guard_name}</strong><br>{c.certification}</div>
                <div>Expires: {c.expires_on}<br><small>{c.days_remaining} days remaining</small></div>
              </div>
            {/each}
          </div>
        {/if}

        {#if activeTab === "draft"}
          <div class="two-col">
            <div class="card">
              <h3>Decision summaries</h3>
              {#each traces as trace}
                <button class="list-row" on:click={() => showTrace(trace.id)}>
                  <span>{trace.summary}</span>
                  <small>{trace.decided_at}</small>
                </button>
              {/each}
            </div>
            <div class="card">
              <h3>Why this decision</h3>
              {#if selectedTrace}
                <p>{selectedTrace.summary}</p>
                <pre>{JSON.stringify(selectedTrace.payload, null, 2)}</pre>
              {:else}
                <p>Select a decision to see the reasoning trace.</p>
              {/if}
            </div>
          </div>
        {/if}

        {#if activeTab === "assistant"}
          <div class="two-col">
            <div class="card">
              <h3>Ask Lifebot</h3>
              {#each assistantExamples as ex}
                <button class="list-row" on:click={() => (assistantQuery = ex)}>{ex}</button>
              {/each}
              <textarea bind:value={assistantQuery} placeholder="Ask about shifts, queues, certifications…"></textarea>
              <button class="btn-primary" disabled={busy} on:click={runAssistant}>Run</button>
            </div>
            <div class="card">
              <h3>{assistantResponse?.title ?? "Response"}</h3>
              <p>{assistantResponse?.explanation ?? "Responses appear here."}</p>
              {#if assistantResponse}
                <pre>{JSON.stringify(assistantResponse.data, null, 2)}</pre>
              {/if}
            </div>
          </div>
        {/if}
        {#if activeTab === "sentinel"}
          <div class="sentinel-disclaimer">
            Sentinel is an assistive safety layer only. It is NOT a replacement for active lifeguard surveillance. All alerts require human acknowledgment.
          </div>

          <div class="two-col">
            <div class="card">
              <h3>Active Alerts</h3>
              {#if sentinelAlerts.length === 0}
                <p>No active alerts. The pool is clear.</p>
              {:else}
                {#each sentinelAlerts as alert}
                  <div class="sentinel-alert" style="border-left: 4px solid {severityColor(alert.severity)}">
                    <div class="sentinel-alert-header">
                      <span class="severity-badge" style="background: {severityColor(alert.severity)}">{alert.severity.toUpperCase()}</span>
                      <span class="alert-status">{alert.status}</span>
                      {#if alert.escalation_count > 0}<span class="escalation-badge">Escalated x{alert.escalation_count}</span>{/if}
                    </div>
                    <p class="alert-explanation">{alert.explanation}</p>
                    <div class="alert-meta">
                      <small>{alert.pool_name} · {alert.zone_name}</small>
                      <small>{alert.created_at}</small>
                    </div>
                    <div class="alert-actions">
                      <button class="btn-primary" disabled={busy} on:click={() => sentinelAction(alert.id, "acknowledged")}>Acknowledge</button>
                      <button class="btn-secondary" disabled={busy} on:click={() => sentinelAction(alert.id, "resolved")}>Resolved</button>
                      <button class="btn-secondary" disabled={busy} on:click={() => sentinelAction(alert.id, "false_positive")}>False Positive</button>
                      <button class="btn-secondary" disabled={busy} on:click={() => sentinelAction(alert.id, "escalated")}>Escalate</button>
                    </div>
                  </div>
                {/each}
              {/if}
            </div>

            <div class="card">
              <h3>Simulate Event (Demo)</h3>
              <p style="margin-bottom: 12px; color: #6b7a8a; font-size: 0.82rem;">Generate a simulated detection event for testing and presentation purposes.</p>
              <label class="sim-label">Zone
                <select bind:value={simZoneId}>
                  {#each sentinelZones as zone}
                    <option value={zone.id}>{zone.name} ({zone.zone_type})</option>
                  {/each}
                </select>
              </label>
              <label class="sim-label">Event type
                <select bind:value={simEventType}>
                  <option value="immobility">Prolonged immobility</option>
                  <option value="unresponsive">Possible unresponsive</option>
                  <option value="motion_timeout">Motion timeout</option>
                </select>
              </label>
              <label class="sim-label">Confidence ({(simConfidence * 100).toFixed(0)}%)
                <input type="range" min="0.1" max="1.0" step="0.05" bind:value={simConfidence} />
              </label>
              <label class="sim-label">Duration ({simDuration}s)
                <input type="range" min="5" max="60" step="1" bind:value={simDuration} />
              </label>
              <button class="btn-primary" style="margin-top: 12px; width: 100%;" disabled={busy} on:click={simulateSentinelEvent}>
                Simulate detection event
              </button>
            </div>
          </div>

          <div class="card" style="margin-top: 16px;">
            <h3>Event History</h3>
            {#each sentinelEvents as evt}
              <div class="row">
                <div><strong>{evt.event_type}</strong><br><small>{evt.detected_at}</small></div>
                <div>{evt.description}</div>
                <div>Confidence: {((evt.confidence ?? 0) * 100).toFixed(0)}%<br><small>{(evt.duration_secs ?? 0).toFixed(0)}s duration</small></div>
              </div>
            {/each}
            {#if sentinelEvents.length === 0}
              <p>No events recorded yet. Use the simulator to generate test events.</p>
            {/if}
          </div>
        {/if}

        {#if activeTab === "integrations"}
          <div class="card">
            <h3>Integrations</h3>
            <p style="margin-bottom: 16px; color: #6b7a8a;">Connect external services to enable live data sync and AI-powered scheduling.</p>
            {#each integrations as integ}
              <div class="integration-card">
                <div class="integration-header">
                  <div>
                    <strong>{integ.label}</strong>
                    <span class="integration-status" class:connected={integ.connected}>
                      {integ.connected ? "Connected" : "Not connected"}
                    </span>
                  </div>
                </div>
                <p class="integration-desc">{integ.description}</p>
                <div class="integration-form">
                  <input
                    type="text"
                    placeholder={integ.key === "sling_api_key" ? "Enter Sling API key" : integ.key === "openclaw_endpoint" ? "e.g. http://localhost:8080" : "e.g. groupme or sms"}
                    bind:value={integrationEditing[integ.key]}
                  />
                  <button class="btn-primary" disabled={busy} on:click={() => saveIntegration(integ.key)}>
                    {integ.connected ? "Update" : "Connect"}
                  </button>
                  {#if integ.connected}
                    <button class="btn-secondary" disabled={busy} on:click={() => disconnectIntegration(integ.key)}>
                      Disconnect
                    </button>
                  {/if}
                </div>
              </div>
            {/each}
          </div>

          <!-- Camera Management -->
          <div class="card" style="margin-top: 16px;">
            <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 14px;">
              <h3 style="margin-bottom: 0;">Camera Management</h3>
              <div style="display: flex; align-items: center; gap: 12px;">
                <span class="cv-status" class:cv-online={cvHealthy === true} class:cv-offline={cvHealthy === false}>
                  CV Worker: {cvHealthy === null ? "Checking..." : cvHealthy ? "Online" : "Offline"}
                </span>
                <button class="btn-primary" on:click={() => { resetCameraForm(); showCameraForm = true; }}>
                  + Add Camera
                </button>
              </div>
            </div>
            <p style="margin-bottom: 16px; color: #6b7a8a; font-size: 0.82rem;">
              Register cameras, assign them to pool zones, and manage stream URLs. When a CV provider is connected, these cameras will feed the Sentinel detection pipeline.
            </p>

            {#if showCameraForm}
              <div class="camera-form-card">
                <h4>{editingCameraId ? "Edit Camera" : "Add Camera"}</h4>
                <div class="camera-form-grid">
                  <label class="cam-label">Name
                    <input type="text" placeholder="e.g. Main Pool Overhead" bind:value={cameraForm.name} />
                  </label>
                  <label class="cam-label">Location
                    <input type="text" placeholder="e.g. North wall, 12ft" bind:value={cameraForm.location} />
                  </label>
                  <label class="cam-label" style="grid-column: 1 / -1;">Stream URL
                    <input type="text" placeholder="rtsp://192.168.1.100:554/stream or mock://demo" bind:value={cameraForm.streamUrl} />
                  </label>
                  {#if editingCameraId}
                    <label class="cam-label cam-checkbox">
                      <input type="checkbox" bind:checked={cameraForm.active} />
                      Active
                    </label>
                  {/if}
                </div>
                <div class="camera-form-actions">
                  <button class="btn-primary" disabled={busy || !cameraForm.name.trim()} on:click={saveCamera}>
                    {editingCameraId ? "Update" : "Add"}
                  </button>
                  <button class="btn-secondary" on:click={resetCameraForm}>Cancel</button>
                </div>
              </div>
            {/if}

            {#if cameras.length === 0}
              <p style="color: #6b7a8a; text-align: center; padding: 24px 0;">No cameras registered. Add one to start building your surveillance infrastructure.</p>
            {:else}
              {#each cameras as cam}
                <div class="camera-card" class:camera-inactive={!cam.active}>
                  <div class="camera-card-header">
                    <div>
                      <strong>{cam.name}</strong>
                      <span class="camera-active-badge" class:active={cam.active}>
                        {cam.active ? "Active" : "Inactive"}
                      </span>
                    </div>
                    <div class="camera-card-actions">
                      <button class="btn-secondary btn-sm" on:click={() => toggleCamera(cam)}>
                        {cam.active ? "Disable" : "Enable"}
                      </button>
                      <button class="btn-secondary btn-sm" on:click={() => startEditCamera(cam)}>Edit</button>
                      <button class="btn-secondary btn-sm btn-danger" disabled={busy} on:click={() => deleteCamera(cam.id)}>Delete</button>
                    </div>
                  </div>
                  <div class="camera-card-details">
                    <div><small>Location:</small> {cam.location || "—"}</div>
                    <div><small>Stream:</small> <code>{cam.stream_url || "Not set"}</code></div>
                  </div>
                </div>
              {/each}
            {/if}
          </div>

          <!-- Zone → Camera Assignment -->
          <div class="card" style="margin-top: 16px;">
            <h3>Zone Camera Assignment</h3>
            <p style="margin-bottom: 14px; color: #6b7a8a; font-size: 0.82rem;">
              Assign registered cameras to pool zones. Each zone can have one camera feeding detections.
            </p>
            {#if sentinelZones.length === 0}
              <p style="color: #6b7a8a; text-align: center; padding: 16px 0;">No zones configured. Zones are created during Sentinel setup.</p>
            {:else}
              {#each sentinelZones as zone}
                <div class="zone-assign-row">
                  <div class="zone-assign-info">
                    <strong>{zone.name}</strong>
                    <small>{zone.zone_type} · Immobility threshold: {zone.immobility_threshold_secs}s</small>
                  </div>
                  <select
                    class="zone-assign-select"
                    value={zone.camera_id ?? ""}
                    on:change={(e) => assignCameraToZone(zone.id, e.currentTarget.value || null)}
                  >
                    <option value="">No camera assigned</option>
                    {#each cameras.filter(c => c.active) as cam}
                      <option value={cam.id}>{cam.name} — {cam.location}</option>
                    {/each}
                  </select>
                </div>
              {/each}
            {/if}
          </div>
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  :global(*, *::before, *::after) { box-sizing: border-box; margin: 0; padding: 0; }

  :global(body) {
    font-family: "Segoe UI", system-ui, sans-serif;
    font-size: 14px;
    background: #f0f2f5;
    color: #1a2530;
  }

  .splash {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100vh;
    font-size: 1.2rem;
    color: #555;
  }

  .error-box {
    background: #fff;
    border: 1px solid #ddd;
    border-radius: 8px;
    padding: 32px;
    max-width: 480px;
    text-align: center;
  }

  .error-box h2 { margin-bottom: 12px; }
  .error-box p { margin-bottom: 20px; color: #555; }

  .shell {
    display: flex;
    height: 100vh;
    overflow: hidden;
  }

  .sidebar {
    width: 200px;
    min-width: 200px;
    background: #1a2d3d;
    color: #e8edf2;
    display: flex;
    flex-direction: column;
    padding: 20px 12px;
    gap: 16px;
  }

  .brand h1 {
    font-size: 1.4rem;
    font-weight: 700;
    color: #fff;
    margin-top: 4px;
  }

  .eyebrow {
    font-size: 0.68rem;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    color: #8fa8be;
  }

  nav { display: flex; flex-direction: column; gap: 4px; }

  .nav-btn {
    background: none;
    border: none;
    color: #b8ccd8;
    text-align: left;
    padding: 8px 10px;
    border-radius: 6px;
    cursor: pointer;
    font-size: 0.88rem;
    font-family: inherit;
  }

  .nav-btn:hover { background: rgba(255,255,255,0.08); color: #fff; }
  .nav-btn.active { background: #2c6fa8; color: #fff; }

  .sidebar-note {
    margin-top: auto;
    font-size: 0.78rem;
    color: #7a9ab5;
    border-top: 1px solid rgba(255,255,255,0.1);
    padding-top: 12px;
  }

  .sidebar-note strong { color: #aac4d8; display: block; margin-bottom: 4px; }

  .main {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .topbar {
    background: #fff;
    border-bottom: 1px solid #dde2e8;
    padding: 16px 24px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
  }

  .topbar h2 { font-size: 1.1rem; font-weight: 600; margin-top: 2px; }

  .topbar-actions { display: flex; gap: 8px; }

  button { font-family: inherit; cursor: pointer; }

  .btn-primary {
    background: #2c6fa8;
    color: #fff;
    border: none;
    padding: 8px 16px;
    border-radius: 6px;
    font-size: 0.88rem;
    font-weight: 500;
  }

  .btn-primary:hover { background: #255d8e; }
  .btn-primary:disabled { opacity: 0.5; cursor: not-allowed; }

  .btn-secondary {
    background: #fff;
    color: #1a2530;
    border: 1px solid #ccd2d8;
    padding: 8px 16px;
    border-radius: 6px;
    font-size: 0.88rem;
  }

  .btn-secondary:hover { background: #f5f7f9; }
  .btn-secondary:disabled { opacity: 0.5; cursor: not-allowed; }

  .content {
    flex: 1;
    overflow-y: auto;
    padding: 20px 24px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .card {
    background: #fff;
    border: 1px solid #dde2e8;
    border-radius: 8px;
    padding: 20px;
  }

  .card h3 { font-size: 0.95rem; font-weight: 600; margin-bottom: 14px; color: #1a2530; }

  .stat-row { display: flex; gap: 12px; }

  .stat-card {
    flex: 1;
    background: #fff;
    border: 1px solid #dde2e8;
    border-radius: 8px;
    padding: 16px;
  }

  .stat-label { font-size: 0.78rem; color: #6b7a8a; margin-bottom: 6px; }
  .stat-val { font-size: 2rem; font-weight: 700; color: #1a2530; }

  .two-col { display: flex; gap: 16px; }
  .two-col .card { flex: 1; }

  .row {
    display: flex;
    gap: 12px;
    padding: 12px 0;
    border-bottom: 1px solid #eef0f3;
    font-size: 0.85rem;
    line-height: 1.5;
  }

  .row > div { flex: 1; }
  .row:last-child { border-bottom: none; }

  .list-row {
    display: block;
    width: 100%;
    background: #f7f9fb;
    border: 1px solid #e4e8ed;
    border-radius: 6px;
    padding: 10px 12px;
    text-align: left;
    margin-bottom: 6px;
    font-size: 0.85rem;
    color: #1a2530;
  }

  .list-row:hover { background: #edf2f7; }
  .list-row span { display: block; }
  .list-row small { color: #6b7a8a; }

  textarea {
    display: block;
    width: 100%;
    margin: 12px 0;
    padding: 10px 12px;
    border: 1px solid #ccd2d8;
    border-radius: 6px;
    font: inherit;
    font-size: 0.88rem;
    min-height: 100px;
    resize: vertical;
    background: #fff;
    color: #1a2530;
  }

  pre {
    background: #f4f6f8;
    border: 1px solid #dde2e8;
    border-radius: 6px;
    padding: 12px;
    font-size: 0.8rem;
    overflow: auto;
    max-height: 360px;
    margin-top: 12px;
  }

  ul { padding-left: 18px; }
  li { margin-bottom: 6px; font-size: 0.88rem; line-height: 1.5; }
  small { font-size: 0.8rem; color: #6b7a8a; }
  strong { font-weight: 600; }
  p { font-size: 0.88rem; line-height: 1.5; color: #3a4a5a; }

  .sentinel-disclaimer {
    background: #fef3c7;
    border: 1px solid #f59e0b;
    border-radius: 8px;
    padding: 12px 16px;
    font-size: 0.82rem;
    color: #92400e;
    margin-bottom: 16px;
    font-weight: 500;
  }

  .sentinel-alert {
    border: 1px solid #e4e8ed;
    border-radius: 8px;
    padding: 14px;
    margin-bottom: 10px;
    background: #fafbfc;
  }

  .sentinel-alert-header {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 8px;
  }

  .severity-badge {
    color: #fff;
    font-size: 0.7rem;
    font-weight: 700;
    padding: 2px 8px;
    border-radius: 4px;
    letter-spacing: 0.05em;
  }

  .alert-status {
    font-size: 0.78rem;
    color: #6b7a8a;
    text-transform: capitalize;
  }

  .escalation-badge {
    font-size: 0.72rem;
    background: #fee2e2;
    color: #991b1b;
    padding: 2px 6px;
    border-radius: 4px;
  }

  .alert-explanation {
    font-size: 0.85rem;
    line-height: 1.5;
    color: #1a2530;
    margin-bottom: 8px;
  }

  .alert-meta {
    display: flex;
    justify-content: space-between;
    margin-bottom: 10px;
  }

  .alert-actions {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }

  .alert-actions button {
    font-size: 0.78rem;
    padding: 5px 10px;
  }

  .sim-label {
    display: block;
    font-size: 0.82rem;
    color: #3a4a5a;
    margin-bottom: 10px;
    font-weight: 500;
  }

  .sim-label select,
  .sim-label input[type="range"] {
    display: block;
    width: 100%;
    margin-top: 4px;
  }

  .sim-label select {
    padding: 6px 10px;
    border: 1px solid #ccd2d8;
    border-radius: 6px;
    font: inherit;
    font-size: 0.85rem;
    background: #fff;
  }

  .integration-card {
    border: 1px solid #e4e8ed;
    border-radius: 8px;
    padding: 16px;
    margin-bottom: 12px;
    background: #fafbfc;
  }

  .integration-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 8px;
  }

  .integration-status {
    display: inline-block;
    margin-left: 10px;
    padding: 2px 8px;
    border-radius: 10px;
    font-size: 0.75rem;
    font-weight: 500;
    background: #fee2e2;
    color: #991b1b;
  }

  .integration-status.connected {
    background: #d1fae5;
    color: #065f46;
  }

  .integration-desc {
    font-size: 0.82rem;
    color: #6b7a8a;
    margin-bottom: 12px;
  }

  .integration-form {
    display: flex;
    gap: 8px;
    align-items: center;
  }

  .integration-form input {
    flex: 1;
    padding: 8px 12px;
    border: 1px solid #ccd2d8;
    border-radius: 6px;
    font: inherit;
    font-size: 0.85rem;
    background: #fff;
    color: #1a2530;
  }

  /* Camera management */

  .cv-status {
    font-size: 0.75rem;
    font-weight: 500;
    padding: 3px 10px;
    border-radius: 10px;
    background: #e5e7eb;
    color: #6b7280;
  }

  .cv-status.cv-online {
    background: #d1fae5;
    color: #065f46;
  }

  .cv-status.cv-offline {
    background: #fee2e2;
    color: #991b1b;
  }

  .camera-form-card {
    background: #f7f9fb;
    border: 1px solid #dde2e8;
    border-radius: 8px;
    padding: 16px;
    margin-bottom: 16px;
  }

  .camera-form-card h4 {
    font-size: 0.88rem;
    font-weight: 600;
    margin-bottom: 12px;
  }

  .camera-form-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
  }

  .cam-label {
    display: block;
    font-size: 0.82rem;
    font-weight: 500;
    color: #3a4a5a;
  }

  .cam-label input[type="text"] {
    display: block;
    width: 100%;
    margin-top: 4px;
    padding: 7px 10px;
    border: 1px solid #ccd2d8;
    border-radius: 6px;
    font: inherit;
    font-size: 0.85rem;
    background: #fff;
  }

  .cam-checkbox {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 0.85rem;
    margin-top: 8px;
  }

  .cam-checkbox input[type="checkbox"] {
    width: 16px;
    height: 16px;
  }

  .camera-form-actions {
    display: flex;
    gap: 8px;
    margin-top: 12px;
  }

  .camera-card {
    border: 1px solid #e4e8ed;
    border-radius: 8px;
    padding: 14px;
    margin-bottom: 10px;
    background: #fafbfc;
  }

  .camera-card.camera-inactive {
    opacity: 0.6;
  }

  .camera-card-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 8px;
  }

  .camera-active-badge {
    display: inline-block;
    margin-left: 8px;
    padding: 2px 8px;
    border-radius: 10px;
    font-size: 0.72rem;
    font-weight: 500;
    background: #fee2e2;
    color: #991b1b;
  }

  .camera-active-badge.active {
    background: #d1fae5;
    color: #065f46;
  }

  .camera-card-actions {
    display: flex;
    gap: 6px;
  }

  .btn-sm {
    font-size: 0.75rem;
    padding: 4px 10px;
  }

  .btn-danger {
    color: #dc2626;
    border-color: #fca5a5;
  }

  .btn-danger:hover {
    background: #fef2f2;
  }

  .camera-card-details {
    display: flex;
    gap: 24px;
    font-size: 0.82rem;
    color: #3a4a5a;
  }

  .camera-card-details code {
    font-size: 0.78rem;
    background: #f0f2f5;
    padding: 1px 5px;
    border-radius: 3px;
  }

  .zone-assign-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 0;
    border-bottom: 1px solid #eef0f3;
    gap: 16px;
  }

  .zone-assign-row:last-child {
    border-bottom: none;
  }

  .zone-assign-info {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .zone-assign-info strong {
    font-size: 0.88rem;
  }

  .zone-assign-info small {
    font-size: 0.78rem;
    color: #6b7a8a;
  }

  .zone-assign-select {
    padding: 6px 10px;
    border: 1px solid #ccd2d8;
    border-radius: 6px;
    font: inherit;
    font-size: 0.82rem;
    background: #fff;
    min-width: 220px;
  }

  /* Setup wizard */

  .setup-card {
    background: #fff;
    border: 1px solid #dde2e8;
    border-radius: 12px;
    padding: 40px;
    max-width: 480px;
    width: 100%;
    box-shadow: 0 4px 24px rgba(0,0,0,0.08);
  }

  .setup-header {
    margin-bottom: 28px;
    text-align: center;
  }

  .setup-error {
    background: #fee2e2;
    color: #991b1b;
    border: 1px solid #fca5a5;
    border-radius: 6px;
    padding: 10px 14px;
    font-size: 0.85rem;
    margin-bottom: 18px;
  }

  .setup-choices {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .setup-choice-btn {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 4px;
    padding: 16px 20px;
    border-radius: 8px;
    border: 2px solid transparent;
    cursor: pointer;
    font: inherit;
    text-align: left;
    transition: background 0.15s, border-color 0.15s;
  }

  .setup-choice-btn strong {
    font-size: 0.95rem;
    font-weight: 600;
  }

  .setup-choice-btn span {
    font-size: 0.82rem;
    opacity: 0.8;
  }

  .setup-choice-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .setup-choice-primary {
    background: #2c6fa8;
    color: #fff;
    border-color: #2c6fa8;
  }

  .setup-choice-primary:hover:not(:disabled) {
    background: #255d8e;
    border-color: #255d8e;
  }

  .setup-choice-secondary {
    background: #f0f2f5;
    color: #1a2530;
    border-color: #dde2e8;
  }

  .setup-choice-secondary:hover:not(:disabled) {
    background: #e5e9ef;
    border-color: #c5ccd4;
  }

  .setup-form {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .setup-label {
    display: flex;
    flex-direction: column;
    gap: 5px;
    font-size: 0.85rem;
    font-weight: 500;
    color: #3a4a5a;
  }

  .setup-label input {
    padding: 8px 12px;
    border: 1px solid #ccd2d8;
    border-radius: 6px;
    font: inherit;
    font-size: 0.88rem;
    background: #fff;
    color: #1a2530;
  }

  .setup-label input:disabled {
    background: #f5f7f9;
  }

  .setup-date-row {
    display: flex;
    gap: 12px;
  }

  .setup-date-row .setup-label {
    flex: 1;
  }

  .setup-form-actions {
    display: flex;
    gap: 8px;
    margin-top: 4px;
  }

  .setup-import-result {
    background: #d1fae5;
    border: 1px solid #6ee7b7;
    border-radius: 6px;
    padding: 12px 16px;
    font-size: 0.85rem;
    color: #065f46;
  }

  .setup-import-result strong {
    display: block;
    margin-bottom: 6px;
  }

  .setup-import-result ul {
    padding-left: 16px;
  }

  .setup-import-result li {
    margin-bottom: 2px;
  }
</style>
