use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Credentials & session
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlingCredentials {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlingSession {
    pub token: String,
    pub org_id: i64,
    pub user_name: String,
}

// ---------------------------------------------------------------------------
// Users
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SlingUser {
    pub id: i64,
    pub name: String,
    pub lastname: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub birthday_date: Option<String>,
    pub hours_cap: Option<f64>,
    #[serde(default)]
    pub deleted: bool,
    #[serde(default)]
    pub hidden_on_schedule: bool,
}

impl SlingUser {
    /// Parse `birthday_date` (expected format `YYYY-MM-DD`) into a `NaiveDate`.
    pub fn date_of_birth(&self) -> Option<NaiveDate> {
        let raw = self.birthday_date.as_deref()?;
        NaiveDate::parse_from_str(raw, "%Y-%m-%d").ok()
    }

    /// First name followed by last name when present.
    pub fn full_name(&self) -> String {
        match &self.lastname {
            Some(last) if !last.is_empty() => format!("{} {}", self.name, last),
            _ => self.name.clone(),
        }
    }
}

// ---------------------------------------------------------------------------
// Groups (locations / positions)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SlingGroup {
    pub id: i64,
    pub name: String,
    #[serde(rename = "type")]
    pub group_type: String,
    pub timezone: Option<String>,
    pub color: Option<String>,
    pub address: Option<String>,
}

impl SlingGroup {
    pub fn is_location(&self) -> bool {
        self.group_type == "location"
    }

    pub fn is_position(&self) -> bool {
        self.group_type == "position"
    }
}

// ---------------------------------------------------------------------------
// Shifts
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SlingShiftUser {
    pub id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SlingShiftRef {
    pub id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SlingShift {
    pub id: String,
    pub summary: Option<String>,
    pub dtstart: String,
    pub dtend: String,
    pub user: Option<SlingShiftUser>,
    pub location: Option<SlingShiftRef>,
    pub position: Option<SlingShiftRef>,
    pub status: Option<String>,
    pub break_duration: Option<i32>,
}

impl SlingShift {
    /// Parse `dtstart` as a UTC `DateTime`.
    pub fn start_datetime(&self) -> Option<DateTime<Utc>> {
        self.dtstart.parse::<DateTime<Utc>>().ok()
    }

    /// Parse `dtend` as a UTC `DateTime`.
    pub fn end_datetime(&self) -> Option<DateTime<Utc>> {
        self.dtend.parse::<DateTime<Utc>>().ok()
    }
}

/// Payload used when creating a new shift via the Sling API.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SlingShiftCreate {
    pub dtstart: String,
    pub dtend: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<SlingShiftUser>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<SlingShiftRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<SlingShiftRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
}

// ---------------------------------------------------------------------------
// Import result
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ImportResult {
    pub guards_imported: usize,
    pub guards_updated: usize,
    pub sites_imported: usize,
    pub positions_imported: usize,
    pub shifts_imported: usize,
    pub errors: Vec<String>,
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_user(name: &str, lastname: Option<&str>, birthday: Option<&str>) -> SlingUser {
        SlingUser {
            id: 1,
            name: name.to_string(),
            lastname: lastname.map(str::to_string),
            email: None,
            phone: None,
            birthday_date: birthday.map(str::to_string),
            hours_cap: None,
            deleted: false,
            hidden_on_schedule: false,
        }
    }

    #[test]
    fn full_name_with_last() {
        let u = make_user("Jane", Some("Doe"), None);
        assert_eq!(u.full_name(), "Jane Doe");
    }

    #[test]
    fn full_name_without_last() {
        let u = make_user("Jane", None, None);
        assert_eq!(u.full_name(), "Jane");
    }

    #[test]
    fn full_name_empty_last() {
        let u = make_user("Jane", Some(""), None);
        assert_eq!(u.full_name(), "Jane");
    }

    #[test]
    fn date_of_birth_valid() {
        let u = make_user("Jane", None, Some("1990-06-15"));
        let dob = u.date_of_birth().expect("should parse");
        assert_eq!(dob, NaiveDate::from_ymd_opt(1990, 6, 15).unwrap());
    }

    #[test]
    fn date_of_birth_none() {
        let u = make_user("Jane", None, None);
        assert!(u.date_of_birth().is_none());
    }

    #[test]
    fn date_of_birth_invalid() {
        let u = make_user("Jane", None, Some("not-a-date"));
        assert!(u.date_of_birth().is_none());
    }

    fn make_group(group_type: &str) -> SlingGroup {
        SlingGroup {
            id: 1,
            name: "Test Group".to_string(),
            group_type: group_type.to_string(),
            timezone: None,
            color: None,
            address: None,
        }
    }

    #[test]
    fn is_location_true() {
        assert!(make_group("location").is_location());
    }

    #[test]
    fn is_location_false() {
        assert!(!make_group("position").is_location());
    }

    #[test]
    fn is_position_true() {
        assert!(make_group("position").is_position());
    }

    #[test]
    fn is_position_false() {
        assert!(!make_group("location").is_position());
    }

    fn make_shift(dtstart: &str, dtend: &str) -> SlingShift {
        SlingShift {
            id: "shift-1".to_string(),
            summary: None,
            dtstart: dtstart.to_string(),
            dtend: dtend.to_string(),
            user: None,
            location: None,
            position: None,
            status: None,
            break_duration: None,
        }
    }

    #[test]
    fn start_datetime_valid() {
        let s = make_shift("2024-07-04T08:00:00Z", "2024-07-04T16:00:00Z");
        let dt = s.start_datetime().expect("should parse");
        assert_eq!(dt.to_rfc3339(), "2024-07-04T08:00:00+00:00");
    }

    #[test]
    fn end_datetime_valid() {
        let s = make_shift("2024-07-04T08:00:00Z", "2024-07-04T16:00:00Z");
        let dt = s.end_datetime().expect("should parse");
        assert_eq!(dt.to_rfc3339(), "2024-07-04T16:00:00+00:00");
    }

    #[test]
    fn start_datetime_invalid() {
        let s = make_shift("not-a-datetime", "2024-07-04T16:00:00Z");
        assert!(s.start_datetime().is_none());
    }

    #[test]
    fn end_datetime_invalid() {
        let s = make_shift("2024-07-04T08:00:00Z", "bad");
        assert!(s.end_datetime().is_none());
    }
}
