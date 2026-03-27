use std::{env, path::PathBuf};

use anyhow::{Context, Result};
use chrono::{Datelike, NaiveDate, Utc};
use rusqlite::{params, OptionalExtension};
use serde_json::json;

use lifebot_sling::{mapping, SlingShiftCreate, SlingShiftRef, SlingShiftUser};

use crate::{
    db::LifebotDb,
    import,
    models::{
        AssistantResponse, CertificationExpiryView, DashboardData, DecisionTraceDetail,
        DecisionTraceSummary, GuardCertificationStatus, GuardProfile, ImportRunResult,
        IntegrationStatus, PolicyViolationView, SetupStatus, ShiftAssignmentView, ShiftQueueEntry,
    },
    scheduling::generate_next_cycle_draft,
    seed::seed_demo,
};

#[derive(Clone)]
pub struct LifebotService {
    db: LifebotDb,
    demo_mode: bool,
    admin_mode: bool,
}

impl LifebotService {
    pub fn from_env(base_dir: impl Into<PathBuf>) -> Self {
        let base_dir = base_dir.into();
        let db_path = env::var("LIFEBOT_DB_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| base_dir.join("lifebot-demo.db"));
        let demo_mode = env::var("LIFEBOT_DEMO_MODE").unwrap_or_else(|_| "true".into()) == "true";
        let admin_mode = env::var("LIFEBOT_ADMIN_MODE").unwrap_or_else(|_| "false".into()) == "true";
        Self {
            db: LifebotDb::new(db_path),
            demo_mode,
            admin_mode,
        }
    }

    pub fn init(&self) -> Result<()> {
        self.db.migrate()?;
        let conn = self.db.connect()?;
        // Check app_mode from app_settings; fall back to "demo" for backward compatibility.
        let app_mode: String = conn
            .query_row(
                "SELECT value FROM app_settings WHERE key = 'app_mode'",
                [],
                |row| row.get(0),
            )
            .unwrap_or_else(|_| "demo".to_string());
        if app_mode == "demo" {
            seed_demo(&conn)?;
            crate::sentinel::seed_sentinel_demo(&conn)?;
        }
        Ok(())
    }

    pub fn reseed_demo(&self) -> Result<()> {
        let conn = self.db.connect()?;
        conn.execute_batch(
            "
            DELETE FROM decision_traces;
            DELETE FROM shift_assignments;
            DELETE FROM shifts;
            DELETE FROM rollover_requests;
            DELETE FROM shift_requests;
            DELETE FROM shift_templates;
            DELETE FROM scheduling_cycles;
            DELETE FROM guard_certifications;
            DELETE FROM guard_roles;
            DELETE FROM guards;
            DELETE FROM certifications;
            DELETE FROM roles;
            DELETE FROM pools;
            DELETE FROM sites;
            DELETE FROM provider_sync_state;
            DELETE FROM policy_rules;
            ",
        )?;
        seed_demo(&conn)?;
        Ok(())
    }

    pub fn dashboard(&self) -> Result<DashboardData> {
        let conn = self.db.connect()?;
        let active_guards = count(&conn, "SELECT COUNT(*) FROM guards WHERE active = 1")?;
        let open_shift_count = count(
            &conn,
            "SELECT COUNT(*) FROM shifts sh LEFT JOIN shift_assignments sa ON sa.shift_id = sh.id WHERE sh.cycle_id = 'cycle-next' AND sa.id IS NULL",
        )?;
        let pending_request_count = count(
            &conn,
            "SELECT COUNT(*) FROM shift_requests WHERE cycle_id = 'cycle-next' AND status = 'queued'",
        )?;
        let expiring_cert_count = count(
            &conn,
            "SELECT COUNT(*) FROM guard_certifications WHERE expires_on <= date('now', '+30 day')",
        )?;
        let draft_status: String = conn.query_row(
            "SELECT status FROM scheduling_cycles WHERE id = 'cycle-next'",
            [],
            |row| row.get(0),
        )?;
        Ok(DashboardData {
            demo_mode: self.demo_mode,
            admin_mode: self.admin_mode,
            current_cycle_name: "Current Cycle".into(),
            next_cycle_name: "Next Cycle".into(),
            active_guards,
            open_shift_count,
            pending_request_count,
            expiring_cert_count,
            draft_status,
            recent_decisions: self.decision_traces()?.into_iter().take(5).collect(),
        })
    }

    pub fn guard_profiles(&self) -> Result<Vec<GuardProfile>> {
        let conn = self.db.connect()?;
        let mut stmt = conn.prepare(
            "SELECT id, name, date_of_birth, phone, email, active, notes, preferred_shifts
             FROM guards ORDER BY name"
        )?;
        let guards = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                ))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        let mut results = Vec::new();
        for (id, name, dob, phone, email, active, notes, preferred_shifts) in guards {
            let roles = load_string_list(
                &conn,
                "SELECT r.name FROM guard_roles gr JOIN roles r ON r.id = gr.role_id WHERE gr.guard_id = ?1 ORDER BY r.name",
                &id,
            )?;
            let cert_rows = load_pairs(
                &conn,
                "SELECT c.name, gc.expires_on FROM guard_certifications gc JOIN certifications c ON c.id = gc.certification_id WHERE gc.guard_id = ?1 ORDER BY gc.expires_on",
                &id,
            )?;
            let date_of_birth = NaiveDate::parse_from_str(&dob, "%Y-%m-%d")?;
            let age = age_on(date_of_birth, Utc::now().date_naive());
            results.push(GuardProfile {
                id,
                name,
                date_of_birth: dob,
                age,
                phone,
                email,
                active: active == 1,
                notes,
                preferred_shifts,
                roles,
                certifications: cert_rows
                    .into_iter()
                    .map(|(certification, expires_on)| GuardCertificationStatus {
                        status: if expires_on <= Utc::now().date_naive().to_string() {
                            "Expired".into()
                        } else {
                            "Valid".into()
                        },
                        certification,
                        expires_on,
                    })
                    .collect(),
            });
        }
        Ok(results)
    }

    pub fn schedule_view(&self) -> Result<Vec<ShiftAssignmentView>> {
        let conn = self.db.connect()?;
        let mut stmt = conn.prepare(
            "SELECT
                sh.id,
                st.name,
                sc.name,
                si.name,
                p.name,
                r.name,
                st.day_of_week,
                st.start_time,
                st.end_time,
                g.name,
                COALESCE(sa.status, 'open'),
                incumbent.name,
                sc.rollover_deadline
             FROM shifts sh
             JOIN shift_templates st ON st.id = sh.template_id
             JOIN scheduling_cycles sc ON sc.id = sh.cycle_id
             JOIN sites si ON si.id = st.site_id
             JOIN pools p ON p.id = st.pool_id
             JOIN roles r ON r.id = st.role_id
             LEFT JOIN shift_assignments sa ON sa.shift_id = sh.id
             LEFT JOIN guards g ON g.id = sa.guard_id
             LEFT JOIN shifts current_sh ON current_sh.template_id = st.id AND current_sh.cycle_id = 'cycle-current'
             LEFT JOIN shift_assignments current_sa ON current_sa.shift_id = current_sh.id
             LEFT JOIN guards incumbent ON incumbent.id = current_sa.guard_id
             ORDER BY sc.starts_on, st.day_of_week, st.start_time"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(ShiftAssignmentView {
                shift_id: row.get(0)?,
                template_name: row.get(1)?,
                cycle_name: row.get(2)?,
                site_name: row.get(3)?,
                pool_name: row.get(4)?,
                role_name: row.get(5)?,
                day_name: row.get(6)?,
                start_time: row.get(7)?,
                end_time: row.get(8)?,
                assigned_guard_name: row.get(9)?,
                assignment_status: row.get(10)?,
                incumbent_guard_name: row.get(11)?,
                rollover_deadline: row.get(12)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn request_queue(&self) -> Result<Vec<ShiftQueueEntry>> {
        let conn = self.db.connect()?;
        let mut stmt = conn.prepare(
            "SELECT sr.id, COALESCE(sh.id, ''), st.name, g.name, sr.requested_at, sr.status, sr.reason
             FROM shift_requests sr
             JOIN shift_templates st ON st.id = sr.shift_template_id
             JOIN guards g ON g.id = sr.guard_id
             LEFT JOIN shifts sh ON sh.template_id = st.id AND sh.cycle_id = sr.cycle_id
             WHERE sr.cycle_id = 'cycle-next'
             ORDER BY st.name, sr.requested_at"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(ShiftQueueEntry {
                request_id: row.get(0)?,
                shift_id: row.get(1)?,
                template_name: row.get(2)?,
                requester_name: row.get(3)?,
                requested_at: row.get(4)?,
                status: row.get(5)?,
                reason: row.get(6)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn certification_expirations(&self) -> Result<Vec<CertificationExpiryView>> {
        let conn = self.db.connect()?;
        let today = Utc::now().date_naive();
        let mut stmt = conn.prepare(
            "SELECT g.name, c.name, gc.expires_on
             FROM guard_certifications gc
             JOIN guards g ON g.id = gc.guard_id
             JOIN certifications c ON c.id = gc.certification_id
             WHERE gc.expires_on <= date('now', '+45 day')
             ORDER BY gc.expires_on"
        )?;
        let rows = stmt.query_map([], |row| {
            let expires_on: String = row.get(2)?;
            let expires_date = NaiveDate::parse_from_str(&expires_on, "%Y-%m-%d").unwrap();
            Ok(CertificationExpiryView {
                guard_name: row.get(0)?,
                certification: row.get(1)?,
                days_remaining: (expires_date - today).num_days(),
                expires_on,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn policy_violations(&self) -> Result<Vec<PolicyViolationView>> {
        let queue = self.request_queue()?;
        Ok(queue
            .into_iter()
            .filter_map(|entry| {
                entry.reason.map(|reason| PolicyViolationView {
                    shift_id: entry.shift_id,
                    guard_name: entry.requester_name,
                    template_name: entry.template_name,
                    violation: "Eligibility rule".into(),
                    reason,
                })
            })
            .collect())
    }

    pub fn generate_draft(&self) -> Result<Vec<DecisionTraceSummary>> {
        let conn = self.db.connect()?;
        generate_next_cycle_draft(&conn)?;
        self.log_message("in_app", "Draft schedule generated for Next Cycle.")?;
        self.decision_traces()
    }

    pub fn approve_draft_schedule(&self) -> Result<()> {
        let conn = self.db.connect()?;
        conn.execute("UPDATE shifts SET status = 'reviewed' WHERE cycle_id = 'cycle-next'", [])?;
        conn.execute("UPDATE shift_assignments SET status = 'reviewed' WHERE shift_id IN (SELECT id FROM shifts WHERE cycle_id = 'cycle-next')", [])?;
        conn.execute("UPDATE scheduling_cycles SET status = 'awaiting_export' WHERE id = 'cycle-next'", [])?;
        self.log_message("in_app", "Draft schedule marked as reviewed and ready for future export.")?;
        Ok(())
    }

    pub fn decision_traces(&self) -> Result<Vec<DecisionTraceSummary>> {
        let conn = self.db.connect()?;
        let mut stmt = conn.prepare(
            "SELECT id, shift_id, summary, decision_type, decided_at
             FROM decision_traces
             ORDER BY decided_at DESC"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(DecisionTraceSummary {
                id: row.get(0)?,
                shift_id: row.get(1)?,
                summary: row.get(2)?,
                decision_type: row.get(3)?,
                decided_at: row.get(4)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn decision_trace_detail(&self, trace_id: &str) -> Result<Option<DecisionTraceDetail>> {
        let conn = self.db.connect()?;
        conn.query_row(
            "SELECT id, shift_id, summary, decision_type, decided_at, payload_json FROM decision_traces WHERE id = ?1",
            params![trace_id],
            |row| {
                Ok(DecisionTraceDetail {
                    id: row.get(0)?,
                    shift_id: row.get(1)?,
                    summary: row.get(2)?,
                    decision_type: row.get(3)?,
                    decided_at: row.get(4)?,
                    payload: serde_json::from_str::<serde_json::Value>(&row.get::<_, String>(5)?)
                        .unwrap_or(json!({})),
                })
            },
        )
        .optional()
        .map_err(Into::into)
    }

    pub fn assistant_examples(&self) -> Vec<String> {
        vec![
            "Who usually works Tuesday close at the main pool?".into(),
            "Generate next cycle draft and preserve recurring shifts where requested.".into(),
            "Why didn't Marcus get that shift?".into(),
            "Show open shifts that still need coverage.".into(),
            "Who is first in line for Saturday 8am?".into(),
            "Which guards have expiring certifications?".into(),
            "Show active Sentinel alerts.".into(),
            "Simulate an unresponsive swimmer in the deep end.".into(),
            "Who are the current supervisors for this pool?".into(),
            "Show Sentinel event history.".into(),
        ]
    }

    pub fn run_assistant_query(&self, query: &str) -> Result<AssistantResponse> {
        let lower = query.to_lowercase();
        if lower.contains("tuesday close") {
            let schedule = self.schedule_view()?;
            let matches: Vec<_> = schedule
                .into_iter()
                .filter(|s| s.template_name.contains("Tuesday Close"))
                .collect();
            return Ok(AssistantResponse {
                tool: "get_shift_history".into(),
                title: "Tuesday close coverage".into(),
                explanation: "Olivia is the current recurring holder for Tuesday close at the Main Pool.".into(),
                data: serde_json::to_value(matches)?,
            });
        }
        if lower.contains("generate next cycle draft") {
            let traces = self.generate_draft()?;
            return Ok(AssistantResponse {
                tool: "generate_next_cycle_draft".into(),
                title: "Draft generated".into(),
                explanation: "I created a reviewable next-cycle draft and logged the reasons for each assignment decision.".into(),
                data: serde_json::to_value(traces)?,
            });
        }
        if lower.contains("why") && lower.contains("marcus") {
            let traces = self.decision_traces()?;
            let detail = traces
                .iter()
                .find(|trace| trace.summary.contains("Saturday 8am"))
                .and_then(|trace| self.decision_trace_detail(&trace.id).ok().flatten());
            return Ok(AssistantResponse {
                tool: "explain_assignment".into(),
                title: "Why Marcus did not receive the shift".into(),
                explanation: "Marcus lost priority because he did not submit a rollover request before the deadline. Noah requested earlier but was skipped for an expired waterfront certification, so Ben received the shift as the first eligible requester.".into(),
                data: serde_json::to_value(detail)?,
            });
        }
        if lower.contains("open shifts") {
            let schedule = self.schedule_view()?;
            let open: Vec<_> = schedule.into_iter().filter(|s| s.assigned_guard_name.is_none()).collect();
            return Ok(AssistantResponse {
                tool: "list_open_shifts".into(),
                title: "Open shifts".into(),
                explanation: "These draft shifts still need coverage or review.".into(),
                data: serde_json::to_value(open)?,
            });
        }
        if lower.contains("first in line") || lower.contains("queue") {
            let queue = self.request_queue()?;
            let matches: Vec<_> = queue
                .into_iter()
                .filter(|entry| entry.template_name.contains("Saturday 8am"))
                .collect();
            return Ok(AssistantResponse {
                tool: "get_shift_queue".into(),
                title: "Saturday 8am request line".into(),
                explanation: "Noah requested first, but Ben is the first eligible requester because Noah's waterfront certification is expired.".into(),
                data: serde_json::to_value(matches)?,
            });
        }
        if lower.contains("expiring certification") || lower.contains("expiring certifications") {
            let expirations = self.certification_expirations()?;
            return Ok(AssistantResponse {
                tool: "list_cert_expirations".into(),
                title: "Expiring certifications".into(),
                explanation: "These staff records need renewal attention soon.".into(),
                data: serde_json::to_value(expirations)?,
            });
        }

        if lower.contains("sentinel alert") || lower.contains("active alert") {
            let alerts = self.sentinel_active_alerts()?;
            return Ok(AssistantResponse {
                tool: "list_active_sentinel_alerts".into(),
                title: "Active Sentinel alerts".into(),
                explanation: if alerts.is_empty() {
                    "No active Sentinel alerts. The pool is clear.".into()
                } else {
                    format!("{} active alert(s) requiring attention.", alerts.len())
                },
                data: serde_json::to_value(alerts)?,
            });
        }
        if lower.contains("sentinel") && lower.contains("history") {
            let events = self.sentinel_event_history(20)?;
            return Ok(AssistantResponse {
                tool: "list_sentinel_event_history".into(),
                title: "Sentinel event history".into(),
                explanation: format!("Showing the last {} Sentinel detection events.", events.len()),
                data: serde_json::to_value(events)?,
            });
        }
        if lower.contains("simulate") && (lower.contains("sentinel") || lower.contains("swimmer") || lower.contains("immobility")) {
            let zones = self.sentinel_zones()?;
            if let Some(zone) = zones.first() {
                let alert = self.sentinel_simulate_event(&zone.id, "immobility", 0.85, 22.0)?;
                return Ok(AssistantResponse {
                    tool: "simulate_sentinel_event".into(),
                    title: "Simulated Sentinel event".into(),
                    explanation: alert.explanation.clone(),
                    data: serde_json::to_value(alert)?,
                });
            }
        }
        if lower.contains("supervisor") && lower.contains("pool") {
            // Find first pool and its supervisors
            let conn = self.db.connect()?;
            let pool_id: Option<String> = conn.query_row("SELECT id FROM pools LIMIT 1", [], |r| r.get(0)).optional()?;
            if let Some(pid) = pool_id {
                let supervisors = self.sentinel_supervisors_for_pool(&pid)?;
                let result: Vec<_> = supervisors.into_iter().map(|(id, name)| json!({"guard_id": id, "name": name})).collect();
                return Ok(AssistantResponse {
                    tool: "get_current_supervisors_for_pool".into(),
                    title: "Current deck supervisors".into(),
                    explanation: if result.is_empty() { "No supervisors currently on shift.".into() } else { format!("{} supervisor(s) on duty.", result.len()) },
                    data: serde_json::to_value(result)?,
                });
            }
        }

        Ok(AssistantResponse {
            tool: "help".into(),
            title: "Try one of these".into(),
            explanation: "I can help with schedule drafts, queue order, open shifts, guard profiles, certification renewals, and Sentinel alerts.".into(),
            data: serde_json::to_value(self.assistant_examples())?,
        })
    }

    /// Returns true if this integration key holds a credential (should use keyring).
    fn is_credential_key(key: &str) -> bool {
        key.contains("api_key") || key.contains("token") || key.contains("secret") || key.contains("password")
    }

    pub fn get_integrations(&self) -> Result<Vec<IntegrationStatus>> {
        let conn = self.db.connect()?;
        let integrations = vec![
            ("sling_api_key", "Sling", "Workforce scheduling platform. Connect to import/export shift schedules."),
            ("openclaw_endpoint", "OpenClaw Agent", "AI orchestration endpoint for natural-language scheduling commands."),
            ("cv_endpoint", "CV Worker", "Computer vision worker endpoint for Sentinel camera analysis (e.g. http://localhost:5050)."),
            ("messaging_provider", "Messaging Provider", "GroupMe or SMS provider for staff notifications and alerts."),
        ];
        let mut result = Vec::new();
        for (key, label, description) in integrations {
            let value: String = if Self::is_credential_key(key) {
                keyring::Entry::new("lifebot", &format!("integration-{}", key))
                    .and_then(|e| e.get_password())
                    .unwrap_or_default()
            } else {
                conn.query_row(
                    "SELECT value FROM app_settings WHERE key = ?1",
                    params![key],
                    |row| row.get(0),
                )
                .unwrap_or_default()
            };
            result.push(IntegrationStatus {
                key: key.into(),
                label: label.into(),
                connected: !value.is_empty(),
                value: value.clone(),
                description: description.into(),
            });
        }
        Ok(result)
    }

    pub fn save_integration(&self, key: &str, value: &str) -> Result<()> {
        let allowed = ["sling_api_key", "openclaw_endpoint", "cv_endpoint", "messaging_provider"];
        if !allowed.contains(&key) {
            anyhow::bail!("Unknown integration key: {}", key);
        }
        if Self::is_credential_key(key) {
            keyring::Entry::new("lifebot", &format!("integration-{}", key))
                .and_then(|e| e.set_password(value))
                .map_err(|e| anyhow::anyhow!("Keyring error: {}", e))?;
        } else {
            let conn = self.db.connect()?;
            conn.execute(
                "INSERT INTO app_settings (key, value) VALUES (?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![key, value],
            )?;
        }
        self.log_message("in_app", &format!("Integration '{}' updated.", key))?;
        Ok(())
    }

    pub fn disconnect_integration(&self, key: &str) -> Result<()> {
        if Self::is_credential_key(key) {
            if let Ok(entry) = keyring::Entry::new("lifebot", &format!("integration-{}", key)) {
                let _ = entry.delete_credential();
            }
        } else {
            let conn = self.db.connect()?;
            conn.execute("DELETE FROM app_settings WHERE key = ?1", params![key])?;
        }
        self.log_message("in_app", &format!("Integration '{}' disconnected.", key))?;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Sentinel
    // -----------------------------------------------------------------------

    pub fn sentinel_dashboard(&self) -> Result<crate::sentinel::SentinelDashboard> {
        let conn = self.db.connect()?;
        crate::sentinel::get_sentinel_dashboard(&conn)
    }

    pub fn sentinel_active_alerts(&self) -> Result<Vec<crate::sentinel::SentinelAlert>> {
        let conn = self.db.connect()?;
        crate::sentinel::list_active_alerts(&conn)
    }

    pub fn sentinel_all_alerts(&self, limit: i64) -> Result<Vec<crate::sentinel::SentinelAlert>> {
        let conn = self.db.connect()?;
        crate::sentinel::list_all_alerts(&conn, limit)
    }

    pub fn sentinel_event_history(&self, limit: i64) -> Result<Vec<crate::sentinel::SentinelEvent>> {
        let conn = self.db.connect()?;
        crate::sentinel::list_events(&conn, limit)
    }

    pub fn sentinel_simulate_event(
        &self,
        zone_id: &str,
        event_type: &str,
        confidence: f64,
        duration_secs: f64,
    ) -> Result<crate::sentinel::SentinelAlert> {
        let conn = self.db.connect()?;
        crate::sentinel::simulate_event(&conn, zone_id, event_type, confidence, duration_secs)
    }

    pub fn sentinel_acknowledge(
        &self,
        alert_id: &str,
        guard_id: &str,
        action: &str,
        notes: &str,
    ) -> Result<()> {
        let conn = self.db.connect()?;
        crate::sentinel::acknowledge_alert(&conn, alert_id, guard_id, action, notes)
    }

    pub fn sentinel_alert_detail(&self, alert_id: &str) -> Result<Option<crate::sentinel::SentinelAlert>> {
        let conn = self.db.connect()?;
        crate::sentinel::get_alert_detail(&conn, alert_id)
    }

    pub fn sentinel_zones(&self) -> Result<Vec<crate::sentinel::PoolZone>> {
        let conn = self.db.connect()?;
        crate::sentinel::list_zones(&conn)
    }

    pub fn sentinel_add_camera(&self, site_id: &str, name: &str, location: &str, stream_url: &str) -> Result<crate::sentinel::Camera> {
        let conn = self.db.connect()?;
        crate::sentinel::add_camera(&conn, site_id, name, location, stream_url)
    }

    pub fn sentinel_update_camera(&self, camera_id: &str, name: &str, location: &str, stream_url: &str, active: bool) -> Result<()> {
        let conn = self.db.connect()?;
        crate::sentinel::update_camera(&conn, camera_id, name, location, stream_url, active)
    }

    pub fn sentinel_delete_camera(&self, camera_id: &str) -> Result<()> {
        let conn = self.db.connect()?;
        crate::sentinel::delete_camera(&conn, camera_id)
    }

    pub fn sentinel_assign_camera_to_zone(&self, zone_id: &str, camera_id: Option<&str>) -> Result<()> {
        let conn = self.db.connect()?;
        crate::sentinel::assign_camera_to_zone(&conn, zone_id, camera_id)
    }

    pub fn sentinel_cameras(&self) -> Result<Vec<crate::sentinel::Camera>> {
        let conn = self.db.connect()?;
        crate::sentinel::list_cameras(&conn)
    }

    pub fn sentinel_cv_health(&self) -> Result<bool> {
        let conn = self.db.connect()?;
        let endpoint: String = conn.query_row(
            "SELECT value FROM app_settings WHERE key = 'cv_endpoint'",
            [],
            |r| r.get(0),
        ).unwrap_or_default();
        if endpoint.is_empty() {
            return Ok(false);
        }
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build()?;
        rt.block_on(crate::sentinel::cv_health_check(&endpoint))
    }

    pub fn sentinel_run_detection_pass(&self) -> Result<Vec<crate::sentinel::SentinelAlert>> {
        let conn = self.db.connect()?;
        let endpoint: String = conn.query_row(
            "SELECT value FROM app_settings WHERE key = 'cv_endpoint'",
            [],
            |r| r.get(0),
        ).unwrap_or_default();
        if endpoint.is_empty() {
            anyhow::bail!("No CV endpoint configured. Set it in Integrations → OpenClaw Agent or add 'cv_endpoint' to app_settings.");
        }
        crate::sentinel::run_detection_pass_sync(&conn, &endpoint)
    }

    pub fn sentinel_supervisors_for_pool(&self, pool_id: &str) -> Result<Vec<(String, String)>> {
        let conn = self.db.connect()?;
        let site_id: String = conn.query_row("SELECT site_id FROM pools WHERE id = ?1", params![pool_id], |r| r.get(0))?;
        crate::sentinel::find_current_supervisors(&conn, &site_id)
    }

    // -----------------------------------------------------------------------
    // Sling connect, import, and cycle management
    // -----------------------------------------------------------------------

    /// Public accessor for the DB so Tauri commands can read stored credentials.
    pub fn db(&self) -> &LifebotDb {
        &self.db
    }

    /// Query app_settings and DB counts to build a SetupStatus snapshot.
    pub fn setup_status(&self) -> Result<SetupStatus> {
        let conn = self.db.connect()?;

        let app_mode: String = conn
            .query_row(
                "SELECT value FROM app_settings WHERE key = 'app_mode'",
                [],
                |row| row.get(0),
            )
            .unwrap_or_else(|_| "uninitialized".to_string());

        let last_import: Option<String> = conn
            .query_row(
                "SELECT completed_at FROM import_runs ORDER BY completed_at DESC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .optional()?;

        let guard_count = count(&conn, "SELECT COUNT(*) FROM guards")?;
        let site_count = count(&conn, "SELECT COUNT(*) FROM sites")?;
        let template_count = count(&conn, "SELECT COUNT(*) FROM shift_templates")?;

        Ok(SetupStatus {
            app_mode,
            sling_connected: self.get_sling_token().is_ok(),
            last_import,
            guard_count,
            site_count,
            template_count,
        })
    }

    /// Set app_mode in app_settings. If mode == "demo", seed demo data.
    pub fn init_app_mode(&self, mode: &str) -> Result<()> {
        let conn = self.db.connect()?;
        conn.execute(
            "INSERT INTO app_settings (key, value, updated_at) VALUES ('app_mode', ?1, datetime('now'))
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = datetime('now')",
            params![mode],
        )?;
        if mode == "demo" {
            seed_demo(&conn)?;
            crate::sentinel::seed_sentinel_demo(&conn)?;
        }
        Ok(())
    }

    /// Store Sling session credentials securely and set app_mode to "live".
    /// The token is stored in the OS keychain (macOS Keychain, Windows
    /// Credential Manager, Linux Secret Service). Only the org_id and
    /// connection status go into SQLite.
    pub fn store_sling_session(&self, token: &str, org_id: i64) -> Result<()> {
        // Store token in OS keychain — never in the database
        let entry = keyring::Entry::new("lifebot", "sling-token")
            .context("Failed to access OS keychain")?;
        entry
            .set_password(token)
            .context("Failed to store Sling token in OS keychain")?;

        // Store non-secret metadata in DB
        let conn = self.db.connect()?;
        conn.execute(
            "INSERT INTO app_settings (key, value, updated_at) VALUES ('sling_org_id', ?1, datetime('now'))
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = datetime('now')",
            params![org_id.to_string()],
        )?;
        conn.execute(
            "INSERT INTO app_settings (key, value, updated_at) VALUES ('sling_connected', 'true', datetime('now'))
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = datetime('now')",
            [],
        )?;
        conn.execute(
            "INSERT INTO app_settings (key, value, updated_at) VALUES ('app_mode', 'live', datetime('now'))
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = datetime('now')",
            [],
        )?;
        Ok(())
    }

    /// Retrieve the Sling token from the OS keychain.
    pub fn get_sling_token(&self) -> Result<String> {
        let entry = keyring::Entry::new("lifebot", "sling-token")
            .context("Failed to access OS keychain")?;
        entry
            .get_password()
            .context("No Sling token found in OS keychain — please reconnect")
    }

    /// Map Sling API data through the mapping layer, upsert into the DB, and
    /// record an import run. Returns a summary of what was imported.
    pub fn run_import(
        &self,
        users: Vec<lifebot_sling::SlingUser>,
        groups: Vec<lifebot_sling::SlingGroup>,
        shifts: Vec<lifebot_sling::SlingShift>,
        cycle_id: &str,
    ) -> Result<ImportRunResult> {
        let conn = self.db.connect()?;

        // Map users → guards.
        let guard_imports: Vec<_> = users.iter().map(mapping::map_user_to_guard).collect();
        let (guards_imported, guards_updated) = import::upsert_guards(&conn, &guard_imports)?;

        // Split groups into locations and positions.
        let (locations, positions) = mapping::split_groups(&groups);
        let location_pairs: Vec<(i64, String)> = locations
            .iter()
            .map(|g| (g.id, g.name.clone()))
            .collect();
        let position_pairs: Vec<(i64, String)> = positions
            .iter()
            .map(|g| (g.id, g.name.clone()))
            .collect();

        let (sites_imported, _) = import::upsert_sites(&conn, &location_pairs)?;
        let (positions_imported, _) = import::upsert_roles(&conn, &position_pairs)?;

        // Map shifts, filtering out any that could not be parsed.
        let shift_imports: Vec<_> = shifts.iter().filter_map(mapping::map_shift).collect();
        let shifts_imported = import::import_shifts(&conn, &shift_imports, cycle_id)?;

        // Record the import run.
        import::record_import_run(
            &conn,
            guards_imported,
            guards_updated,
            sites_imported,
            positions_imported,
            shifts_imported,
            &[],
        )?;

        Ok(ImportRunResult {
            guards_imported,
            guards_updated,
            sites_imported,
            positions_imported,
            shifts_imported,
            errors: vec![],
        })
    }

    /// Insert a scheduling cycle in "draft" status.  Returns the new cycle id.
    pub fn create_cycle(
        &self,
        name: &str,
        starts_on: &str,
        ends_on: &str,
        rollover_deadline: &str,
    ) -> Result<String> {
        let id = format!("cycle-{}", starts_on);
        let conn = self.db.connect()?;
        conn.execute(
            "INSERT INTO scheduling_cycles (id, name, starts_on, ends_on, rollover_deadline, status)
             VALUES (?1, ?2, ?3, ?4, ?5, 'draft')",
            params![id, name, starts_on, ends_on, rollover_deadline],
        )?;
        Ok(id)
    }

    /// Build a list of Sling shift payloads for all reviewed/accepted assignments
    /// in the given cycle, ready to push to the Sling API.
    pub fn build_sling_export(&self, cycle_id: &str) -> Result<Vec<SlingShiftCreate>> {
        let conn = self.db.connect()?;
        let mut stmt = conn.prepare(
            "SELECT
                sh.shift_date,
                st.start_time,
                st.end_time,
                st.name,
                g.sling_id,
                si.sling_id,
                r.sling_id
             FROM shift_assignments sa
             JOIN shifts sh ON sh.id = sa.shift_id
             JOIN shift_templates st ON st.id = sh.template_id
             JOIN guards g ON g.id = sa.guard_id
             LEFT JOIN sites si ON si.id = st.site_id
             LEFT JOIN roles r ON r.id = st.role_id
             WHERE sh.cycle_id = ?1
               AND sa.status IN ('reviewed', 'accepted')"
        )?;
        let rows = stmt.query_map(params![cycle_id], |row| {
            Ok((
                row.get::<_, String>(0)?,   // shift_date
                row.get::<_, String>(1)?,   // start_time
                row.get::<_, String>(2)?,   // end_time
                row.get::<_, String>(3)?,   // template name (summary)
                row.get::<_, Option<i64>>(4)?, // guard sling_id
                row.get::<_, Option<i64>>(5)?, // site sling_id
                row.get::<_, Option<i64>>(6)?, // role sling_id
            ))
        })?;
        let mut result = Vec::new();
        for row in rows {
            let (shift_date, start_time, end_time, name, guard_sling_id, site_sling_id, role_sling_id) = row?;
            result.push(SlingShiftCreate {
                dtstart: format!("{}T{}:00Z", shift_date, start_time),
                dtend: format!("{}T{}:00Z", shift_date, end_time),
                user: guard_sling_id.map(|id| SlingShiftUser { id }),
                location: site_sling_id.map(|id| SlingShiftRef { id }),
                position: role_sling_id.map(|id| SlingShiftRef { id }),
                summary: Some(name),
            });
        }
        Ok(result)
    }

    pub fn log_message(&self, provider: &str, body: &str) -> Result<()> {
        let conn = self.db.connect()?;
        conn.execute(
            "INSERT INTO message_log (id, provider_name, recipient, body, created_at)
             VALUES (?1, ?2, 'local-demo', ?3, ?4)",
            params![
                format!("msg-{}", uuid::Uuid::new_v4()),
                provider,
                body,
                Utc::now().naive_utc().to_string()
            ],
        )?;
        Ok(())
    }

}

fn count(conn: &rusqlite::Connection, sql: &str) -> Result<i64> {
    Ok(conn.query_row(sql, [], |row| row.get(0))?)
}

fn load_string_list(conn: &rusqlite::Connection, sql: &str, guard_id: &str) -> Result<Vec<String>> {
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map(params![guard_id], |row| row.get(0))?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

fn load_pairs(conn: &rusqlite::Connection, sql: &str, guard_id: &str) -> Result<Vec<(String, String)>> {
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map(params![guard_id], |row| Ok((row.get(0)?, row.get(1)?)))?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

fn age_on(dob: NaiveDate, today: NaiveDate) -> i64 {
    let mut years = today.year() - dob.year();
    if (today.month(), today.day()) < (dob.month(), dob.day()) {
        years -= 1;
    }
    years as i64
}
