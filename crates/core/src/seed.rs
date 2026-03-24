use anyhow::Result;
use chrono::{Duration, NaiveDate, NaiveDateTime, Utc};
use rusqlite::{params, Connection};

use crate::models::SchedulingCycleRecord;

pub fn seed_demo(conn: &Connection) -> Result<()> {
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM guards", [], |row| row.get(0))?;
    if count > 0 {
        return Ok(());
    }

    insert_roles(conn)?;
    insert_sites_and_pools(conn)?;
    insert_certifications(conn)?;
    insert_guards(conn)?;
    insert_guard_roles_and_certs(conn)?;
    insert_cycles_and_templates(conn)?;
    insert_current_cycle_assignments(conn)?;
    insert_requests_and_rollovers(conn)?;
    insert_policy_rules(conn)?;
    insert_provider_state(conn)?;

    Ok(())
}

fn insert_roles(conn: &Connection) -> Result<()> {
    for (id, name) in [
        ("role-lifeguard", "Lifeguard"),
        ("role-deck-supervisor", "Deck Supervisor"),
        ("role-swim-instructor", "Swim Instructor"),
    ] {
        conn.execute(
            "INSERT INTO roles (id, name) VALUES (?1, ?2)",
            params![id, name],
        )?;
    }
    Ok(())
}

fn insert_sites_and_pools(conn: &Connection) -> Result<()> {
    for (site_id, site_name) in [
        ("site-main", "Downtown YMCA"),
        ("site-north", "Northside YMCA"),
    ] {
        conn.execute(
            "INSERT INTO sites (id, name, region) VALUES (?1, ?2, 'Metro')",
            params![site_id, site_name],
        )?;
    }

    for (pool_id, site_id, pool_name) in [
        ("pool-main-competition", "site-main", "Main Pool"),
        ("pool-main-rec", "site-main", "Rec Pool"),
        ("pool-north", "site-north", "Northside Pool"),
    ] {
        conn.execute(
            "INSERT INTO pools (id, site_id, name) VALUES (?1, ?2, ?3)",
            params![pool_id, site_id, pool_name],
        )?;
    }
    Ok(())
}

fn insert_certifications(conn: &Connection) -> Result<()> {
    for (id, name) in [
        ("cert-lifeguard", "Lifeguard"),
        ("cert-cpr", "CPR/AED"),
        ("cert-waterfront", "Waterfront"),
        ("cert-instructor", "Swim Instruction"),
    ] {
        conn.execute(
            "INSERT INTO certifications (id, name) VALUES (?1, ?2)",
            params![id, name],
        )?;
    }
    Ok(())
}

fn insert_guards(conn: &Connection) -> Result<()> {
    let guards = vec![
        ("guard-olivia", "Olivia Chen", "2001-05-14", "555-0101", "olivia@lifebot.local", "Prefers Tuesday and Thursday closes", "Tuesday close, Thursday close"),
        ("guard-marcus", "Marcus Hill", "2000-08-22", "555-0102", "marcus@lifebot.local", "Often covers weekend opens", "Saturday morning"),
        ("guard-ben", "Ben Torres", "1998-10-10", "555-0103", "ben@lifebot.local", "Reliable for weekend opening shifts", "Saturday morning"),
        ("guard-noah", "Noah Patel", "1999-02-01", "555-0104", "noah@lifebot.local", "Needs waterfront renewal", "Weekend day shifts"),
        ("guard-jada", "Jada Brooks", "2009-07-11", "555-0105", "jada@lifebot.local", "Minor guard, no late closes", "After school, daytime weekends"),
        ("guard-emma", "Emma Rivera", "2007-03-19", "555-0106", "emma@lifebot.local", "Strong swim lesson coverage", "Afternoons"),
        ("guard-lucas", "Lucas Green", "1997-09-30", "555-0107", "lucas@lifebot.local", "Deck supervisor rotation", "Mornings"),
        ("guard-sophia", "Sophia Reed", "2002-11-17", "555-0108", "sophia@lifebot.local", "Rec pool closer", "Evenings"),
        ("guard-liam", "Liam Foster", "1995-01-05", "555-0109", "liam@lifebot.local", "Northside lead", "Weekdays"),
        ("guard-maya", "Maya Campbell", "2003-06-28", "555-0110", "maya@lifebot.local", "Likes recurring lesson blocks", "Wednesday and Sunday"),
        ("guard-ethan", "Ethan Wright", "2004-04-09", "555-0111", "ethan@lifebot.local", "Flexible coverage", "Open availability"),
        ("guard-zoe", "Zoe Parker", "2008-01-26", "555-0112", "zoe@lifebot.local", "Minor, avoid late nights", "Weekends"),
        ("guard-amelia", "Amelia Diaz", "1996-12-14", "555-0113", "amelia@lifebot.local", "Deck supervisor and instructor", "Mornings"),
        ("guard-jacob", "Jacob Long", "1994-03-03", "555-0114", "jacob@lifebot.local", "Main pool opener", "Early mornings"),
        ("guard-harper", "Harper Scott", "2005-08-16", "555-0115", "harper@lifebot.local", "Weekend relief", "Saturday afternoons"),
        ("guard-aiden", "Aiden Cruz", "2006-09-12", "555-0116", "aiden@lifebot.local", "Junior instructor", "Afternoons"),
        ("guard-evelyn", "Evelyn Perry", "1993-05-08", "555-0117", "evelyn@lifebot.local", "Northside closer", "Evenings"),
        ("guard-daniel", "Daniel Kim", "1991-07-20", "555-0118", "daniel@lifebot.local", "Weekday deck supervisor", "Weekdays"),
        ("guard-abigail", "Abigail Lee", "2002-02-27", "555-0119", "abigail@lifebot.local", "CPR trainer", "Mornings"),
        ("guard-mason", "Mason Bell", "2004-12-01", "555-0120", "mason@lifebot.local", "Pool setup and opening", "Morning opens"),
        ("guard-ella", "Ella Ross", "2007-10-04", "555-0121", "ella@lifebot.local", "Minor with lesson availability", "After school"),
        ("guard-henry", "Henry Price", "1990-06-15", "555-0122", "henry@lifebot.local", "Senior supervisor", "Weekday closes"),
    ];

    for (id, name, dob, phone, email, notes, preferred_shifts) in guards {
        conn.execute(
            "INSERT INTO guards (id, name, date_of_birth, phone, email, notes, preferred_shifts, active)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1)",
            params![id, name, dob, phone, email, notes, preferred_shifts],
        )?;
    }
    Ok(())
}

fn insert_guard_roles_and_certs(conn: &Connection) -> Result<()> {
    let today = Utc::now().date_naive();
    let expiring_soon = today + Duration::days(14);
    let healthy = today + Duration::days(180);
    let expired = today - Duration::days(5);

    let role_map = vec![
        ("guard-olivia", vec!["role-lifeguard"]),
        ("guard-marcus", vec!["role-lifeguard"]),
        ("guard-ben", vec!["role-lifeguard"]),
        ("guard-noah", vec!["role-lifeguard"]),
        ("guard-jada", vec!["role-lifeguard"]),
        ("guard-emma", vec!["role-lifeguard", "role-swim-instructor"]),
        ("guard-lucas", vec!["role-lifeguard", "role-deck-supervisor"]),
        ("guard-sophia", vec!["role-lifeguard"]),
        ("guard-liam", vec!["role-lifeguard", "role-deck-supervisor"]),
        ("guard-maya", vec!["role-lifeguard", "role-swim-instructor"]),
        ("guard-ethan", vec!["role-lifeguard"]),
        ("guard-zoe", vec!["role-lifeguard"]),
        ("guard-amelia", vec!["role-lifeguard", "role-swim-instructor", "role-deck-supervisor"]),
        ("guard-jacob", vec!["role-lifeguard"]),
        ("guard-harper", vec!["role-lifeguard"]),
        ("guard-aiden", vec!["role-swim-instructor", "role-lifeguard"]),
        ("guard-evelyn", vec!["role-lifeguard"]),
        ("guard-daniel", vec!["role-deck-supervisor", "role-lifeguard"]),
        ("guard-abigail", vec!["role-lifeguard"]),
        ("guard-mason", vec!["role-lifeguard"]),
        ("guard-ella", vec!["role-swim-instructor", "role-lifeguard"]),
        ("guard-henry", vec!["role-deck-supervisor", "role-lifeguard"]),
    ];

    for (guard_id, roles) in role_map {
        for role_id in roles {
            conn.execute(
                "INSERT INTO guard_roles (guard_id, role_id) VALUES (?1, ?2)",
                params![guard_id, role_id],
            )?;
        }
    }

    let guards = [
        "guard-olivia","guard-marcus","guard-ben","guard-noah","guard-jada","guard-emma","guard-lucas",
        "guard-sophia","guard-liam","guard-maya","guard-ethan","guard-zoe","guard-amelia","guard-jacob",
        "guard-harper","guard-aiden","guard-evelyn","guard-daniel","guard-abigail","guard-mason","guard-ella","guard-henry"
    ];
    for guard_id in guards {
        conn.execute(
            "INSERT INTO guard_certifications (guard_id, certification_id, expires_on)
             VALUES (?1, 'cert-lifeguard', ?2)",
            params![guard_id, healthy.to_string()],
        )?;
        conn.execute(
            "INSERT INTO guard_certifications (guard_id, certification_id, expires_on)
             VALUES (?1, 'cert-cpr', ?2)",
            params![guard_id, healthy.to_string()],
        )?;
    }

    for guard_id in ["guard-emma", "guard-maya", "guard-aiden", "guard-amelia", "guard-ella"] {
        conn.execute(
            "INSERT INTO guard_certifications (guard_id, certification_id, expires_on)
             VALUES (?1, 'cert-instructor', ?2)",
            params![guard_id, healthy.to_string()],
        )?;
    }

    for guard_id in ["guard-olivia", "guard-marcus", "guard-ben", "guard-lucas", "guard-liam", "guard-henry"] {
        conn.execute(
            "INSERT INTO guard_certifications (guard_id, certification_id, expires_on)
             VALUES (?1, 'cert-waterfront', ?2)",
            params![guard_id, healthy.to_string()],
        )?;
    }

    conn.execute(
        "UPDATE guard_certifications SET expires_on = ?1 WHERE guard_id = 'guard-noah' AND certification_id = 'cert-waterfront'",
        params![expired.to_string()],
    )?;
    conn.execute(
        "UPDATE guard_certifications SET expires_on = ?1 WHERE guard_id = 'guard-emma' AND certification_id = 'cert-cpr'",
        params![expiring_soon.to_string()],
    )?;
    conn.execute(
        "UPDATE guard_certifications SET expires_on = ?1 WHERE guard_id = 'guard-jada' AND certification_id = 'cert-cpr'",
        params![expiring_soon.to_string()],
    )?;

    Ok(())
}

fn cycle_record(id: &str, name: &str, starts_on: &str, ends_on: &str, deadline: &str, status: &str) -> SchedulingCycleRecord {
    SchedulingCycleRecord {
        id: id.to_string(),
        name: name.to_string(),
        starts_on: NaiveDate::parse_from_str(starts_on, "%Y-%m-%d").unwrap(),
        ends_on: NaiveDate::parse_from_str(ends_on, "%Y-%m-%d").unwrap(),
        rollover_deadline: NaiveDateTime::parse_from_str(deadline, "%Y-%m-%d %H:%M:%S").unwrap(),
        status: status.to_string(),
    }
}

fn insert_cycles_and_templates(conn: &Connection) -> Result<()> {
    let cycles = vec![
        cycle_record("cycle-current", "Current Cycle", "2026-03-16", "2026-03-22", "2026-03-19 17:00:00", "active"),
        cycle_record("cycle-next", "Next Cycle", "2026-03-23", "2026-03-29", "2026-03-22 17:00:00", "draft"),
    ];
    for cycle in cycles {
        conn.execute(
            "INSERT INTO scheduling_cycles (id, name, starts_on, ends_on, rollover_deadline, status)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                cycle.id,
                cycle.name,
                cycle.starts_on.to_string(),
                cycle.ends_on.to_string(),
                cycle.rollover_deadline.to_string(),
                cycle.status
            ],
        )?;
    }

    let templates = vec![
        ("template-tue-close-main", "Tuesday Close - Main Pool", "site-main", "pool-main-competition", "role-lifeguard", "Tuesday", "18:00", "22:00", "[\"cert-lifeguard\",\"cert-cpr\",\"cert-waterfront\"]"),
        ("template-sat-open-main", "Saturday 8am - Main Pool", "site-main", "pool-main-competition", "role-lifeguard", "Saturday", "08:00", "12:00", "[\"cert-lifeguard\",\"cert-cpr\",\"cert-waterfront\"]"),
        ("template-wed-lessons", "Wednesday Lessons - Rec Pool", "site-main", "pool-main-rec", "role-swim-instructor", "Wednesday", "16:00", "20:00", "[\"cert-lifeguard\",\"cert-cpr\",\"cert-instructor\"]"),
        ("template-fri-close-rec", "Friday Close - Rec Pool", "site-main", "pool-main-rec", "role-lifeguard", "Friday", "17:00", "21:00", "[\"cert-lifeguard\",\"cert-cpr\"]"),
        ("template-sun-open-north", "Sunday Open - Northside", "site-north", "pool-north", "role-lifeguard", "Sunday", "09:00", "13:00", "[\"cert-lifeguard\",\"cert-cpr\"]"),
        ("template-mon-supervisor", "Monday Deck Supervisor", "site-main", "pool-main-competition", "role-deck-supervisor", "Monday", "05:30", "10:30", "[\"cert-lifeguard\",\"cert-cpr\"]"),
        ("template-thu-close-main", "Thursday Close - Main Pool", "site-main", "pool-main-competition", "role-lifeguard", "Thursday", "18:00", "22:00", "[\"cert-lifeguard\",\"cert-cpr\",\"cert-waterfront\"]"),
        ("template-sat-lessons", "Saturday Lessons - Rec Pool", "site-main", "pool-main-rec", "role-swim-instructor", "Saturday", "09:00", "12:00", "[\"cert-lifeguard\",\"cert-cpr\",\"cert-instructor\"]"),
    ];
    for (id, name, site_id, pool_id, role_id, day_of_week, start_time, end_time, required_certifications) in templates {
        conn.execute(
            "INSERT INTO shift_templates (id, name, site_id, pool_id, role_id, day_of_week, start_time, end_time, required_certifications, active)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 1)",
            params![id, name, site_id, pool_id, role_id, day_of_week, start_time, end_time, required_certifications],
        )?;
    }
    Ok(())
}

fn insert_current_cycle_assignments(conn: &Connection) -> Result<()> {
    let shifts = vec![
        ("shift-current-tue-close-main", "template-tue-close-main", "cycle-current", "2026-03-17"),
        ("shift-current-sat-open-main", "template-sat-open-main", "cycle-current", "2026-03-21"),
        ("shift-current-wed-lessons", "template-wed-lessons", "cycle-current", "2026-03-18"),
        ("shift-current-fri-close-rec", "template-fri-close-rec", "cycle-current", "2026-03-20"),
        ("shift-current-sun-open-north", "template-sun-open-north", "cycle-current", "2026-03-22"),
        ("shift-current-mon-supervisor", "template-mon-supervisor", "cycle-current", "2026-03-16"),
        ("shift-current-thu-close-main", "template-thu-close-main", "cycle-current", "2026-03-19"),
        ("shift-current-sat-lessons", "template-sat-lessons", "cycle-current", "2026-03-21"),
    ];

    for (id, template_id, cycle_id, shift_date) in shifts {
        conn.execute(
            "INSERT INTO shifts (id, template_id, cycle_id, shift_date, status) VALUES (?1, ?2, ?3, ?4, 'published')",
            params![id, template_id, cycle_id, shift_date],
        )?;
    }

    for (shift_id, guard_id) in [
        ("shift-current-tue-close-main", "guard-olivia"),
        ("shift-current-sat-open-main", "guard-marcus"),
        ("shift-current-wed-lessons", "guard-emma"),
        ("shift-current-fri-close-rec", "guard-sophia"),
        ("shift-current-sun-open-north", "guard-liam"),
        ("shift-current-mon-supervisor", "guard-lucas"),
        ("shift-current-thu-close-main", "guard-henry"),
        ("shift-current-sat-lessons", "guard-maya"),
    ] {
        conn.execute(
            "INSERT INTO shift_assignments (id, shift_id, guard_id, status, assigned_at)
             VALUES (?1, ?2, ?3, 'assigned', '2026-03-10 09:00:00')",
            params![format!("assignment-{shift_id}"), shift_id, guard_id],
        )?;
    }
    Ok(())
}

fn insert_requests_and_rollovers(conn: &Connection) -> Result<()> {
    conn.execute(
        "INSERT INTO rollover_requests (id, shift_template_id, cycle_id, guard_id, requested_at, status)
         VALUES ('rollover-olivia', 'template-tue-close-main', 'cycle-current', 'guard-olivia', '2026-03-18 09:15:00', 'requested')",
        [],
    )?;
    conn.execute(
        "INSERT INTO rollover_requests (id, shift_template_id, cycle_id, guard_id, requested_at, status)
         VALUES ('rollover-emma', 'template-wed-lessons', 'cycle-current', 'guard-emma', '2026-03-18 08:00:00', 'requested')",
        [],
    )?;
    conn.execute(
        "INSERT INTO rollover_requests (id, shift_template_id, cycle_id, guard_id, requested_at, status)
         VALUES ('rollover-sophia', 'template-fri-close-rec', 'cycle-current', 'guard-sophia', '2026-03-18 14:00:00', 'requested')",
        [],
    )?;

    let requests = vec![
        ("request-ben-sat-open", "template-sat-open-main", "guard-ben", "2026-03-19 09:00:00"),
        ("request-noah-sat-open", "template-sat-open-main", "guard-noah", "2026-03-19 08:30:00"),
        ("request-jada-tue-close", "template-tue-close-main", "guard-jada", "2026-03-19 10:00:00"),
        ("request-harper-fri-close", "template-fri-close-rec", "guard-harper", "2026-03-19 08:45:00"),
        ("request-zoe-fri-close", "template-fri-close-rec", "guard-zoe", "2026-03-19 08:15:00"),
        ("request-amelia-mon-supervisor", "template-mon-supervisor", "guard-amelia", "2026-03-19 07:30:00"),
        ("request-daniel-mon-supervisor", "template-mon-supervisor", "guard-daniel", "2026-03-19 07:45:00"),
        ("request-aiden-sat-lessons", "template-sat-lessons", "guard-aiden", "2026-03-19 11:00:00"),
        ("request-ella-sat-lessons", "template-sat-lessons", "guard-ella", "2026-03-19 10:30:00"),
        ("request-jacob-thu-close", "template-thu-close-main", "guard-jacob", "2026-03-19 09:45:00"),
    ];

    for (id, template_id, guard_id, requested_at) in requests {
        conn.execute(
            "INSERT INTO shift_requests (id, shift_template_id, cycle_id, guard_id, requested_at, status)
             VALUES (?1, ?2, 'cycle-next', ?3, ?4, 'queued')",
            params![id, template_id, guard_id, requested_at],
        )?;
    }
    Ok(())
}

fn insert_policy_rules(conn: &Connection) -> Result<()> {
    let rules = vec![
        (
            "policy-minor-night-cutoff",
            "minor_time_window",
            "Minor guards may not work past 20:00.",
            "{\"max_age\":17,\"allowed_end_time\":\"20:00\"}"
        ),
        (
            "policy-max-daily-hours",
            "max_daily_hours",
            "Guards may work at most 8 hours in a day.",
            "{\"max_daily_hours\":8}"
        ),
        (
            "policy-max-weekly-hours",
            "max_weekly_hours",
            "Guards may work at most 24 hours in a scheduling cycle.",
            "{\"max_weekly_hours\":24}"
        ),
        (
            "policy-min-gap",
            "min_gap_hours",
            "Guards need a 10 hour gap between shifts.",
            "{\"min_gap_hours\":10}"
        ),
    ];
    for (id, rule_type, description, config_json) in rules {
        conn.execute(
            "INSERT INTO policy_rules (id, rule_type, description, config_json, active)
             VALUES (?1, ?2, ?3, ?4, 1)",
            params![id, rule_type, description, config_json],
        )?;
    }
    Ok(())
}

fn insert_provider_state(conn: &Connection) -> Result<()> {
    conn.execute(
        "INSERT INTO provider_sync_state (id, provider_name, status, last_synced_at, details_json)
         VALUES ('sync-mock-sling', 'sling-mock', 'ready', '2026-03-19 06:00:00', '{\"mode\":\"fixture\"}')",
        [],
    )?;
    Ok(())
}
