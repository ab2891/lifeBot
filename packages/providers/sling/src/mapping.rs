use chrono::{NaiveDate, Timelike, Datelike, Weekday};

use crate::types::{SlingGroup, SlingShift, SlingUser};

// ---------------------------------------------------------------------------
// Intermediate structs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct GuardImport {
    pub sling_id: i64,
    pub name: String,
    pub date_of_birth: Option<NaiveDate>,
    pub phone: Option<String>,
    pub email: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ShiftImport {
    pub sling_shift_id: String,
    pub summary: Option<String>,
    pub start_time: String,        // "HH:MM"
    pub end_time: String,          // "HH:MM"
    pub shift_date: NaiveDate,
    pub day_of_week: String,
    pub location_sling_id: Option<i64>,
    pub position_sling_id: Option<i64>,
    pub assigned_user_sling_id: Option<i64>,
}

// ---------------------------------------------------------------------------
// Mapping functions
// ---------------------------------------------------------------------------

/// Map a `SlingUser` to a `GuardImport`.
pub fn map_user_to_guard(user: &SlingUser) -> GuardImport {
    GuardImport {
        sling_id: user.id,
        name: user.full_name(),
        date_of_birth: user.date_of_birth(),
        phone: user.phone.clone(),
        email: user.email.clone(),
    }
}

/// Split a slice of `SlingGroup` into (locations, positions).
pub fn split_groups<'a>(
    groups: &'a [SlingGroup],
) -> (Vec<&'a SlingGroup>, Vec<&'a SlingGroup>) {
    let locations = groups.iter().filter(|g| g.is_location()).collect();
    let positions = groups.iter().filter(|g| g.is_position()).collect();
    (locations, positions)
}

/// Map a `SlingShift` to a `ShiftImport`.
///
/// Returns `None` when `dtstart` or `dtend` cannot be parsed as RFC 3339 /
/// ISO 8601 UTC datetimes.
pub fn map_shift(shift: &SlingShift) -> Option<ShiftImport> {
    let start = shift.start_datetime()?;
    let end = shift.end_datetime()?;

    let shift_date = start.date_naive();

    let start_time = format!("{:02}:{:02}", start.hour(), start.minute());
    let end_time = format!("{:02}:{:02}", end.hour(), end.minute());

    let day_of_week = match shift_date.weekday() {
        Weekday::Mon => "Monday",
        Weekday::Tue => "Tuesday",
        Weekday::Wed => "Wednesday",
        Weekday::Thu => "Thursday",
        Weekday::Fri => "Friday",
        Weekday::Sat => "Saturday",
        Weekday::Sun => "Sunday",
    }
    .to_string();

    Some(ShiftImport {
        sling_shift_id: shift.id.clone(),
        summary: shift.summary.clone(),
        start_time,
        end_time,
        shift_date,
        day_of_week,
        location_sling_id: shift.location.as_ref().map(|r| r.id),
        position_sling_id: shift.position.as_ref().map(|r| r.id),
        assigned_user_sling_id: shift.user.as_ref().map(|u| u.id),
    })
}
