use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolContract {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvocationEnvelope {
    pub tool_name: String,
    pub args: serde_json::Value,
}

pub struct MockOpenClawAdapter;

impl MockOpenClawAdapter {
    pub fn health(&self) -> Result<serde_json::Value> {
        Ok(serde_json::json!({
            "adapter": "mock-openclaw",
            "status": "ready"
        }))
    }

    pub fn contracts(&self) -> Vec<ToolContract> {
        vec![
            ToolContract {
                name: "generate_next_cycle_draft".into(),
                description: "Generate a reviewable next-cycle draft schedule.".into(),
            },
            ToolContract {
                name: "explain_assignment".into(),
                description: "Explain why a guard received or did not receive a shift.".into(),
            },
        ]
    }
}
