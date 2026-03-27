use anyhow::{anyhow, Context, Result};
use chrono::{Datelike, NaiveDate, NaiveDateTime, Utc};
use lifebot_policies::{
    evaluate_candidate, ExistingAssignment, GuardContext, PolicyConfig, PolicyInput, ShiftContext,
};
use rusqlite::{params, Connection};
use serde_json::json;

use crate::models::{CandidateRequest, TemplateContext};

pub fn generate_next_cycle_draft(conn: &Connection) -> Result<()> {
    // Look up the draft cycle dynamically — bail with a clear error if none exists.
    let draft_cycle: Option<(String, String, String)> = conn
        .query_row(
            "SELECT id, starts_on, rollover_deadline FROM scheduling_cycles WHERE status = 'draft' LIMIT 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .ok();

    let (next_cycle_id, cycle_starts_on, cycle_rollover_deadline) = match draft_cycle {
        Some(row) => row,
        None => anyhow::bail!(
            "No draft scheduling cycle found. Create a scheduling cycle with status = 'draft' before generating a draft."
        ),
    };

    let next_exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM shifts WHERE cycle_id = ?1",
            params![next_cycle_id],
            |row| row.get(0),
        )
        .context("Failed to count existing shifts for the draft cycle")?;

    if next_exists > 0 {
        conn.execute("DELETE FROM decision_traces WHERE cycle_id = ?1", params![next_cycle_id])
            .context("Failed to clear previous decision traces for the draft cycle")?;
        conn.execute("DELETE FROM shift_assignments WHERE shift_id IN (SELECT id FROM shifts WHERE cycle_id = ?1)", params![next_cycle_id])
            .context("Failed to clear previous shift assignments for the draft cycle")?;
        conn.execute("DELETE FROM shifts WHERE cycle_id = ?1", params![next_cycle_id])
            .context("Failed to clear previous shifts for the draft cycle")?;
        conn.execute("UPDATE shift_requests SET status = 'queued', reason = NULL WHERE cycle_id = ?1", params![next_cycle_id])
            .context("Failed to reset shift request statuses for the draft cycle")?;
    }

    let next_cycle_start = NaiveDate::parse_from_str(&cycle_starts_on, "%Y-%m-%d")
        .context("Failed to parse draft cycle starts_on date")?;
    let rollover_deadline = NaiveDateTime::parse_from_str(&cycle_rollover_deadline, "%Y-%m-%d %H:%M:%S")
        .context("Failed to parse draft cycle rollover_deadline")?;

    for template in load_templates(conn)? {
        let shift_id = format!("shift-next-{}", template.template_id);
        let shift_date = next_date_for_day(next_cycle_start, &template.day_of_week)?;
        conn.execute(
            "INSERT INTO shifts (id, template_id, cycle_id, shift_date, status) VALUES (?1, ?2, ?3, ?4, 'draft')",
            params![shift_id, template.template_id, next_cycle_id, shift_date.to_string()],
        )
        .context("Failed to insert draft shift")?;

        let mut trace_steps = Vec::new();
        let mut winner: Option<String> = None;
        let mut winner_reason = String::new();

        if let (Some(incumbent_id), Some(incumbent_name)) =
            (&template.current_incumbent_guard_id, &template.current_incumbent_name)
        {
            let rollover: Option<String> = conn
                .query_row(
                    "SELECT requested_at FROM rollover_requests WHERE shift_template_id = ?1 AND guard_id = ?2 AND cycle_id = 'cycle-current' AND status = 'requested'",
                    params![template.template_id, incumbent_id],
                    |row| row.get(0),
                )
                .ok();

            match rollover {
                Some(requested_at) => {
                    let requested = NaiveDateTime::parse_from_str(&requested_at, "%Y-%m-%d %H:%M:%S")?;
                    if requested <= rollover_deadline
                        && evaluate_eligibility(conn, incumbent_id, &template, &shift_date, &shift_id).is_ok()
                    {
                        winner = Some(incumbent_id.clone());
                        winner_reason = format!("{incumbent_name} kept priority by requesting rollover before the deadline.");
                        trace_steps.push(json!({
                            "type": "rollover_awarded",
                            "guard": incumbent_name,
                            "requested_at": requested_at,
                            "reason": winner_reason
                        }));
                    } else {
                        trace_steps.push(json!({
                            "type": "rollover_missed_or_ineligible",
                            "guard": incumbent_name,
                            "requested_at": requested_at,
                            "reason": "The incumbent did not keep priority because the request was late or failed an eligibility check."
                        }));
                    }
                }
                None => {
                    trace_steps.push(json!({
                        "type": "rollover_absent",
                        "guard": incumbent_name,
                        "reason": "The incumbent did not request rollover before the deadline, so the shift opened."
                    }));
                }
            }
        }

        if winner.is_none() {
            let requests = load_requests_for_template(conn, &template.template_id, &next_cycle_id)?;
            if requests.is_empty() {
                trace_steps.push(json!({
                    "type": "no_requests",
                    "reason": "No requests received for this shift"
                }));
            }
            for request in requests {
                match evaluate_eligibility(conn, &request.guard_id, &template, &shift_date, &shift_id) {
                    Ok(()) => {
                        winner = Some(request.guard_id.clone());
                        winner_reason = format!(
                            "{} was the first eligible requester in line.",
                            request.guard_name
                        );
                        conn.execute(
                            "UPDATE shift_requests SET status = 'accepted', reason = ?2 WHERE id = ?1",
                            params![request.request_id, winner_reason],
                        )
                        .context("Failed to mark shift request as accepted")?;
                        trace_steps.push(json!({
                            "type": "request_awarded",
                            "guard": request.guard_name,
                            "requested_at": request.requested_at.to_rfc3339(),
                            "reason": winner_reason
                        }));
                        break;
                    }
                    Err(reason) => {
                        conn.execute(
                            "UPDATE shift_requests SET status = 'skipped', reason = ?2 WHERE id = ?1",
                            params![request.request_id, reason.to_string()],
                        )
                        .context("Failed to mark shift request as skipped")?;
                        trace_steps.push(json!({
                            "type": "request_skipped",
                            "guard": request.guard_name,
                            "requested_at": request.requested_at.to_rfc3339(),
                            "reason": reason.to_string()
                        }));
                    }
                }
            }
        }

        if let Some(guard_id) = winner.clone() {
            conn.execute(
                "INSERT INTO shift_assignments (id, shift_id, guard_id, status, assigned_at)
                 VALUES (?1, ?2, ?3, 'draft', ?4)",
                params![format!("assignment-{shift_id}"), shift_id, guard_id, Utc::now().naive_utc().to_string()],
            )?;
        }

        let summary = if let Some(guard_id) = winner {
            let guard_name: String = conn.query_row(
                "SELECT name FROM guards WHERE id = ?1",
                params![guard_id],
                |row| row.get(0),
            )?;
            format!("{} -> {}", template.template_name, guard_name)
        } else {
            format!("{} remains open", template.template_name)
        };

        conn.execute(
            "INSERT INTO decision_traces (id, cycle_id, shift_id, decision_type, summary, payload_json, decided_at)
             VALUES (?1, ?2, ?3, 'assignment_decision', ?4, ?5, ?6)",
            params![
                format!("trace-{shift_id}"),
                next_cycle_id,
                shift_id,
                summary,
                json!({
                    "template_name": template.template_name,
                    "winner_reason": winner_reason,
                    "steps": trace_steps
                })
                .to_string(),
                Utc::now().naive_utc().to_string()
            ],
        )
        .context("Failed to insert decision trace")?;
    }

    conn.execute(
        "UPDATE scheduling_cycles SET status = 'draft_ready' WHERE id = ?1",
        params![next_cycle_id],
    )
    .context("Failed to update scheduling cycle status to draft_ready")?;

    Ok(())
}

fn next_date_for_day(cycle_start: NaiveDate, day_of_week: &str) -> Result<NaiveDate> {
    let names = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"];
    let target = names
        .iter()
        .position(|name| *name == day_of_week)
        .ok_or_else(|| anyhow!("unknown day of week"))?;
    let start_idx = cycle_start.weekday().num_days_from_monday() as usize;
    let offset = (target + 7 - start_idx) % 7;
    Ok(cycle_start + chrono::Duration::days(offset as i64))
}

fn load_templates(conn: &Connection) -> Result<Vec<TemplateContext>> {
    let mut stmt = conn.prepare(
        "SELECT
            st.id,
            st.name,
            s.id,
            s.name,
            p.id,
            p.name,
            r.id,
            r.name,
            st.day_of_week,
            st.start_time,
            st.end_time,
            st.required_certifications,
            sh.id,
            g.id,
            g.name
         FROM shift_templates st
         JOIN sites s ON s.id = st.site_id
         JOIN pools p ON p.id = st.pool_id
         JOIN roles r ON r.id = st.role_id
         LEFT JOIN shifts sh ON sh.template_id = st.id AND sh.cycle_id = 'cycle-current'
         LEFT JOIN shift_assignments sa ON sa.shift_id = sh.id AND sa.status = 'assigned'
         LEFT JOIN guards g ON g.id = sa.guard_id
         WHERE st.active = 1
         ORDER BY st.day_of_week, st.start_time"
    )?;
    let rows = stmt.query_map([], |row| {
        let required: String = row.get(11)?;
        Ok(TemplateContext {
            template_id: row.get(0)?,
            template_name: row.get(1)?,
            site_id: row.get(2)?,
            site_name: row.get(3)?,
            pool_id: row.get(4)?,
            pool_name: row.get(5)?,
            role_id: row.get(6)?,
            role_name: row.get(7)?,
            day_of_week: row.get(8)?,
            start_time: row.get(9)?,
            end_time: row.get(10)?,
            required_certifications: serde_json::from_str(&required).unwrap_or_default(),
            current_shift_id: row.get(12)?,
            current_incumbent_guard_id: row.get(13)?,
            current_incumbent_name: row.get(14)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

fn load_requests_for_template(conn: &Connection, template_id: &str, cycle_id: &str) -> Result<Vec<CandidateRequest>> {
    let mut stmt = conn.prepare(
        "SELECT sr.id, g.id, g.name, sr.requested_at
         FROM shift_requests sr
         JOIN guards g ON g.id = sr.guard_id
         WHERE sr.shift_template_id = ?1 AND sr.cycle_id = ?2
         ORDER BY sr.requested_at ASC"
    )?;
    let rows = stmt.query_map(params![template_id, cycle_id], |row| {
        let requested_at: String = row.get(3)?;
        Ok(CandidateRequest {
            request_id: row.get(0)?,
            guard_id: row.get(1)?,
            guard_name: row.get(2)?,
            requested_at: chrono::DateTime::parse_from_rfc3339(
                &format!("{}Z", requested_at.replace(' ', "T")),
            )
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap(),
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

fn evaluate_eligibility(
    conn: &Connection,
    guard_id: &str,
    template: &TemplateContext,
    shift_date: &NaiveDate,
    new_shift_id: &str,
) -> Result<()> {
    let guard = load_guard_context(conn, guard_id)?;
    let existing = load_existing_assignments(conn, guard_id, new_shift_id)?;
    let policy_input = PolicyInput {
        guard,
        shift: ShiftContext {
            shift_id: new_shift_id.to_string(),
            site_id: template.site_id.clone(),
            role_id: template.role_id.clone(),
            shift_date: *shift_date,
            start_time: template.start_time.clone(),
            end_time: template.end_time.clone(),
            required_certifications: template.required_certifications.clone(),
        },
        existing_assignments: existing,
        policies: load_policy_config(conn)?,
    };
    evaluate_candidate(&policy_input).map_err(|err| anyhow!(err.reason))
}

fn load_guard_context(conn: &Connection, guard_id: &str) -> Result<GuardContext> {
    let (name, dob): (String, String) = conn.query_row(
        "SELECT name, date_of_birth FROM guards WHERE id = ?1",
        params![guard_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    let mut cert_stmt = conn.prepare(
        "SELECT c.name, gc.expires_on
         FROM guard_certifications gc
         JOIN certifications c ON c.id = gc.certification_id
         WHERE gc.guard_id = ?1"
    )?;
    let certs = cert_stmt
        .query_map(params![guard_id], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    Ok(GuardContext {
        guard_id: guard_id.to_string(),
        name,
        date_of_birth: NaiveDate::parse_from_str(&dob, "%Y-%m-%d")?,
        certifications: certs
            .into_iter()
            .map(|(name, expires_on)| {
                (
                    name,
                    NaiveDate::parse_from_str(&expires_on, "%Y-%m-%d")
                        .unwrap_or_else(|_| Utc::now().date_naive()),
                )
            })
            .collect(),
    })
}

fn load_existing_assignments(conn: &Connection, guard_id: &str, new_shift_id: &str) -> Result<Vec<ExistingAssignment>> {
    let mut stmt = conn.prepare(
        "SELECT sh.id, sh.shift_date, st.start_time, st.end_time
         FROM shift_assignments sa
         JOIN shifts sh ON sh.id = sa.shift_id
         JOIN shift_templates st ON st.id = sh.template_id
         WHERE sa.guard_id = ?1 AND sa.status IN ('assigned', 'draft') AND sh.id != ?2"
    )?;
    let rows = stmt.query_map(params![guard_id, new_shift_id], |row| {
        let shift_date: String = row.get(1)?;
        Ok(ExistingAssignment {
            shift_id: row.get(0)?,
            shift_date: NaiveDate::parse_from_str(&shift_date, "%Y-%m-%d").unwrap(),
            start_time: row.get(2)?,
            end_time: row.get(3)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

fn load_policy_config(conn: &Connection) -> Result<PolicyConfig> {
    let mut config = PolicyConfig::default();
    let mut stmt = conn.prepare("SELECT rule_type, config_json FROM policy_rules WHERE active = 1")?;
    let rows = stmt.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))?;
    for row in rows {
        let (rule_type, config_json) = row?;
        let value: serde_json::Value = serde_json::from_str(&config_json)?;
        match rule_type.as_str() {
            "minor_time_window" => {
                config.minor_max_age = value.get("max_age").and_then(|v| v.as_i64()).unwrap_or(17) as u8;
                config.minor_allowed_end_time = value
                    .get("allowed_end_time")
                    .and_then(|v| v.as_str())
                    .unwrap_or("20:00")
                    .to_string();
            }
            "max_daily_hours" => {
                config.max_daily_hours = value.get("max_daily_hours").and_then(|v| v.as_i64()).unwrap_or(8) as u8;
            }
            "max_weekly_hours" => {
                config.max_weekly_hours = value.get("max_weekly_hours").and_then(|v| v.as_i64()).unwrap_or(24) as u8;
            }
            "min_gap_hours" => {
                config.min_gap_hours = value.get("min_gap_hours").and_then(|v| v.as_i64()).unwrap_or(10) as u8;
            }
            _ => {}
        }
    }
    Ok(config)
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use rusqlite::Connection;

    use crate::seed::seed_demo;

    use super::generate_next_cycle_draft;

    #[test]
    fn draft_generation_assigns_expected_examples() -> Result<()> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(include_str!("../migrations/001_initial.sql"))?;
        seed_demo(&conn)?;

        generate_next_cycle_draft(&conn)?;

        let sat_open_guard: String = conn.query_row(
            "SELECT g.name
             FROM shift_assignments sa
             JOIN shifts sh ON sh.id = sa.shift_id
             JOIN guards g ON g.id = sa.guard_id
             WHERE sh.template_id = 'template-sat-open-main' AND sh.cycle_id = 'cycle-next'",
            [],
            |row| row.get(0),
        )?;
        assert_eq!(sat_open_guard, "Ben Torres");

        let tue_close_guard: String = conn.query_row(
            "SELECT g.name
             FROM shift_assignments sa
             JOIN shifts sh ON sh.id = sa.shift_id
             JOIN guards g ON g.id = sa.guard_id
             WHERE sh.template_id = 'template-tue-close-main' AND sh.cycle_id = 'cycle-next'",
            [],
            |row| row.get(0),
        )?;
        assert_eq!(tue_close_guard, "Olivia Chen");

        let noah_status: String = conn.query_row(
            "SELECT status || ':' || COALESCE(reason,'') FROM shift_requests WHERE id = 'request-noah-sat-open'",
            [],
            |row| row.get(0),
        )?;
        assert!(noah_status.contains("skipped"));
        assert!(noah_status.contains("certification"));

        Ok(())
    }
}
