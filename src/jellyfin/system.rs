use super::jellyfin::JellyfinState;
use super::types::*;
use axum::{extract::State, response::Json};

/// GET /System/Info - Get system information
pub async fn system_info(State(state): State<JellyfinState>) -> Json<SystemInfo> {
    Json(SystemInfo {
        server_name: state.server_name.clone(),
        local_address: None,
        version: "10.10.7".to_string(),
        product_name: "Jellofin Server".to_string(),
        operating_system: std::env::consts::OS.to_string(),
        id: state.server_id.clone(),
        startup_wizard_completed: true,
        ..Default::default()
    })
}

/// GET /System/Info/Public - Get public system information
pub async fn system_info_public(State(state): State<JellyfinState>) -> Json<SystemInfoPublicResponse> {
    Json(SystemInfoPublicResponse {
        server_name: state.server_name.clone(),
        local_address: None,
        version: "10.10.7".to_string(),
        product_name: "Jellofin Server".to_string(),
        operating_system: std::env::consts::OS.to_string(),
        id: state.server_id.clone(),
        startup_wizard_completed: true,
    })
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
