use std::sync::Mutex;

use lifebot_core::LifebotService;

#[derive(Default)]
pub struct AppState {
    service: Mutex<Option<LifebotService>>,
}

mod commands {
    use std::env;

    use lifebot_assistant_tools::AssistantTools;
    use serde::Serialize;
    use tauri::{Manager, State};

    use crate::AppState;
    use lifebot_core::LifebotService;

    fn with_service<R>(
        state: &State<AppState>,
        f: impl FnOnce(&LifebotService) -> anyhow::Result<R>,
    ) -> Result<R, String> {
        let guard = state
            .service
            .lock()
            .map_err(|_| "Unable to lock app state".to_string())?;
        let service = guard
            .as_ref()
            .ok_or_else(|| "Lifebot service is not initialized".to_string())?;
        f(service).map_err(|err| err.to_string())
    }

    #[tauri::command]
    pub fn bootstrap_app(app: tauri::AppHandle, state: State<AppState>) -> Result<(), String> {
        let base_dir = app
            .path()
            .app_local_data_dir()
            .map_err(|err| err.to_string())?;
        std::fs::create_dir_all(&base_dir).map_err(|err| err.to_string())?;
        env::set_var("LIFEBOT_DB_PATH", base_dir.join("lifebot-demo.db"));
        let service = LifebotService::from_env(base_dir);
        service.init().map_err(|err| err.to_string())?;
        *state
            .service
            .lock()
            .map_err(|_| "Unable to store app state".to_string())? = Some(service);
        Ok(())
    }

    #[tauri::command]
    pub fn get_dashboard(state: State<AppState>) -> Result<impl Serialize, String> {
        with_service(&state, |service| service.dashboard())
    }

    #[tauri::command]
    pub fn list_guard_profiles(state: State<AppState>) -> Result<impl Serialize, String> {
        with_service(&state, |service| service.guard_profiles())
    }

    #[tauri::command]
    pub fn list_schedule_view(state: State<AppState>) -> Result<impl Serialize, String> {
        with_service(&state, |service| service.schedule_view())
    }

    #[tauri::command]
    pub fn list_request_queue(state: State<AppState>) -> Result<impl Serialize, String> {
        with_service(&state, |service| service.request_queue())
    }

    #[tauri::command]
    pub fn list_policy_violations(state: State<AppState>) -> Result<impl Serialize, String> {
        with_service(&state, |service| service.policy_violations())
    }

    #[tauri::command]
    pub fn list_cert_expirations(state: State<AppState>) -> Result<impl Serialize, String> {
        with_service(&state, |service| service.certification_expirations())
    }

    #[tauri::command]
    pub fn generate_next_cycle_draft(state: State<AppState>) -> Result<impl Serialize, String> {
        with_service(&state, |service| service.generate_draft())
    }

    #[tauri::command]
    pub fn approve_draft_schedule(state: State<AppState>) -> Result<(), String> {
        with_service(&state, |service| service.approve_draft_schedule())
    }

    #[tauri::command]
    pub fn list_decision_traces(state: State<AppState>) -> Result<impl Serialize, String> {
        with_service(&state, |service| service.decision_traces())
    }

    #[tauri::command]
    pub fn get_decision_trace_detail(
        trace_id: String,
        state: State<AppState>,
    ) -> Result<impl Serialize, String> {
        with_service(&state, |service| service.decision_trace_detail(&trace_id))
    }

    #[tauri::command]
    pub fn get_assistant_examples(state: State<AppState>) -> Result<impl Serialize, String> {
        with_service(&state, |service| Ok(service.assistant_examples()))
    }

    // --- Sentinel ---

    #[tauri::command]
    pub fn sentinel_dashboard(state: State<AppState>) -> Result<impl Serialize, String> {
        with_service(&state, |service| service.sentinel_dashboard())
    }

    #[tauri::command]
    pub fn sentinel_active_alerts(state: State<AppState>) -> Result<impl Serialize, String> {
        with_service(&state, |service| service.sentinel_active_alerts())
    }

    #[tauri::command]
    pub fn sentinel_event_history(state: State<AppState>) -> Result<impl Serialize, String> {
        with_service(&state, |service| service.sentinel_event_history(50))
    }

    #[tauri::command]
    pub fn sentinel_zones(state: State<AppState>) -> Result<impl Serialize, String> {
        with_service(&state, |service| service.sentinel_zones())
    }

    #[tauri::command]
    pub fn sentinel_simulate(
        zone_id: String,
        event_type: String,
        confidence: f64,
        duration_secs: f64,
        state: State<AppState>,
    ) -> Result<impl Serialize, String> {
        with_service(&state, |service| {
            service.sentinel_simulate_event(&zone_id, &event_type, confidence, duration_secs)
        })
    }

    #[tauri::command]
    pub fn sentinel_acknowledge(
        alert_id: String,
        guard_id: String,
        action: String,
        notes: String,
        state: State<AppState>,
    ) -> Result<(), String> {
        with_service(&state, |service| {
            service.sentinel_acknowledge(&alert_id, &guard_id, &action, &notes)
        })
    }

    #[tauri::command]
    pub fn sentinel_cameras(state: State<AppState>) -> Result<impl Serialize, String> {
        with_service(&state, |service| service.sentinel_cameras())
    }

    #[tauri::command]
    pub fn sentinel_add_camera(site_id: String, name: String, location: String, stream_url: String, state: State<AppState>) -> Result<impl Serialize, String> {
        with_service(&state, |service| service.sentinel_add_camera(&site_id, &name, &location, &stream_url))
    }

    #[tauri::command]
    pub fn sentinel_update_camera(camera_id: String, name: String, location: String, stream_url: String, active: bool, state: State<AppState>) -> Result<(), String> {
        with_service(&state, |service| service.sentinel_update_camera(&camera_id, &name, &location, &stream_url, active))
    }

    #[tauri::command]
    pub fn sentinel_delete_camera(camera_id: String, state: State<AppState>) -> Result<(), String> {
        with_service(&state, |service| service.sentinel_delete_camera(&camera_id))
    }

    #[tauri::command]
    pub fn sentinel_assign_camera_to_zone(zone_id: String, camera_id: Option<String>, state: State<AppState>) -> Result<(), String> {
        with_service(&state, |service| service.sentinel_assign_camera_to_zone(&zone_id, camera_id.as_deref()))
    }

    #[tauri::command]
    pub fn sentinel_cv_health(state: State<AppState>) -> Result<bool, String> {
        with_service(&state, |service| service.sentinel_cv_health())
    }

    #[tauri::command]
    pub fn sentinel_run_detection(state: State<AppState>) -> Result<impl Serialize, String> {
        with_service(&state, |service| service.sentinel_run_detection_pass())
    }

    #[tauri::command]
    pub fn sentinel_alert_detail(alert_id: String, state: State<AppState>) -> Result<impl Serialize, String> {
        with_service(&state, |service| service.sentinel_alert_detail(&alert_id))
    }

    // --- Integrations ---

    #[tauri::command]
    pub fn get_integrations(state: State<AppState>) -> Result<impl Serialize, String> {
        with_service(&state, |service| service.get_integrations())
    }

    #[tauri::command]
    pub fn save_integration(key: String, value: String, state: State<AppState>) -> Result<(), String> {
        with_service(&state, |service| service.save_integration(&key, &value))
    }

    #[tauri::command]
    pub fn disconnect_integration(key: String, state: State<AppState>) -> Result<(), String> {
        with_service(&state, |service| service.disconnect_integration(&key))
    }

    #[tauri::command]
    pub fn run_assistant_query(
        query: String,
        state: State<AppState>,
    ) -> Result<impl Serialize, String> {
        with_service(&state, |service| {
            let tools = AssistantTools::new(service.clone());
            if query.to_lowercase().contains("approve") {
                tools.approve_draft_schedule()?;
            }
            service.run_assistant_query(&query)
        })
    }
}

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            // On Linux/WSL2, WebKit's GPU compositor fails because there is no
            // DRM device. Force software rendering at the API level so the
            // webview actually paints instead of showing a blank window.
            #[cfg(target_os = "linux")]
            {
                use tauri::Manager;
                use webkit2gtk::{SettingsExt, WebViewExt, HardwareAccelerationPolicy};
                let window = app.get_webview_window("main")
                    .expect("main window must exist");
                window.with_webview(|webview| {
                    let wv = webview.inner();
                    if let Some(settings) = wv.settings() {
                        settings.set_hardware_acceleration_policy(
                            HardwareAccelerationPolicy::Never,
                        );
                    }
                })?;
            }
            Ok(())
        })
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::bootstrap_app,
            commands::get_dashboard,
            commands::list_guard_profiles,
            commands::list_schedule_view,
            commands::list_request_queue,
            commands::list_policy_violations,
            commands::list_cert_expirations,
            commands::generate_next_cycle_draft,
            commands::approve_draft_schedule,
            commands::list_decision_traces,
            commands::get_decision_trace_detail,
            commands::get_assistant_examples,
            commands::run_assistant_query,
            commands::sentinel_dashboard,
            commands::sentinel_active_alerts,
            commands::sentinel_event_history,
            commands::sentinel_zones,
            commands::sentinel_simulate,
            commands::sentinel_acknowledge,
            commands::sentinel_alert_detail,
            commands::sentinel_cameras,
            commands::sentinel_add_camera,
            commands::sentinel_update_camera,
            commands::sentinel_delete_camera,
            commands::sentinel_assign_camera_to_zone,
            commands::sentinel_cv_health,
            commands::sentinel_run_detection,
            commands::get_integrations,
            commands::save_integration,
            commands::disconnect_integration
        ])
        .run(tauri::generate_context!())
        .expect("error while running Lifebot desktop");
}
