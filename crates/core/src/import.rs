use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension};
use uuid::Uuid;

use lifebot_sling::mapping::{GuardImport, ShiftImport};

// ---------------------------------------------------------------------------
// Guards
// ---------------------------------------------------------------------------

/// Upsert guards from a Sling import.
///
/// For each guard:
/// - If `sling_id` already exists → UPDATE name/dob/phone/email.
/// - Otherwise → INSERT with id = `"guard-sling-{sling_id}"`.
///
/// Returns `(inserted, updated)`.
pub fn upsert_guards(conn: &Connection, guards: &[GuardImport]) -> Result<(usize, usize)> {
    let mut inserted = 0usize;
    let mut updated = 0usize;

    for g in guards {
        let existing_id: Option<String> = conn
            .query_row(
                "SELECT id FROM guards WHERE sling_id = ?1",
                params![g.sling_id],
                |row| row.get(0),
            )
            .optional()?;

        let dob = g.date_of_birth.map(|d| d.to_string());
        let phone = g.phone.as_deref().unwrap_or("");
        let email = g.email.as_deref().unwrap_or("");

        if let Some(_id) = existing_id {
            conn.execute(
                "UPDATE guards SET name = ?1, date_of_birth = ?2, phone = ?3, email = ?4 WHERE sling_id = ?5",
                params![g.name, dob.as_deref().unwrap_or(""), phone, email, g.sling_id],
            )?;
            updated += 1;
        } else {
            let new_id = format!("guard-sling-{}", g.sling_id);
            conn.execute(
                "INSERT INTO guards (id, name, date_of_birth, phone, email, notes, preferred_shifts, active, sling_id)
                 VALUES (?1, ?2, ?3, ?4, ?5, '', '', 1, ?6)",
                params![new_id, g.name, dob.as_deref().unwrap_or(""), phone, email, g.sling_id],
            )?;
            inserted += 1;
        }
    }

    Ok((inserted, updated))
}

// ---------------------------------------------------------------------------
// Sites
// ---------------------------------------------------------------------------

/// Upsert sites from Sling location groups.
///
/// `locations` is a slice of `(sling_id, name)` pairs.
/// Returns `(inserted, updated)`.
pub fn upsert_sites(conn: &Connection, locations: &[(i64, String)]) -> Result<(usize, usize)> {
    let mut inserted = 0usize;
    let mut updated = 0usize;

    for (sling_id, name) in locations {
        let existing_id: Option<String> = conn
            .query_row(
                "SELECT id FROM sites WHERE sling_id = ?1",
                params![sling_id],
                |row| row.get(0),
            )
            .optional()?;

        if existing_id.is_some() {
            conn.execute(
                "UPDATE sites SET name = ?1 WHERE sling_id = ?2",
                params![name, sling_id],
            )?;
            updated += 1;
        } else {
            let new_id = format!("site-sling-{}", sling_id);
            conn.execute(
                "INSERT INTO sites (id, name, region, sling_id) VALUES (?1, ?2, '', ?3)",
                params![new_id, name, sling_id],
            )?;
            inserted += 1;
        }
    }

    Ok((inserted, updated))
}

// ---------------------------------------------------------------------------
// Roles
// ---------------------------------------------------------------------------

/// Upsert roles from Sling position groups.
///
/// `positions` is a slice of `(sling_id, name)` pairs.
/// Returns `(inserted, updated)`.
pub fn upsert_roles(conn: &Connection, positions: &[(i64, String)]) -> Result<(usize, usize)> {
    let mut inserted = 0usize;
    let mut updated = 0usize;

    for (sling_id, name) in positions {
        let existing_id: Option<String> = conn
            .query_row(
                "SELECT id FROM roles WHERE sling_id = ?1",
                params![sling_id],
                |row| row.get(0),
            )
            .optional()?;

        if existing_id.is_some() {
            conn.execute(
                "UPDATE roles SET name = ?1 WHERE sling_id = ?2",
                params![name, sling_id],
            )?;
            updated += 1;
        } else {
            let new_id = format!("role-sling-{}", sling_id);
            conn.execute(
                "INSERT INTO roles (id, name, sling_id) VALUES (?1, ?2, ?3)",
                params![new_id, name, sling_id],
            )?;
            inserted += 1;
        }
    }

    Ok((inserted, updated))
}

// ---------------------------------------------------------------------------
// Shifts
// ---------------------------------------------------------------------------

/// Import shifts from a Sling import.
///
/// For each shift:
/// - Skip if `sling_shift_id` already exists in the shifts table.
/// - Find or create a matching shift_template.
/// - Insert the shift row.
/// - If the shift has an assigned user, create a shift_assignment.
///
/// Returns the count of newly imported shifts.
pub fn import_shifts(
    conn: &Connection,
    shifts: &[ShiftImport],
    cycle_id: &str,
) -> Result<usize> {
    let mut count = 0usize;

    for shift in shifts {
        // Skip duplicates.
        let exists: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM shifts WHERE sling_shift_id = ?1",
                params![shift.sling_shift_id],
                |row| row.get::<_, i64>(0),
            )
            .map(|n| n > 0)
            .unwrap_or(false);

        if exists {
            continue;
        }

        // Find or create the shift template.
        let template_id = match find_or_create_template(conn, shift) {
            Ok(id) => id,
            Err(_) => continue, // Skip if we can't resolve site/role.
        };

        // Insert the shift.
        let shift_id = format!("shift-sling-{}", shift.sling_shift_id);
        conn.execute(
            "INSERT INTO shifts (id, template_id, cycle_id, shift_date, status, sling_shift_id)
             VALUES (?1, ?2, ?3, ?4, 'open', ?5)",
            params![
                shift_id,
                template_id,
                cycle_id,
                shift.shift_date.to_string(),
                shift.sling_shift_id,
            ],
        )?;

        // Create shift assignment if a user is assigned.
        if let Some(user_sling_id) = shift.assigned_user_sling_id {
            let guard_id: Option<String> = conn
                .query_row(
                    "SELECT id FROM guards WHERE sling_id = ?1",
                    params![user_sling_id],
                    |row| row.get(0),
                )
                .optional()?;

            if let Some(guard_id) = guard_id {
                let assignment_id = Uuid::new_v4().to_string();
                conn.execute(
                    "INSERT INTO shift_assignments (id, shift_id, guard_id, status, assigned_at)
                     VALUES (?1, ?2, ?3, 'confirmed', datetime('now'))",
                    params![assignment_id, shift_id, guard_id],
                )?;
            }
        }

        count += 1;
    }

    Ok(count)
}

// ---------------------------------------------------------------------------
// Template helpers
// ---------------------------------------------------------------------------

/// Find an existing shift_template matching (day_of_week, start_time, end_time,
/// site_id, role_id), or create one if none exists.
///
/// Returns the template id, or an error if the site/role cannot be resolved.
fn find_or_create_template(conn: &Connection, shift: &ShiftImport) -> Result<String> {
    // Resolve location → site id.
    let site_id: Option<String> = if let Some(loc_id) = shift.location_sling_id {
        conn.query_row(
            "SELECT id FROM sites WHERE sling_id = ?1",
            params![loc_id],
            |row| row.get(0),
        )
        .optional()?
    } else {
        None
    };

    // Resolve position → role id.
    let role_id: Option<String> = if let Some(pos_id) = shift.position_sling_id {
        conn.query_row(
            "SELECT id FROM roles WHERE sling_id = ?1",
            params![pos_id],
            |row| row.get(0),
        )
        .optional()?
    } else {
        None
    };

    // Both must be resolvable for a meaningful template.
    let site_id = site_id.ok_or_else(|| {
        anyhow::anyhow!(
            "Cannot resolve location_sling_id {:?} to a site",
            shift.location_sling_id
        )
    })?;
    let role_id = role_id.ok_or_else(|| {
        anyhow::anyhow!(
            "Cannot resolve position_sling_id {:?} to a role",
            shift.position_sling_id
        )
    })?;

    // Look for an existing template.
    let existing: Option<String> = conn
        .query_row(
            "SELECT id FROM shift_templates
             WHERE day_of_week = ?1 AND start_time = ?2 AND end_time = ?3
               AND site_id = ?4 AND role_id = ?5
             LIMIT 1",
            params![
                shift.day_of_week,
                shift.start_time,
                shift.end_time,
                site_id,
                role_id,
            ],
            |row| row.get(0),
        )
        .optional()?;

    if let Some(id) = existing {
        return Ok(id);
    }

    // Create a new template. We need a pool — pick the first pool for this site.
    let pool_id: Option<String> = conn
        .query_row(
            "SELECT id FROM pools WHERE site_id = ?1 LIMIT 1",
            params![site_id],
            |row| row.get(0),
        )
        .optional()?;

    // If no pool exists, create a default one.
    let pool_id = if let Some(pid) = pool_id {
        pid
    } else {
        let new_pool_id = format!("pool-sling-{}", Uuid::new_v4());
        conn.execute(
            "INSERT INTO pools (id, site_id, name) VALUES (?1, ?2, 'Default Pool')",
            params![new_pool_id, site_id],
        )?;
        new_pool_id
    };

    let template_id = format!("tmpl-sling-{}", Uuid::new_v4());
    let template_name = format!(
        "{} {}-{}",
        shift.day_of_week, shift.start_time, shift.end_time
    );

    conn.execute(
        "INSERT INTO shift_templates
             (id, name, site_id, pool_id, role_id, day_of_week, start_time, end_time,
              required_certifications, active)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, '[]', 1)",
        params![
            template_id,
            template_name,
            site_id,
            pool_id,
            role_id,
            shift.day_of_week,
            shift.start_time,
            shift.end_time,
        ],
    )?;

    Ok(template_id)
}

// ---------------------------------------------------------------------------
// Import run log
// ---------------------------------------------------------------------------

/// Record an import run in the `import_runs` table.
///
/// Returns the `last_insert_rowid` for the new row.
pub fn record_import_run(
    conn: &Connection,
    guards_imported: usize,
    guards_updated: usize,
    sites_imported: usize,
    positions_imported: usize,
    shifts_imported: usize,
    errors: &[String],
) -> Result<i64> {
    let errors_json = if errors.is_empty() {
        None
    } else {
        Some(serde_json::to_string(errors)?)
    };

    conn.execute(
        "INSERT INTO import_runs
             (guards_imported, guards_updated, sites_imported, positions_imported,
              shifts_imported, errors_json, completed_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, datetime('now'))",
        params![
            guards_imported as i64,
            guards_updated as i64,
            sites_imported as i64,
            positions_imported as i64,
            shifts_imported as i64,
            errors_json,
        ],
    )?;

    Ok(conn.last_insert_rowid())
}
