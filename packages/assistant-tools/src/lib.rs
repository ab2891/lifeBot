use anyhow::Result;
use serde_json::Value;

use lifebot_core::LifebotService;

pub struct AssistantTools {
    service: LifebotService,
}

impl AssistantTools {
    pub fn new(service: LifebotService) -> Self {
        Self { service }
    }

    pub fn get_guard_profile(&self) -> Result<Value> {
        Ok(serde_json::to_value(self.service.guard_profiles()?)?)
    }

    pub fn list_open_shifts(&self) -> Result<Value> {
        let shifts = self.service.schedule_view()?;
        Ok(serde_json::to_value(
            shifts
                .into_iter()
                .filter(|shift| shift.assigned_guard_name.is_none())
                .collect::<Vec<_>>(),
        )?)
    }

    pub fn get_shift_history(&self) -> Result<Value> {
        Ok(serde_json::to_value(self.service.schedule_view()?)?)
    }

    pub fn get_shift_queue(&self) -> Result<Value> {
        Ok(serde_json::to_value(self.service.request_queue()?)?)
    }

    pub fn explain_assignment(&self, trace_id: &str) -> Result<Value> {
        Ok(serde_json::to_value(self.service.decision_trace_detail(trace_id)?)?)
    }

    pub fn generate_next_cycle_draft(&self) -> Result<Value> {
        Ok(serde_json::to_value(self.service.generate_draft()?)?)
    }

    pub fn list_cert_expirations(&self) -> Result<Value> {
        Ok(serde_json::to_value(self.service.certification_expirations()?)?)
    }

    pub fn list_policy_violations(&self) -> Result<Value> {
        Ok(serde_json::to_value(self.service.policy_violations()?)?)
    }

    pub fn request_shift_rollover(&self) -> Result<Value> {
        Ok(serde_json::json!({"status":"use seeded demo rollover requests in MVP 1"}))
    }

    pub fn submit_shift_request(&self) -> Result<Value> {
        Ok(serde_json::json!({"status":"use seeded demo request queue in MVP 1"}))
    }

    pub fn approve_draft_schedule(&self) -> Result<Value> {
        self.service.approve_draft_schedule()?;
        Ok(serde_json::json!({"status":"approved"}))
    }

    // --- Sentinel tools ---

    pub fn list_active_sentinel_alerts(&self) -> Result<Value> {
        Ok(serde_json::to_value(self.service.sentinel_active_alerts()?)?)
    }

    pub fn acknowledge_sentinel_alert(&self, alert_id: &str, guard_id: &str, notes: &str) -> Result<Value> {
        self.service.sentinel_acknowledge(alert_id, guard_id, "acknowledged", notes)?;
        Ok(serde_json::json!({"status":"acknowledged", "alert_id": alert_id}))
    }

    pub fn dismiss_sentinel_alert(&self, alert_id: &str, guard_id: &str, notes: &str) -> Result<Value> {
        self.service.sentinel_acknowledge(alert_id, guard_id, "false_positive", notes)?;
        Ok(serde_json::json!({"status":"dismissed", "alert_id": alert_id}))
    }

    pub fn explain_sentinel_event(&self, alert_id: &str) -> Result<Value> {
        Ok(serde_json::to_value(self.service.sentinel_alert_detail(alert_id)?)?)
    }

    pub fn get_current_supervisors_for_pool(&self, pool_id: &str) -> Result<Value> {
        let supervisors = self.service.sentinel_supervisors_for_pool(pool_id)?;
        let result: Vec<_> = supervisors.into_iter().map(|(id, name)| {
            serde_json::json!({"guard_id": id, "name": name})
        }).collect();
        Ok(serde_json::to_value(result)?)
    }

    pub fn simulate_sentinel_event(&self, zone_id: &str, event_type: &str, confidence: f64, duration_secs: f64) -> Result<Value> {
        Ok(serde_json::to_value(self.service.sentinel_simulate_event(zone_id, event_type, confidence, duration_secs)?)?)
    }

    pub fn list_sentinel_event_history(&self) -> Result<Value> {
        Ok(serde_json::to_value(self.service.sentinel_event_history(50)?)?)
    }
}
