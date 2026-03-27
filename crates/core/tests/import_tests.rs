use chrono::NaiveDate;
use lifebot_core::{db::migrate_conn, import};
use lifebot_sling::mapping::GuardImport;
use rusqlite::Connection;

fn in_memory_conn() -> Connection {
    let conn = Connection::open_in_memory().expect("open in-memory DB");
    conn.pragma_update(None, "foreign_keys", "ON")
        .expect("enable FK");
    migrate_conn(&conn).expect("migrate failed");
    conn
}

#[test]
fn upsert_guard_inserts_new() {
    let conn = in_memory_conn();

    let guard = GuardImport {
        sling_id: 1001,
        name: "Alice Smith".to_string(),
        date_of_birth: Some(NaiveDate::from_ymd_opt(1990, 6, 15).unwrap()),
        phone: Some("555-0100".to_string()),
        email: Some("alice@example.com".to_string()),
    };

    let (inserted, updated) = import::upsert_guards(&conn, &[guard]).expect("upsert failed");

    assert_eq!(inserted, 1);
    assert_eq!(updated, 0);

    let found_sling_id: i64 = conn
        .query_row(
            "SELECT sling_id FROM guards WHERE sling_id = 1001",
            [],
            |row| row.get(0),
        )
        .expect("guard not found");

    assert_eq!(found_sling_id, 1001);
}

#[test]
fn upsert_guard_updates_existing() {
    let conn = in_memory_conn();

    let guard = GuardImport {
        sling_id: 2002,
        name: "Bob Jones".to_string(),
        date_of_birth: None,
        phone: Some("555-0200".to_string()),
        email: Some("bob@example.com".to_string()),
    };

    import::upsert_guards(&conn, &[guard]).expect("first upsert failed");

    let updated_guard = GuardImport {
        sling_id: 2002,
        name: "Bob Jones".to_string(),
        date_of_birth: None,
        phone: Some("555-9999".to_string()),
        email: Some("bob@example.com".to_string()),
    };

    let (inserted, updated) =
        import::upsert_guards(&conn, &[updated_guard]).expect("second upsert failed");

    assert_eq!(inserted, 0);
    assert_eq!(updated, 1);

    let phone: String = conn
        .query_row(
            "SELECT phone FROM guards WHERE sling_id = 2002",
            [],
            |row| row.get(0),
        )
        .expect("guard not found");

    assert_eq!(phone, "555-9999");
}

#[test]
fn upsert_site_from_location() {
    let conn = in_memory_conn();

    let locations = vec![(3001_i64, "Downtown Pool".to_string())];
    let (inserted, updated) = import::upsert_sites(&conn, &locations).expect("upsert failed");

    assert_eq!(inserted, 1);
    assert_eq!(updated, 0);

    let found_sling_id: i64 = conn
        .query_row(
            "SELECT sling_id FROM sites WHERE sling_id = 3001",
            [],
            |row| row.get(0),
        )
        .expect("site not found");

    assert_eq!(found_sling_id, 3001);
}

#[test]
fn upsert_role_from_position() {
    let conn = in_memory_conn();

    let positions = vec![(4001_i64, "Lifeguard".to_string())];
    let (inserted, updated) = import::upsert_roles(&conn, &positions).expect("upsert failed");

    assert_eq!(inserted, 1);
    assert_eq!(updated, 0);

    let found_sling_id: i64 = conn
        .query_row(
            "SELECT sling_id FROM roles WHERE sling_id = 4001",
            [],
            |row| row.get(0),
        )
        .expect("role not found");

    assert_eq!(found_sling_id, 4001);
}
