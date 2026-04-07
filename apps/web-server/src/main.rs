use std::{env, net::SocketAddr, path::PathBuf, sync::Arc};

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use serde::Deserialize;
use tower_http::{
    cors::CorsLayer,
    services::{ServeDir, ServeFile},
};

use lifebot_core::LifebotService;

// ---------------------------------------------------------------------------
// Shared state
// ---------------------------------------------------------------------------

type AppState = Arc<LifebotService>;

// ---------------------------------------------------------------------------
// Error helper
// ---------------------------------------------------------------------------

struct AppError(anyhow::Error);

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        eprintln!("Internal error: {}", self.0);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": "An internal error occurred" })),
        )
            .into_response()
    }
}

impl<E: Into<anyhow::Error>> From<E> for AppError {
    fn from(err: E) -> Self {
        Self(err.into())
    }
}

type ApiResult<T> = Result<Json<T>, AppError>;

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn bootstrap(State(svc): State<AppState>) -> Result<Json<()>, AppError> {
    svc.init()?;
    Ok(Json(()))
}

async fn dashboard(State(svc): State<AppState>) -> ApiResult<serde_json::Value> {
    Ok(Json(serde_json::to_value(svc.dashboard()?)?))
}

async fn guards(State(svc): State<AppState>) -> ApiResult<serde_json::Value> {
    Ok(Json(serde_json::to_value(svc.guard_profiles()?)?))
}

async fn schedule(State(svc): State<AppState>) -> ApiResult<serde_json::Value> {
    Ok(Json(serde_json::to_value(svc.schedule_view()?)?))
}

async fn queue(State(svc): State<AppState>) -> ApiResult<serde_json::Value> {
    Ok(Json(serde_json::to_value(svc.request_queue()?)?))
}

async fn violations(State(svc): State<AppState>) -> ApiResult<serde_json::Value> {
    Ok(Json(serde_json::to_value(svc.policy_violations()?)?))
}

async fn expirations(State(svc): State<AppState>) -> ApiResult<serde_json::Value> {
    Ok(Json(serde_json::to_value(svc.certification_expirations()?)?))
}

async fn generate_draft(State(svc): State<AppState>) -> ApiResult<serde_json::Value> {
    Ok(Json(serde_json::to_value(svc.generate_draft()?)?))
}

async fn approve_draft(State(svc): State<AppState>) -> Result<Json<()>, AppError> {
    svc.approve_draft_schedule()?;
    Ok(Json(()))
}

async fn traces(State(svc): State<AppState>) -> ApiResult<serde_json::Value> {
    Ok(Json(serde_json::to_value(svc.decision_traces()?)?))
}

async fn trace_detail(
    Path(id): Path<String>,
    State(svc): State<AppState>,
) -> ApiResult<serde_json::Value> {
    Ok(Json(serde_json::to_value(svc.decision_trace_detail(&id)?)?))
}

async fn assistant_examples(State(svc): State<AppState>) -> ApiResult<serde_json::Value> {
    Ok(Json(serde_json::to_value(svc.assistant_examples())?))
}

// --- Sentinel handlers ---

async fn sentinel_dashboard_handler(State(svc): State<AppState>) -> ApiResult<serde_json::Value> {
    Ok(Json(serde_json::to_value(svc.sentinel_dashboard()?)?))
}

async fn sentinel_active_alerts_handler(State(svc): State<AppState>) -> ApiResult<serde_json::Value> {
    Ok(Json(serde_json::to_value(svc.sentinel_active_alerts()?)?))
}

async fn sentinel_events_handler(State(svc): State<AppState>) -> ApiResult<serde_json::Value> {
    Ok(Json(serde_json::to_value(svc.sentinel_event_history(50)?)?))
}

async fn sentinel_zones_handler(State(svc): State<AppState>) -> ApiResult<serde_json::Value> {
    Ok(Json(serde_json::to_value(svc.sentinel_zones()?)?))
}

#[derive(Deserialize)]
struct SimulateBody {
    zone_id: String,
    event_type: String,
    confidence: f64,
    duration_secs: f64,
}

async fn sentinel_simulate_handler(
    State(svc): State<AppState>,
    Json(body): Json<SimulateBody>,
) -> ApiResult<serde_json::Value> {
    Ok(Json(serde_json::to_value(svc.sentinel_simulate_event(
        &body.zone_id, &body.event_type, body.confidence, body.duration_secs,
    )?)?))
}

#[derive(Deserialize)]
struct AckBody {
    alert_id: String,
    guard_id: String,
    action: String,
    notes: String,
}

async fn sentinel_acknowledge_handler(
    State(svc): State<AppState>,
    Json(body): Json<AckBody>,
) -> Result<Json<()>, AppError> {
    svc.sentinel_acknowledge(&body.alert_id, &body.guard_id, &body.action, &body.notes)?;
    Ok(Json(()))
}

async fn sentinel_alert_detail_handler(
    Path(id): Path<String>,
    State(svc): State<AppState>,
) -> ApiResult<serde_json::Value> {
    Ok(Json(serde_json::to_value(svc.sentinel_alert_detail(&id)?)?))
}

async fn sentinel_cameras_handler(State(svc): State<AppState>) -> ApiResult<serde_json::Value> {
    Ok(Json(serde_json::to_value(svc.sentinel_cameras()?)?))
}

#[derive(Deserialize)]
struct AddCameraBody { site_id: String, name: String, location: String, stream_url: String }

async fn sentinel_add_camera_handler(State(svc): State<AppState>, Json(body): Json<AddCameraBody>) -> ApiResult<serde_json::Value> {
    Ok(Json(serde_json::to_value(svc.sentinel_add_camera(&body.site_id, &body.name, &body.location, &body.stream_url)?)?))
}

#[derive(Deserialize)]
struct UpdateCameraBody { name: String, location: String, stream_url: String, active: bool }

async fn sentinel_update_camera_handler(Path(id): Path<String>, State(svc): State<AppState>, Json(body): Json<UpdateCameraBody>) -> Result<Json<()>, AppError> {
    svc.sentinel_update_camera(&id, &body.name, &body.location, &body.stream_url, body.active)?;
    Ok(Json(()))
}

async fn sentinel_delete_camera_handler(Path(id): Path<String>, State(svc): State<AppState>) -> Result<Json<()>, AppError> {
    svc.sentinel_delete_camera(&id)?;
    Ok(Json(()))
}

async fn sentinel_cv_health_handler(State(svc): State<AppState>) -> ApiResult<serde_json::Value> {
    Ok(Json(serde_json::json!({"healthy": svc.sentinel_cv_health()?})))
}

async fn sentinel_run_detection_handler(State(svc): State<AppState>) -> ApiResult<serde_json::Value> {
    Ok(Json(serde_json::to_value(svc.sentinel_run_detection_pass()?)?))
}

// --- Integration handlers ---

async fn integrations(State(svc): State<AppState>) -> ApiResult<serde_json::Value> {
    Ok(Json(serde_json::to_value(svc.get_integrations()?)?))
}

#[derive(Deserialize)]
struct IntegrationBody {
    key: String,
    value: String,
}

async fn save_integration_handler(
    State(svc): State<AppState>,
    Json(body): Json<IntegrationBody>,
) -> Result<Json<()>, AppError> {
    svc.save_integration(&body.key, &body.value)?;
    Ok(Json(()))
}

async fn disconnect_integration_handler(
    Path(key): Path<String>,
    State(svc): State<AppState>,
) -> Result<Json<()>, AppError> {
    svc.disconnect_integration(&key)?;
    Ok(Json(()))
}

#[derive(Deserialize)]
struct QueryBody {
    query: String,
}

async fn assistant_query(
    State(svc): State<AppState>,
    Json(body): Json<QueryBody>,
) -> ApiResult<serde_json::Value> {
    // Safety: the assistant must never directly approve schedules.
    // If the user asks to approve, instruct them to use the UI button.
    if body.query.to_lowercase().contains("approve") {
        use lifebot_core::models::AssistantResponse;
        let response = AssistantResponse {
            tool: "approve_draft".into(),
            title: "Approve Draft Schedule".into(),
            explanation: "To approve the draft schedule, use the 'Approve Draft' button in the top bar.".into(),
            data: serde_json::Value::Null,
        };
        return Ok(Json(serde_json::to_value(response)?));
    }
    Ok(Json(serde_json::to_value(
        svc.run_assistant_query(&body.query)?,
    )?))
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Determine data directory
    let data_dir = env::var("LIFEBOT_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("./data"));
    std::fs::create_dir_all(&data_dir)?;

    let db_path = data_dir.join("lifebot-demo.db");
    env::set_var("LIFEBOT_DB_PATH", &db_path);

    // Build and auto-bootstrap service
    let service = LifebotService::from_env(&data_dir);
    service.init()?;
    println!("[lifebot] DB initialized at {}", db_path.display());

    let state: AppState = Arc::new(service);

    // API routes
    let api = Router::new()
        .route("/api/bootstrap", post(bootstrap))
        .route("/api/dashboard", get(dashboard))
        .route("/api/guards", get(guards))
        .route("/api/schedule", get(schedule))
        .route("/api/queue", get(queue))
        .route("/api/violations", get(violations))
        .route("/api/expirations", get(expirations))
        .route("/api/draft/generate", post(generate_draft))
        .route("/api/draft/approve", post(approve_draft))
        .route("/api/traces", get(traces))
        .route("/api/traces/{id}", get(trace_detail))
        .route("/api/assistant/examples", get(assistant_examples))
        .route("/api/assistant/query", post(assistant_query))
        .route("/api/sentinel/dashboard", get(sentinel_dashboard_handler))
        .route("/api/sentinel/alerts", get(sentinel_active_alerts_handler))
        .route("/api/sentinel/alerts/{id}", get(sentinel_alert_detail_handler))
        .route("/api/sentinel/events", get(sentinel_events_handler))
        .route("/api/sentinel/zones", get(sentinel_zones_handler))
        .route("/api/sentinel/simulate", post(sentinel_simulate_handler))
        .route("/api/sentinel/acknowledge", post(sentinel_acknowledge_handler))
        .route("/api/sentinel/cameras", get(sentinel_cameras_handler))
        .route("/api/sentinel/cameras", post(sentinel_add_camera_handler))
        .route("/api/sentinel/cameras/{id}", axum::routing::put(sentinel_update_camera_handler))
        .route("/api/sentinel/cameras/{id}", axum::routing::delete(sentinel_delete_camera_handler))
        .route("/api/sentinel/cv/health", get(sentinel_cv_health_handler))
        .route("/api/sentinel/cv/detect", post(sentinel_run_detection_handler))
        .route("/api/integrations", get(integrations))
        .route("/api/integrations", post(save_integration_handler))
        .route("/api/integrations/{key}", axum::routing::delete(disconnect_integration_handler));

    // Static file serving — serves the built Svelte frontend
    let static_dir = env::var("LIFEBOT_STATIC_DIR")
        .unwrap_or_else(|_| "./apps/desktop/dist".into());

    let index_file = format!("{}/index.html", static_dir);
    let serve_dir = ServeDir::new(&static_dir)
        .not_found_service(ServeFile::new(&index_file));

    let app = api
        .fallback_service(serve_dir)
        .layer(CorsLayer::permissive())
        .with_state(state);

    let port: u16 = env::var("LIFEBOT_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3100);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    println!("[lifebot] Serving on http://localhost:{}", port);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
