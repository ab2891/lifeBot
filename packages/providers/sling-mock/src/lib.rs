use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlingSnapshot {
    pub provider: String,
    pub sites: Vec<SlingSite>,
    pub pools: Vec<SlingPool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlingSite {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlingPool {
    #[serde(rename = "siteId")]
    pub site_id: String,
    pub id: String,
    pub name: String,
}

pub trait SlingProvider {
    fn import_schedule_snapshot(&self) -> Result<SlingSnapshot>;
    fn export_draft_schedule(&self, payload: serde_json::Value) -> Result<serde_json::Value>;
    fn get_sync_status(&self) -> Result<serde_json::Value>;
}

pub struct MockSlingProvider {
    fixture: &'static str,
}

impl Default for MockSlingProvider {
    fn default() -> Self {
        Self {
            fixture: include_str!("../../../../fixtures/sling/mock-schedule.json"),
        }
    }
}

impl SlingProvider for MockSlingProvider {
    fn import_schedule_snapshot(&self) -> Result<SlingSnapshot> {
        Ok(serde_json::from_str(self.fixture)?)
    }

    fn export_draft_schedule(&self, payload: serde_json::Value) -> Result<serde_json::Value> {
        Ok(serde_json::json!({
            "provider": "sling-mock",
            "status": "not_exported_in_mvp",
            "payloadPreview": payload
        }))
    }

    fn get_sync_status(&self) -> Result<serde_json::Value> {
        Ok(serde_json::json!({
            "provider": "sling-mock",
            "mode": "fixture",
            "status": "ready"
        }))
    }
}
