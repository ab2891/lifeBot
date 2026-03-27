use lifebot_core::{db::migrate_conn, scheduling::generate_next_cycle_draft};
use rusqlite::Connection;

/// Create a migrated in-memory connection with foreign keys enabled.
fn in_memory_conn() -> Connection {
    let conn = Connection::open_in_memory().expect("open in-memory DB");
    conn.pragma_update(None, "foreign_keys", "ON")
        .expect("enable FK");
    migrate_conn(&conn).expect("migrate failed");
    conn
}

/// Insert the minimal rows needed for most tests: 1 site, 1 pool, 1 role, 1 cert,
/// 1 guard with that cert, and 1 active template pointing at them.
/// Returns (site_id, pool_id, role_id, cert_id, guard_id, template_id).
fn insert_base_rows(conn: &Connection) -> (&'static str, &'static str, &'static str, &'static str, &'static str, &'static str) {
    conn.execute_batch(
        "INSERT INTO sites (id, name, region) VALUES ('site-1', 'Main Site', 'north');
         INSERT INTO pools (id, site_id, name) VALUES ('pool-1', 'site-1', 'Main Pool');
         INSERT INTO roles (id, name) VALUES ('role-1', 'Lifeguard');
         INSERT INTO certifications (id, name) VALUES ('cert-1', 'CPR');
         INSERT INTO guards (id, name, date_of_birth, phone, email, notes, preferred_shifts)
             VALUES ('guard-1', 'Alice Edge', '1990-01-01', '555-0000', 'alice@example.com', '', '');
         INSERT INTO guard_certifications (guard_id, certification_id, expires_on)
             VALUES ('guard-1', 'cert-1', '2030-01-01');",
    )
    .expect("insert base rows");
    ("site-1", "pool-1", "role-1", "cert-1", "guard-1", "template-1")
}

/// Insert an active shift template.
fn insert_template(conn: &Connection, active: i64) {
    conn.execute(
        "INSERT INTO shift_templates (id, name, site_id, pool_id, role_id, day_of_week, start_time, end_time, required_certifications, active)
         VALUES ('template-1', 'Monday Morning', 'site-1', 'pool-1', 'role-1', 'Monday', '08:00', '16:00', '[\"CPR\"]', ?1)",
        rusqlite::params![active],
    )
    .expect("insert template");
}

/// Insert a draft scheduling cycle starting on the coming Monday.
fn insert_draft_cycle(conn: &Connection) {
    conn.execute_batch(
        "INSERT INTO scheduling_cycles (id, name, starts_on, ends_on, rollover_deadline, status)
         VALUES ('cycle-draft', 'Test Draft Cycle', '2026-03-30', '2026-04-05', '2026-03-28 17:00:00', 'draft');",
    )
    .expect("insert draft cycle");
}

/// Insert an active (non-draft) cycle only — used to test the missing-draft error.
fn insert_active_cycle_only(conn: &Connection) {
    conn.execute_batch(
        "INSERT INTO scheduling_cycles (id, name, starts_on, ends_on, rollover_deadline, status)
         VALUES ('cycle-active', 'Active Cycle', '2026-03-23', '2026-03-29', '2026-03-21 17:00:00', 'active');",
    )
    .expect("insert active cycle");
}

// ─── Test 1 ───────────────────────────────────────────────────────────────────

/// When there are no shift requests and no incumbent, the engine should still
/// create the shift but leave it unassigned, and record a decision trace with
/// the "no_requests" step.
#[test]
fn draft_with_no_requests_creates_unassigned_shift() {
    let conn = in_memory_conn();
    insert_base_rows(&conn);
    insert_template(&conn, 1);
    insert_draft_cycle(&conn);

    generate_next_cycle_draft(&conn).expect("draft generation should succeed");

    // A shift must exist for the template in the draft cycle.
    let shift_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM shifts WHERE template_id = 'template-1' AND cycle_id = 'cycle-draft'",
            [],
            |row| row.get(0),
        )
        .expect("count shifts");
    assert_eq!(shift_count, 1, "expected one shift to be created");

    // No assignment should exist.
    let assignment_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM shift_assignments sa
             JOIN shifts sh ON sh.id = sa.shift_id
             WHERE sh.template_id = 'template-1' AND sh.cycle_id = 'cycle-draft'",
            [],
            |row| row.get(0),
        )
        .expect("count assignments");
    assert_eq!(assignment_count, 0, "expected no assignment for an unassigned shift");

    // A decision trace must have been recorded for the cycle.
    let trace_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM decision_traces WHERE cycle_id = 'cycle-draft'",
            [],
            |row| row.get(0),
        )
        .expect("count traces");
    assert_eq!(trace_count, 1, "expected one decision trace");

    // The trace payload should contain the no_requests step.
    let payload: String = conn
        .query_row(
            "SELECT payload_json FROM decision_traces WHERE cycle_id = 'cycle-draft'",
            [],
            |row| row.get(0),
        )
        .expect("fetch trace payload");
    assert!(
        payload.contains("no_requests"),
        "expected 'no_requests' step in trace payload, got: {payload}"
    );
    assert!(
        payload.contains("No requests received for this shift"),
        "expected human-readable reason in trace payload, got: {payload}"
    );
}

// ─── Test 2 ───────────────────────────────────────────────────────────────────

/// Templates with active = 0 should be skipped — no shifts should be created.
#[test]
fn draft_skips_inactive_templates() {
    let conn = in_memory_conn();
    insert_base_rows(&conn);
    insert_template(&conn, 0); // inactive
    insert_draft_cycle(&conn);

    generate_next_cycle_draft(&conn).expect("draft generation should succeed");

    let shift_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM shifts WHERE cycle_id = 'cycle-draft'",
            [],
            |row| row.get(0),
        )
        .expect("count shifts");
    assert_eq!(shift_count, 0, "expected no shifts for inactive template");
}

// ─── Test 3 ───────────────────────────────────────────────────────────────────

/// When there is no scheduling cycle with status = 'draft', the function must
/// return an error rather than panicking or silently producing an empty result.
#[test]
fn draft_fails_without_draft_cycle() {
    let conn = in_memory_conn();
    insert_base_rows(&conn);
    insert_template(&conn, 1);
    insert_active_cycle_only(&conn); // no 'draft' cycle

    let result = generate_next_cycle_draft(&conn);
    assert!(
        result.is_err(),
        "expected an error when no draft cycle exists, got Ok"
    );
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("No draft scheduling cycle found"),
        "expected informative error message, got: {err_msg}"
    );
}
