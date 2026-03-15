use super::jellyfin::JellyfinState;
use super::types::*;
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json, Response},
};
use chrono::Utc;

/// GET /System/Info - Get system information
pub async fn system_info(State(state): State<JellyfinState>) -> Json<SystemInfo> {
    Json(SystemInfo {
        server_name: state.server_name.clone(),
        local_address: None,
        version: "10.11.6".to_string(),
        product_name: "Jellyfin Server".to_string(),
        operating_system: std::env::consts::OS.to_string(),
        id: state.server_id.clone(),
        startup_wizard_completed: true,
        has_update_available: false,
        encoder_location: "System".to_string(),
        system_architecture: std::env::consts::ARCH.to_string(),
        ..Default::default()
    })
}

/// GET /System/Info/Public - Get public system information
pub async fn system_info_public(
    headers: HeaderMap,
    State(state): State<JellyfinState>,
) -> Response {
    // Block desktop and iOS Jellyfin apps — they depend on web assets we don't serve
    if let Some(ua) = headers.get(axum::http::header::USER_AGENT) {
        let ua = ua.to_str().unwrap_or("");
        if ua.starts_with("Jellyfin/") || ua.starts_with("JellyfinMediaPlayer") {
            return StatusCode::IM_A_TEAPOT.into_response();
        }
    }
    Json(SystemInfoPublicResponse {
        server_name: state.server_name.clone(),
        local_address: None,
        version: "10.11.6".to_string(),
        product_name: "Jellyfin Server".to_string(),
        operating_system: std::env::consts::OS.to_string(),
        id: state.server_id.clone(),
        startup_wizard_completed: true,
    })
    .into_response()
}

/// GET /GetUtcTime
pub async fn get_utc_time() -> Json<GetUtcTimeResponse> {
    let now = Utc::now().to_rfc3339();
    Json(GetUtcTimeResponse {
        request_reception_time: now.clone(),
        response_transmission_time: now,
    })
}

/// GET /System/Endpoint
pub async fn system_endpoint() -> Json<SystemEndpointResponse> {
    Json(SystemEndpointResponse {
        is_local: false,
        is_in_network: false,
    })
}

/// GET /System/Logs
pub async fn system_logs() -> Json<Vec<serde_json::Value>> {
    Json(Vec::new())
}

/// POST /System/Restart
pub async fn system_restart() -> StatusCode {
    StatusCode::FORBIDDEN
}

/// POST /System/Shutdown
pub async fn system_shutdown() -> StatusCode {
    StatusCode::FORBIDDEN
}

/// GET /ScheduledTasks
pub async fn scheduled_tasks() -> Json<Vec<ScheduledTaskInfo>> {
    Json(vec![ScheduledTaskInfo {
        id: "RefreshLibrary".to_string(),
        name: "Scan Media Library".to_string(),
        state: "Idle".to_string(),
        category: "Library".to_string(),
        description: "Scans all libraries and refreshes metadata.".to_string(),
        triggers: vec![ScheduledTaskTrigger {
            trigger_type: "IntervalTrigger".to_string(),
            interval_ticks: Some(720_000_000_000), // 2 hours in 100ns ticks
            time_of_day_ticks: None,
            day_of_week: None,
            max_runtime_ticks: None,
        }],
        last_execution_result: None,
    }])
}

/// GET /System/Ping - Ping endpoint
pub async fn system_ping() -> &'static str {
    "\"Jellyfin Server\""
}

/// GET /health - Health check
pub async fn health() -> &'static str {
    "Healthy"
}

/// GET /Plugins - Get plugins (returns empty list)
pub async fn plugins() -> Json<Vec<serde_json::Value>> {
    Json(Vec::new())
}

/// GET /socket - WebSocket endpoint (not implemented)
pub async fn socket_handler() -> axum::http::StatusCode {
    axum::http::StatusCode::NOT_FOUND
}

/// GET / - Root handler
pub async fn root_handler() -> impl axum::response::IntoResponse {
    use axum::response::Html;
    Html("<!DOCTYPE html><html><head><title>Jellofin Server</title></head><body><h1>Jellofin Server</h1></body></html>")
}
