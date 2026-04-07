use chrono::NaiveDate;
use lifebot_sling::{
    mapping::{map_shift, map_user_to_guard, split_groups},
    SlingGroup, SlingShift, SlingShiftRef, SlingShiftUser, SlingUser,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_user(
    id: i64,
    name: &str,
    lastname: Option<&str>,
    email: Option<&str>,
    phone: Option<&str>,
    birthday: Option<&str>,
) -> SlingUser {
    SlingUser {
        id,
        name: name.to_string(),
        lastname: lastname.map(str::to_string),
        email: email.map(str::to_string),
        phone: phone.map(str::to_string),
        birthday_date: birthday.map(str::to_string),
        hours_cap: None,
        deleted: false,
        hidden_on_schedule: false,
    }
}

fn make_group(id: i64, group_type: &str) -> SlingGroup {
    SlingGroup {
        id,
        name: format!("Group {id}"),
        group_type: group_type.to_string(),
        timezone: None,
        color: None,
        address: None,
    }
}

fn make_shift(
    id: &str,
    dtstart: &str,
    dtend: &str,
    user_id: Option<i64>,
    location_id: Option<i64>,
    position_id: Option<i64>,
) -> SlingShift {
    SlingShift {
        id: id.to_string(),
        summary: None,
        dtstart: dtstart.to_string(),
        dtend: dtend.to_string(),
        user: user_id.map(|id| SlingShiftUser { id }),
        location: location_id.map(|id| SlingShiftRef { id }),
        position: position_id.map(|id| SlingShiftRef { id }),
        status: None,
        break_duration: None,
    }
}

// ---------------------------------------------------------------------------
// map_user_to_guard
// ---------------------------------------------------------------------------

#[test]
fn guard_name_full() {
    let u = make_user(42, "Jane", Some("Doe"), None, None, None);
    let g = map_user_to_guard(&u);
    assert_eq!(g.name, "Jane Doe");
    assert_eq!(g.sling_id, 42);
}

#[test]
fn guard_name_first_only() {
    let u = make_user(7, "Alice", None, None, None, None);
    let g = map_user_to_guard(&u);
    assert_eq!(g.name, "Alice");
    assert_eq!(g.sling_id, 7);
}

#[test]
fn guard_email_and_phone() {
    let u = make_user(
        1,
        "Bob",
        None,
        Some("bob@example.com"),
        Some("+15550001234"),
        None,
    );
    let g = map_user_to_guard(&u);
    assert_eq!(g.email.as_deref(), Some("bob@example.com"));
    assert_eq!(g.phone.as_deref(), Some("+15550001234"));
}

#[test]
fn guard_date_of_birth_parsed() {
    let u = make_user(3, "Carol", None, None, None, Some("1985-03-20"));
    let g = map_user_to_guard(&u);
    assert_eq!(
        g.date_of_birth,
        NaiveDate::from_ymd_opt(1985, 3, 20)
    );
}

#[test]
fn guard_date_of_birth_none() {
    let u = make_user(4, "Dave", None, None, None, None);
    let g = map_user_to_guard(&u);
    assert!(g.date_of_birth.is_none());
}

// ---------------------------------------------------------------------------
// split_groups
// ---------------------------------------------------------------------------

#[test]
fn split_groups_separates_correctly() {
    let groups = vec![
        make_group(1, "location"),
        make_group(2, "position"),
        make_group(3, "location"),
        make_group(4, "position"),
        make_group(5, "position"),
    ];

    let (locations, positions) = split_groups(&groups);

    assert_eq!(locations.len(), 2);
    assert_eq!(positions.len(), 3);

    assert!(locations.iter().all(|g| g.is_location()));
    assert!(positions.iter().all(|g| g.is_position()));
}

#[test]
fn split_groups_all_locations() {
    let groups = vec![make_group(10, "location"), make_group(11, "location")];
    let (locations, positions) = split_groups(&groups);
    assert_eq!(locations.len(), 2);
    assert!(positions.is_empty());
}

#[test]
fn split_groups_empty_slice() {
    let groups: Vec<SlingGroup> = vec![];
    let (locations, positions) = split_groups(&groups);
    assert!(locations.is_empty());
    assert!(positions.is_empty());
}

// ---------------------------------------------------------------------------
// map_shift
// ---------------------------------------------------------------------------

#[test]
fn shift_date_and_times_parsed() {
    let s = make_shift(
        "shift-abc",
        "2024-07-04T08:30:00Z",
        "2024-07-04T16:45:00Z",
        None,
        None,
        None,
    );
    let imp = map_shift(&s).expect("should map");

    assert_eq!(imp.sling_shift_id, "shift-abc");
    assert_eq!(imp.shift_date, NaiveDate::from_ymd_opt(2024, 7, 4).unwrap());
    assert_eq!(imp.start_time, "08:30");
    assert_eq!(imp.end_time, "16:45");
}

#[test]
fn shift_day_of_week_thursday() {
    // 2024-07-04 is a Thursday
    let s = make_shift(
        "s1",
        "2024-07-04T08:00:00Z",
        "2024-07-04T16:00:00Z",
        None,
        None,
        None,
    );
    let imp = map_shift(&s).unwrap();
    assert_eq!(imp.day_of_week, "Thursday");
}

#[test]
fn shift_day_of_week_monday() {
    // 2024-07-01 is a Monday
    let s = make_shift(
        "s2",
        "2024-07-01T06:00:00Z",
        "2024-07-01T14:00:00Z",
        None,
        None,
        None,
    );
    let imp = map_shift(&s).unwrap();
    assert_eq!(imp.day_of_week, "Monday");
}

#[test]
fn shift_ids_extracted() {
    let s = make_shift(
        "s3",
        "2024-07-04T08:00:00Z",
        "2024-07-04T16:00:00Z",
        Some(101),
        Some(202),
        Some(303),
    );
    let imp = map_shift(&s).unwrap();
    assert_eq!(imp.assigned_user_sling_id, Some(101));
    assert_eq!(imp.location_sling_id, Some(202));
    assert_eq!(imp.position_sling_id, Some(303));
}

#[test]
fn shift_ids_none_when_absent() {
    let s = make_shift(
        "s4",
        "2024-07-04T08:00:00Z",
        "2024-07-04T16:00:00Z",
        None,
        None,
        None,
    );
    let imp = map_shift(&s).unwrap();
    assert!(imp.assigned_user_sling_id.is_none());
    assert!(imp.location_sling_id.is_none());
    assert!(imp.position_sling_id.is_none());
}

#[test]
fn shift_invalid_dtstart_returns_none() {
    let s = make_shift("s5", "not-a-date", "2024-07-04T16:00:00Z", None, None, None);
    assert!(map_shift(&s).is_none());
}

#[test]
fn shift_invalid_dtend_returns_none() {
    let s = make_shift("s6", "2024-07-04T08:00:00Z", "bad", None, None, None);
    assert!(map_shift(&s).is_none());
}
