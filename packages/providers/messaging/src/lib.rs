use anyhow::Result;
use chrono::Utc;
use rusqlite::{params, Connection};

pub trait MessageProvider {
    fn provider_name(&self) -> &'static str;
    fn deliver(&self, conn: &Connection, recipient: &str, body: &str) -> Result<()>;
}

pub struct ConsoleLogProvider;
pub struct InAppNotificationProvider;
pub struct FakeSmsProvider;
pub struct FakeGroupMeProvider;

impl MessageProvider for ConsoleLogProvider {
    fn provider_name(&self) -> &'static str {
        "console"
    }

    fn deliver(&self, conn: &Connection, recipient: &str, body: &str) -> Result<()> {
        log_message(conn, self.provider_name(), recipient, body)
    }
}

impl MessageProvider for InAppNotificationProvider {
    fn provider_name(&self) -> &'static str {
        "in_app"
    }

    fn deliver(&self, conn: &Connection, recipient: &str, body: &str) -> Result<()> {
        log_message(conn, self.provider_name(), recipient, body)
    }
}

impl MessageProvider for FakeSmsProvider {
    fn provider_name(&self) -> &'static str {
        "fake_sms"
    }

    fn deliver(&self, conn: &Connection, recipient: &str, body: &str) -> Result<()> {
        log_message(conn, self.provider_name(), recipient, body)
    }
}

impl MessageProvider for FakeGroupMeProvider {
    fn provider_name(&self) -> &'static str {
        "fake_groupme"
    }

    fn deliver(&self, conn: &Connection, recipient: &str, body: &str) -> Result<()> {
        log_message(conn, self.provider_name(), recipient, body)
    }
}

pub fn log_message(conn: &Connection, provider_name: &str, recipient: &str, body: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO message_log (id, provider_name, recipient, body, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            format!("msg-{}", uuid::Uuid::new_v4()),
            provider_name,
            recipient,
            body,
            Utc::now().naive_utc().to_string()
        ],
    )?;
    Ok(())
}
