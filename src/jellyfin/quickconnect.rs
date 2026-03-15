use axum::{
    extract::{Query, State},
    http::StatusCode,
    Extension, Json,
};
use std::collections::HashMap;

use crate::database::model;

use super::auth::{generate_random_token, parse_auth_header, JellyfinAuthState};

/// GET /QuickConnect/Enabled
pub async fn quick_connect_enabled(State(state): State<JellyfinAuthState>) -> Json<bool> {
    Json(state.quick_connect)
}

/// POST /QuickConnect/Initiate - Start a new QuickConnect session
pub async fn quick_connect_initiate(
    State(state): State<JellyfinAuthState>,
    headers: axum::http::HeaderMap,
) -> axum::response::Response {
    use axum::response::IntoResponse;
    if !state.quick_connect {
        return StatusCode::FORBIDDEN.into_response();
    }

    let emby = parse_auth_header(&headers);
    let device_id = emby
        .as_ref()
        .map(|h| h.device_id.as_str())
        .unwrap_or("unknown")
        .to_string();

    // Generate 6-digit user-visible code and 32-char secret
    let code = {
        use rand::Rng;
        let n: u32 = rand::thread_rng().gen_range(0..1_000_000);
        format!("{:06}", n)
    };
    let secret = generate_random_token();

    let qc = model::QuickConnectCode {
        user_id: String::new(),
        device_id: device_id.clone(),
        secret: secret.clone(),
        authorized: false,
        code: code.clone(),
        created: chrono::Utc::now(),
    };

    if state.repo.upsert_quick_connect(&qc).await.is_err() {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    Json(serde_json::json!({
        "Secret": secret,
        "Code": code,
        "DeviceId": device_id,
        "Authenticated": false
    }))
    .into_response()
}

/// GET /QuickConnect/Connect - Check QuickConnect status by secret
pub async fn quick_connect_connect(
    State(state): State<JellyfinAuthState>,
    Query(params): Query<HashMap<String, String>>,
) -> axum::response::Response {
    use axum::response::IntoResponse;
    if !state.quick_connect {
        return StatusCode::FORBIDDEN.into_response();
    }

    let secret = match params.get("Secret") {
        Some(s) => s.clone(),
        None => return StatusCode::BAD_REQUEST.into_response(),
    };

    match state.repo.get_quick_connect_by_secret(&secret).await {
        Ok(qc) => Json(serde_json::json!({
            "Secret": qc.secret,
            "Code": qc.code,
            "DeviceId": qc.device_id,
            "Authenticated": qc.authorized
        }))
        .into_response(),
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

/// POST /QuickConnect/Authorize - Authorize a QuickConnect code (requires auth)
/// Uses JellyfinState so it can be placed in the authenticated router section.
pub async fn quick_connect_authorize(
    Extension(token): Extension<model::AccessToken>,
    State(state): State<super::jellyfin::JellyfinState>,
    Query(params): Query<HashMap<String, String>>,
) -> StatusCode {
    if !state.config.quick_connect().unwrap_or(false) {
        return StatusCode::FORBIDDEN;
    }

    let code = match params.get("Code") {
        Some(c) => c.clone(),
        None => return StatusCode::BAD_REQUEST,
    };

    let mut qc = match state.repo.get_quick_connect_by_code(&code).await {
        Ok(q) => q,
        Err(_) => return StatusCode::NOT_FOUND,
    };

    // Check code hasn't expired (10 minute window)
    let age = chrono::Utc::now() - qc.created;
    if age.num_minutes() > 10 {
        return StatusCode::GONE;
    }

    qc.authorized = true;
    qc.user_id = token.user_id.clone();

    match state.repo.upsert_quick_connect(&qc).await {
        Ok(_) => StatusCode::OK,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}
