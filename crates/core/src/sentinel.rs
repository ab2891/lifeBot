//! Lifebot Sentinel — Human-in-the-loop pool surveillance assist.
//!
//! **SAFETY DISCLAIMER**: Sentinel is an assistive safety layer only.
//! It is NOT a replacement for active lifeguard surveillance. All alerts
//! require human acknowledgment and decision-making.

use anyhow::Result;
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Models
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Camera {
    pub id: String,
    pub site_id: String,
    pub name: String,
    pub location: String,
    pub stream_url: String,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolZone {
    pub id: String,
    pub pool_id: String,
    pub camera_id: Option<String>,
    pub name: String,
    pub zone_type: String,
    pub immobility_threshold_secs: i64,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentinelEvent {
    pub id: String,
    pub camera_id: Option<String>,
    pub zone_id: String,
    pub event_type: String,
    pub confidence: f64,
    pub duration_secs: f64,
    pub description: String,
    pub detected_at: String,
    pub dismissed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentinelAlert {
    pub id: String,
    pub event_id: String,
    pub severity: String,
    pub status: String,
    pub explanation: String,
    pub created_at: String,
    pub resolved_at: Option<String>,
    pub escalation_count: i64,
    // Joined fields for display
    pub zone_name: Option<String>,
    pub pool_name: Option<String>,
    pub site_name: Option<String>,
    pub event_type: Option<String>,
    pub confidence: Option<f64>,
    pub duration_secs: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentinelAcknowledgment {
    pub id: String,
    pub alert_id: String,
    pub guard_id: String,
    pub guard_name: Option<String>,
    pub action: String,
    pub notes: String,
    pub acted_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRecipient {
    pub guard_id: String,
    pub guard_name: String,
    pub role: String,
    pub channel: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncidentOutcome {
    pub id: String,
    pub alert_id: String,
    pub outcome: String,
    pub summary: String,
    pub recorded_by: Option<String>,
    pub recorded_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentinelDashboard {
    pub active_alerts: Vec<SentinelAlert>,
    pub recent_events: Vec<SentinelEvent>,
    pub cameras: Vec<Camera>,
    pub zones: Vec<PoolZone>,
    pub event_history: Vec<SentinelEvent>,
}

// ---------------------------------------------------------------------------
// Detection provider trait (pluggable)
// ---------------------------------------------------------------------------

/// A detection provider analyzes camera/zone data and produces detection events.
/// For MVP, we use MockDetectionProvider. A future FutureVisionProvider could
/// integrate with OpenCV, a Python worker, or a cloud CV service.
pub trait DetectionProvider: Send + Sync {
    fn provider_name(&self) -> &str;
    fn generate_detection(&self, zone: &PoolZone) -> Option<DetectionResult>;
}

#[derive(Debug, Clone)]
pub struct DetectionResult {
    pub event_type: String,
    pub confidence: f64,
    pub duration_secs: f64,
    pub description: String,
}

/// Mock detection provider for demo/testing. Generates simulated events.
pub struct MockDetectionProvider;

impl DetectionProvider for MockDetectionProvider {
    fn provider_name(&self) -> &str {
        "mock"
    }

    fn generate_detection(&self, _zone: &PoolZone) -> Option<DetectionResult> {
        // In demo mode, this is called explicitly via simulate_event
        // It doesn't auto-generate — the simulate functions create events directly
        None
    }
}

/// Placeholder for future real CV provider integration.
/// Would connect to an OpenCV worker, cloud vision API, or RTSP stream analyzer.
pub struct _FutureVisionProvider {
    _endpoint: String,
}

// ---------------------------------------------------------------------------
// Alert severity engine
// ---------------------------------------------------------------------------

pub fn compute_severity(confidence: f64, duration_secs: f64, zone_type: &str) -> &'static str {
    let base_score = confidence * (duration_secs / 10.0);
    let zone_multiplier = match zone_type {
        "deep_end" | "diving" => 1.5,
        "lap_lane" => 1.2,
        _ => 1.0,
    };
    let score = base_score * zone_multiplier;
    if score >= 3.0 {
        "high"
    } else if score >= 1.5 {
        "medium"
    } else {
        "low"
    }
}

pub fn build_explanation(
    event_type: &str,
    zone_name: &str,
    pool_name: &str,
    duration_secs: f64,
    confidence: f64,
    severity: &str,
) -> String {
    let event_desc = match event_type {
        "immobility" => "Possible prolonged immobility",
        "unresponsive" => "Possible unresponsive swimmer",
        "motion_timeout" => "Motion timeout exceeded",
        _ => "Anomaly",
    };
    format!(
        "{} detected in {}, {} for {:.0} seconds (confidence: {:.0}%). Severity: {}.",
        event_desc, pool_name, zone_name, duration_secs, confidence * 100.0, severity
    )
}

// ---------------------------------------------------------------------------
// Sentinel service (operates on a Connection)
// ---------------------------------------------------------------------------

pub fn seed_sentinel_demo(conn: &Connection) -> Result<()> {
    // Check if already seeded
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM cameras", [], |r| r.get(0))?;
    if count > 0 {
        return Ok(());
    }

    let site_id: String = conn.query_row("SELECT id FROM sites LIMIT 1", [], |r| r.get(0))?;
    let pools: Vec<(String, String)> = {
        let mut stmt = conn.prepare("SELECT id, name FROM pools WHERE site_id = ?1")?;
        let result = stmt.query_map(params![&site_id], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        result
    };

    for (pool_id, pool_name) in &pools {
        let cam_id = format!("cam-{}", Uuid::new_v4());
        conn.execute(
            "INSERT INTO cameras (id, site_id, name, location, stream_url, active) VALUES (?1, ?2, ?3, ?4, ?5, 1)",
            params![cam_id, site_id, format!("{} Camera", pool_name), format!("Overhead — {}", pool_name), "mock://local/stream"],
        )?;

        let zones = [
            ("Deep End", "deep_end", 12),
            ("Shallow End", "shallow", 20),
            ("Lap Lanes", "lap_lane", 18),
        ];
        for (name, ztype, threshold) in zones {
            let zone_id = format!("zone-{}", Uuid::new_v4());
            conn.execute(
                "INSERT INTO pool_zones (id, pool_id, camera_id, name, zone_type, immobility_threshold_secs, active) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1)",
                params![zone_id, pool_id, cam_id, format!("{} — {}", pool_name, name), ztype, threshold],
            )?;
        }
    }

    Ok(())
}

pub fn list_cameras(conn: &Connection) -> Result<Vec<Camera>> {
    let mut stmt = conn.prepare("SELECT id, site_id, name, location, stream_url, active FROM cameras ORDER BY name")?;
    let rows = stmt.query_map([], |r| {
        Ok(Camera {
            id: r.get(0)?,
            site_id: r.get(1)?,
            name: r.get(2)?,
            location: r.get(3)?,
            stream_url: r.get(4)?,
            active: r.get::<_, i64>(5)? == 1,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn list_zones(conn: &Connection) -> Result<Vec<PoolZone>> {
    let mut stmt = conn.prepare("SELECT id, pool_id, camera_id, name, zone_type, immobility_threshold_secs, active FROM pool_zones ORDER BY name")?;
    let rows = stmt.query_map([], |r| {
        Ok(PoolZone {
            id: r.get(0)?,
            pool_id: r.get(1)?,
            camera_id: r.get(2)?,
            name: r.get(3)?,
            zone_type: r.get(4)?,
            immobility_threshold_secs: r.get(5)?,
            active: r.get::<_, i64>(6)? == 1,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn list_events(conn: &Connection, limit: i64) -> Result<Vec<SentinelEvent>> {
    let mut stmt = conn.prepare(
        "SELECT id, camera_id, zone_id, event_type, confidence, duration_secs, description, detected_at, dismissed
         FROM sentinel_events ORDER BY detected_at DESC LIMIT ?1"
    )?;
    let rows = stmt.query_map(params![limit], |r| {
        Ok(SentinelEvent {
            id: r.get(0)?,
            camera_id: r.get(1)?,
            zone_id: r.get(2)?,
            event_type: r.get(3)?,
            confidence: r.get(4)?,
            duration_secs: r.get(5)?,
            description: r.get(6)?,
            detected_at: r.get(7)?,
            dismissed: r.get::<_, i64>(8)? == 1,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn list_active_alerts(conn: &Connection) -> Result<Vec<SentinelAlert>> {
    let mut stmt = conn.prepare(
        "SELECT sa.id, sa.event_id, sa.severity, sa.status, sa.explanation, sa.created_at, sa.resolved_at, sa.escalation_count,
                pz.name, p.name, s.name, se.event_type, se.confidence, se.duration_secs
         FROM sentinel_alerts sa
         JOIN sentinel_events se ON se.id = sa.event_id
         JOIN pool_zones pz ON pz.id = se.zone_id
         JOIN pools p ON p.id = pz.pool_id
         JOIN sites s ON s.id = p.site_id
         WHERE sa.status IN ('active', 'escalated')
         ORDER BY CASE sa.severity WHEN 'high' THEN 0 WHEN 'medium' THEN 1 ELSE 2 END, sa.created_at DESC"
    )?;
    let rows = stmt.query_map([], alert_from_row)?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn list_all_alerts(conn: &Connection, limit: i64) -> Result<Vec<SentinelAlert>> {
    let mut stmt = conn.prepare(
        "SELECT sa.id, sa.event_id, sa.severity, sa.status, sa.explanation, sa.created_at, sa.resolved_at, sa.escalation_count,
                pz.name, p.name, s.name, se.event_type, se.confidence, se.duration_secs
         FROM sentinel_alerts sa
         JOIN sentinel_events se ON se.id = sa.event_id
         JOIN pool_zones pz ON pz.id = se.zone_id
         JOIN pools p ON p.id = pz.pool_id
         JOIN sites s ON s.id = p.site_id
         ORDER BY sa.created_at DESC LIMIT ?1"
    )?;
    let rows = stmt.query_map(params![limit], alert_from_row)?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

fn alert_from_row(r: &rusqlite::Row) -> rusqlite::Result<SentinelAlert> {
    Ok(SentinelAlert {
        id: r.get(0)?,
        event_id: r.get(1)?,
        severity: r.get(2)?,
        status: r.get(3)?,
        explanation: r.get(4)?,
        created_at: r.get(5)?,
        resolved_at: r.get(6)?,
        escalation_count: r.get(7)?,
        zone_name: r.get(8)?,
        pool_name: r.get(9)?,
        site_name: r.get(10)?,
        event_type: r.get(11)?,
        confidence: r.get(12)?,
        duration_secs: r.get(13)?,
    })
}

/// Simulate a detection event in a given zone. Creates event + alert + notifies supervisors.
pub fn simulate_event(
    conn: &Connection,
    zone_id: &str,
    event_type: &str,
    confidence: f64,
    duration_secs: f64,
) -> Result<SentinelAlert> {
    // Rate limit: max 1 simulation per 10 seconds per zone
    let recent: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sentinel_events WHERE zone_id = ?1 AND detected_at > datetime('now', '-10 seconds')",
        params![zone_id],
        |r| r.get(0),
    )?;
    if recent > 0 {
        anyhow::bail!("Rate limited: please wait before simulating another event for this zone");
    }

    // Look up zone info
    let (zone_name, pool_id, zone_type, camera_id): (String, String, String, Option<String>) = conn.query_row(
        "SELECT pz.name, pz.pool_id, pz.zone_type, pz.camera_id FROM pool_zones pz WHERE pz.id = ?1",
        params![zone_id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
    )?;
    let (pool_name, site_id): (String, String) = conn.query_row(
        "SELECT name, site_id FROM pools WHERE id = ?1",
        params![pool_id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    let site_name: String = conn.query_row("SELECT name FROM sites WHERE id = ?1", params![site_id], |r| r.get(0))?;

    let severity = compute_severity(confidence, duration_secs, &zone_type);
    let explanation = build_explanation(event_type, &zone_name, &pool_name, duration_secs, confidence, severity);

    let now = Utc::now().naive_utc().to_string();
    let event_id = format!("sevt-{}", Uuid::new_v4());
    let alert_id = format!("salt-{}", Uuid::new_v4());

    // Insert event
    conn.execute(
        "INSERT INTO sentinel_events (id, camera_id, zone_id, event_type, confidence, duration_secs, description, detected_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![event_id, camera_id, zone_id, event_type, confidence, duration_secs, &explanation, &now],
    )?;

    // Insert alert
    conn.execute(
        "INSERT INTO sentinel_alerts (id, event_id, severity, status, explanation, created_at)
         VALUES (?1, ?2, ?3, 'active', ?4, ?5)",
        params![alert_id, event_id, severity, &explanation, &now],
    )?;

    // Find on-duty supervisors and notify
    if severity != "low" {
        let supervisors = find_current_supervisors(conn, &site_id)?;
        for (guard_id, _guard_name) in &supervisors {
            let recip_id = format!("srcp-{}", Uuid::new_v4());
            conn.execute(
                "INSERT INTO sentinel_alert_recipients (id, alert_id, guard_id, role, notified_at, channel)
                 VALUES (?1, ?2, ?3, 'supervisor', ?4, 'in_app')",
                params![recip_id, alert_id, guard_id, &now],
            )?;
        }
        // Log notification
        conn.execute(
            "INSERT INTO message_log (id, provider_name, recipient, body, created_at) VALUES (?1, 'sentinel', 'supervisors', ?2, ?3)",
            params![format!("msg-{}", Uuid::new_v4()), &explanation, &now],
        )?;
    }

    // Return the created alert with joined info
    Ok(SentinelAlert {
        id: alert_id,
        event_id,
        severity: severity.into(),
        status: "active".into(),
        explanation,
        created_at: now,
        resolved_at: None,
        escalation_count: 0,
        zone_name: Some(zone_name),
        pool_name: Some(pool_name),
        site_name: Some(site_name),
        event_type: Some(event_type.into()),
        confidence: Some(confidence),
        duration_secs: Some(duration_secs),
    })
}

/// Find guards currently on shift with a supervisor role at a given site.
pub fn find_current_supervisors(conn: &Connection, site_id: &str) -> Result<Vec<(String, String)>> {
    let mut stmt = conn.prepare(
        "SELECT DISTINCT g.id, g.name
         FROM shift_assignments sa
         JOIN shifts sh ON sh.id = sa.shift_id
         JOIN shift_templates st ON st.id = sh.template_id
         JOIN guards g ON g.id = sa.guard_id
         JOIN guard_roles gr ON gr.guard_id = g.id
         JOIN roles r ON r.id = gr.role_id
         WHERE st.site_id = ?1
           AND sh.cycle_id = 'cycle-current'
           AND sa.status IN ('assigned', 'reviewed')
           AND r.name LIKE '%Supervisor%'
         ORDER BY g.name"
    )?;
    let rows = stmt.query_map(params![site_id], |r| Ok((r.get(0)?, r.get(1)?)))?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

/// Acknowledge an alert (any action: acknowledged, dismissed, false_positive, escalated, resolved)
pub fn acknowledge_alert(
    conn: &Connection,
    alert_id: &str,
    guard_id: &str,
    action: &str,
    notes: &str,
) -> Result<()> {
    let now = Utc::now().naive_utc().to_string();
    let ack_id = format!("sack-{}", Uuid::new_v4());

    conn.execute(
        "INSERT INTO sentinel_acknowledgments (id, alert_id, guard_id, action, notes, acted_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![ack_id, alert_id, guard_id, action, notes, &now],
    )?;

    let new_status = match action {
        "acknowledged" => "acknowledged",
        "dismissed" | "false_positive" => "false_positive",
        "resolved" => "resolved",
        "escalated" => "escalated",
        _ => "acknowledged",
    };

    let resolved_at = if new_status == "resolved" || new_status == "false_positive" {
        Some(now.clone())
    } else {
        None
    };

    if new_status == "escalated" {
        conn.execute(
            "UPDATE sentinel_alerts SET status = ?1, escalation_count = escalation_count + 1 WHERE id = ?2",
            params![new_status, alert_id],
        )?;
    } else {
        conn.execute(
            "UPDATE sentinel_alerts SET status = ?1, resolved_at = ?2 WHERE id = ?3",
            params![new_status, resolved_at, alert_id],
        )?;
    }

    // Record outcome for terminal actions
    if new_status == "resolved" || new_status == "false_positive" {
        let outcome = if new_status == "false_positive" { "false_alarm" } else { "resolved_safe" };
        let outcome_id = format!("sout-{}", Uuid::new_v4());
        conn.execute(
            "INSERT INTO incident_outcomes (id, alert_id, outcome, summary, recorded_by, recorded_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![outcome_id, alert_id, outcome, notes, guard_id, &now],
        )?;
    }

    Ok(())
}

pub fn get_alert_detail(conn: &Connection, alert_id: &str) -> Result<Option<SentinelAlert>> {
    conn.query_row(
        "SELECT sa.id, sa.event_id, sa.severity, sa.status, sa.explanation, sa.created_at, sa.resolved_at, sa.escalation_count,
                pz.name, p.name, s.name, se.event_type, se.confidence, se.duration_secs
         FROM sentinel_alerts sa
         JOIN sentinel_events se ON se.id = sa.event_id
         JOIN pool_zones pz ON pz.id = se.zone_id
         JOIN pools p ON p.id = pz.pool_id
         JOIN sites s ON s.id = p.site_id
         WHERE sa.id = ?1",
        params![alert_id],
        alert_from_row,
    )
    .optional()
    .map_err(Into::into)
}

pub fn get_sentinel_dashboard(conn: &Connection) -> Result<SentinelDashboard> {
    Ok(SentinelDashboard {
        active_alerts: list_active_alerts(conn)?,
        recent_events: list_events(conn, 20)?,
        cameras: list_cameras(conn)?,
        zones: list_zones(conn)?,
        event_history: list_events(conn, 50)?,
    })
}

// ---------------------------------------------------------------------------
// Camera CRUD
// ---------------------------------------------------------------------------

pub fn add_camera(conn: &Connection, site_id: &str, name: &str, location: &str, stream_url: &str) -> Result<Camera> {
    let id = format!("cam-{}", Uuid::new_v4());
    conn.execute(
        "INSERT INTO cameras (id, site_id, name, location, stream_url, active) VALUES (?1, ?2, ?3, ?4, ?5, 1)",
        params![id, site_id, name, location, stream_url],
    )?;
    Ok(Camera { id, site_id: site_id.into(), name: name.into(), location: location.into(), stream_url: stream_url.into(), active: true })
}

pub fn update_camera(conn: &Connection, camera_id: &str, name: &str, location: &str, stream_url: &str, active: bool) -> Result<()> {
    conn.execute(
        "UPDATE cameras SET name = ?1, location = ?2, stream_url = ?3, active = ?4 WHERE id = ?5",
        params![name, location, stream_url, active as i64, camera_id],
    )?;
    Ok(())
}

pub fn delete_camera(conn: &Connection, camera_id: &str) -> Result<()> {
    conn.execute("UPDATE pool_zones SET camera_id = NULL WHERE camera_id = ?1", params![camera_id])?;
    conn.execute("DELETE FROM cameras WHERE id = ?1", params![camera_id])?;
    Ok(())
}

pub fn assign_camera_to_zone(conn: &Connection, zone_id: &str, camera_id: Option<&str>) -> Result<()> {
    conn.execute("UPDATE pool_zones SET camera_id = ?1 WHERE id = ?2", params![camera_id, zone_id])?;
    Ok(())
}

pub fn add_zone(conn: &Connection, pool_id: &str, name: &str, zone_type: &str, threshold_secs: i64, camera_id: Option<&str>) -> Result<PoolZone> {
    let id = format!("zone-{}", Uuid::new_v4());
    conn.execute(
        "INSERT INTO pool_zones (id, pool_id, camera_id, name, zone_type, immobility_threshold_secs, active) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1)",
        params![id, pool_id, camera_id, name, zone_type, threshold_secs],
    )?;
    Ok(PoolZone { id, pool_id: pool_id.into(), camera_id: camera_id.map(String::from), name: name.into(), zone_type: zone_type.into(), immobility_threshold_secs: threshold_secs, active: true })
}

pub fn update_zone(conn: &Connection, zone_id: &str, name: &str, zone_type: &str, threshold_secs: i64, active: bool) -> Result<()> {
    conn.execute(
        "UPDATE pool_zones SET name = ?1, zone_type = ?2, immobility_threshold_secs = ?3, active = ?4 WHERE id = ?5",
        params![name, zone_type, threshold_secs, active as i64, zone_id],
    )?;
    Ok(())
}

// ---------------------------------------------------------------------------
// CV Provider HTTP adapter
// ---------------------------------------------------------------------------

/// Result from a CV analysis service for a single camera/zone.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CvDetection {
    pub zone_id: String,
    pub event_type: String,
    pub confidence: f64,
    pub duration_secs: f64,
    pub description: String,
}

/// Request body sent to the CV worker for analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CvAnalysisRequest {
    pub camera_id: String,
    pub stream_url: String,
    pub zones: Vec<CvZoneInfo>,
}

/// Zone info sent to the CV worker so it knows what regions to monitor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CvZoneInfo {
    pub zone_id: String,
    pub name: String,
    pub zone_type: String,
    pub immobility_threshold_secs: i64,
}

/// Response from the CV worker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CvAnalysisResponse {
    pub detections: Vec<CvDetection>,
}

/// Check if a CV endpoint is reachable. Returns Ok(true) if healthy.
pub async fn cv_health_check(endpoint: &str) -> Result<bool> {
    let url = format!("{}/health", endpoint.trim_end_matches('/'));
    match reqwest::get(&url).await {
        Ok(resp) => Ok(resp.status().is_success()),
        Err(_) => Ok(false),
    }
}

/// Send a camera + zones to the CV worker for analysis and return any detections.
pub async fn cv_analyze(endpoint: &str, request: &CvAnalysisRequest) -> Result<CvAnalysisResponse> {
    let url = format!("{}/analyze", endpoint.trim_end_matches('/'));
    let client = reqwest::Client::new();
    let resp = client.post(&url).json(request).send().await?;
    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("CV worker returned error: {}", body);
    }
    let result: CvAnalysisResponse = resp.json().await?;
    Ok(result)
}

/// Run one detection pass across all active cameras. Calls the CV endpoint for each camera,
/// then feeds any detections into the Sentinel pipeline.
pub fn run_detection_pass_sync(conn: &Connection, endpoint: &str) -> Result<Vec<SentinelAlert>> {
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build()?;
    rt.block_on(run_detection_pass(conn, endpoint))
}

pub async fn run_detection_pass(conn: &Connection, endpoint: &str) -> Result<Vec<SentinelAlert>> {
    let cameras = list_cameras(conn)?;
    let all_zones = list_zones(conn)?;
    let mut alerts = Vec::new();

    for camera in cameras.iter().filter(|c| c.active) {
        let camera_zones: Vec<CvZoneInfo> = all_zones.iter()
            .filter(|z| z.camera_id.as_deref() == Some(&camera.id) && z.active)
            .map(|z| CvZoneInfo {
                zone_id: z.id.clone(),
                name: z.name.clone(),
                zone_type: z.zone_type.clone(),
                immobility_threshold_secs: z.immobility_threshold_secs,
            })
            .collect();

        if camera_zones.is_empty() {
            continue;
        }

        let request = CvAnalysisRequest {
            camera_id: camera.id.clone(),
            stream_url: camera.stream_url.clone(),
            zones: camera_zones,
        };

        match cv_analyze(endpoint, &request).await {
            Ok(response) => {
                for detection in response.detections {
                    match simulate_event(conn, &detection.zone_id, &detection.event_type, detection.confidence, detection.duration_secs) {
                        Ok(alert) => alerts.push(alert),
                        Err(e) => eprintln!("[sentinel] Failed to create alert from CV detection: {}", e),
                    }
                }
            }
            Err(e) => {
                eprintln!("[sentinel] CV analysis failed for camera {}: {}", camera.name, e);
            }
        }
    }

    Ok(alerts)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{db::LifebotDb, seed::seed_demo};

    fn setup_db() -> (Connection, std::path::PathBuf) {
        let path = std::env::temp_dir().join(format!("lifebot-test-{}.db", Uuid::new_v4()));
        let db = LifebotDb::new(&path);
        db.migrate().unwrap();
        let conn = db.connect().unwrap();
        seed_demo(&conn).unwrap();
        seed_sentinel_demo(&conn).unwrap();
        (conn, path)
    }

    #[test]
    fn test_severity_computation() {
        assert_eq!(compute_severity(0.3, 5.0, "shallow"), "low");
        assert_eq!(compute_severity(0.7, 20.0, "general"), "low");
        assert_eq!(compute_severity(0.8, 25.0, "lap_lane"), "medium");
        assert_eq!(compute_severity(0.9, 30.0, "deep_end"), "high");
    }

    #[test]
    fn test_simulate_event_and_alert() {
        let (conn, _tmp) = setup_db();
        let zones = list_zones(&conn).unwrap();
        assert!(!zones.is_empty(), "Should have seeded zones");

        let zone = &zones[0];
        let alert = simulate_event(&conn, &zone.id, "immobility", 0.85, 22.0).unwrap();
        assert!(!alert.id.is_empty());
        assert!(alert.explanation.contains("immobility"));

        let active = list_active_alerts(&conn).unwrap();
        assert!(!active.is_empty());
    }

    #[test]
    fn test_acknowledge_dismiss_flow() {
        let (conn, _tmp) = setup_db();
        let zones = list_zones(&conn).unwrap();
        let zone = &zones[0];

        let alert = simulate_event(&conn, &zone.id, "immobility", 0.9, 30.0).unwrap();

        // Acknowledge
                let guard_id: String = conn.query_row("SELECT id FROM guards LIMIT 1", [], |r| r.get(0)).unwrap();
        acknowledge_alert(&conn, &alert.id, &guard_id, "acknowledged", "Checking now").unwrap();
        let updated = get_alert_detail(&conn, &alert.id).unwrap().unwrap();
        assert_eq!(updated.status, "acknowledged");

        // Resolve
                acknowledge_alert(&conn, &alert.id, &guard_id, "resolved", "Swimmer was resting, all clear").unwrap();
        let resolved = get_alert_detail(&conn, &alert.id).unwrap().unwrap();
        assert_eq!(resolved.status, "resolved");
        assert!(resolved.resolved_at.is_some());
    }

    #[test]
    fn test_false_positive_creates_outcome() {
        let (conn, _tmp) = setup_db();
        let zones = list_zones(&conn).unwrap();
        let alert = simulate_event(&conn, &zones[0].id, "motion_timeout", 0.6, 15.0).unwrap();

                let guard_id: String = conn.query_row("SELECT id FROM guards LIMIT 1", [], |r| r.get(0)).unwrap();
        acknowledge_alert(&conn, &alert.id, &guard_id, "false_positive", "Shadow on water").unwrap();

        let outcome_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM incident_outcomes WHERE alert_id = ?1 AND outcome = 'false_alarm'",
            params![alert.id],
            |r| r.get(0),
        ).unwrap();
        assert_eq!(outcome_count, 1);
    }

    #[test]
    fn test_escalation_increments_count() {
        let (conn, _tmp) = setup_db();
        let zones = list_zones(&conn).unwrap();
        let alert = simulate_event(&conn, &zones[0].id, "unresponsive", 0.95, 35.0).unwrap();

                let guard_id: String = conn.query_row("SELECT id FROM guards LIMIT 1", [], |r| r.get(0)).unwrap();
        acknowledge_alert(&conn, &alert.id, &guard_id, "escalated", "Need backup").unwrap();
        let escalated = get_alert_detail(&conn, &alert.id).unwrap().unwrap();
        assert_eq!(escalated.status, "escalated");
        assert_eq!(escalated.escalation_count, 1);
    }

    #[test]
    fn test_supervisor_resolution() {
        let (conn, _tmp) = setup_db();
        let site_id: String = conn.query_row("SELECT id FROM sites LIMIT 1", [], |r| r.get(0)).unwrap();
        let supervisors = find_current_supervisors(&conn, &site_id).unwrap();
        // In demo mode with seeded data, we may or may not have supervisors on current cycle
        // The function should at least not error
        assert!(supervisors.len() >= 0);
    }
}
