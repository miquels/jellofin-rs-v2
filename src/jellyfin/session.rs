use super::error::apierror;
use super::jellyfin::JellyfinState;
use super::types::*;
use crate::database::model::AccessToken;
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json},
    Extension,
};
use std::collections::HashMap;

const SESSION_ID: &str = "e3a869b7a901f8894de8ee65688db6c0";

/// GET /Sessions
pub async fn sessions(
    Extension(token): Extension<AccessToken>,
    State(state): State<JellyfinState>,
) -> impl IntoResponse {
    let user = match state.repo.get_user_by_id(&token.user_id).await {
        Ok(u) => u,
        Err(_) => return apierror(StatusCode::NOT_FOUND, "User not found").into_response(),
    };

    let access_tokens = match state.repo.get_access_tokens(&token.user_id).await {
        Ok(tokens) => tokens,
        Err(_) => return apierror(StatusCode::INTERNAL_SERVER_ERROR, "error retrieving sessions").into_response(),
    };

    // Keep most recent access token per device_id
    let mut unique_tokens = HashMap::new();
    for t in access_tokens {
        let entry = unique_tokens.entry(t.device_id.clone()).or_insert(t.clone());
        if t.last_used > entry.last_used {
            *entry = t;
        }
    }

    let mut sessions = Vec::new();
    for t in unique_tokens.values() {
        sessions.push(make_jf_session_info(&state, t, &user.username));
    }

    Json(sessions).into_response()
}

fn make_jf_session_info(state: &JellyfinState, token: &AccessToken, username: &str) -> SessionInfo {
    SessionInfo {
        id: SESSION_ID.to_string(),
        user_id: token.user_id.clone(),
        user_name: username.to_string(),
        last_activity_date: token.last_used,
        remote_end_point: token.remote_address.clone(),
        device_name: token.device_name.clone(),
        device_id: token.device_id.clone(),
        client: token.application_name.clone(),
        application_version: token.application_version.clone(),
        is_active: true,
        supports_media_control: false,
        supports_remote_control: false,
        has_custom_device_name: false,
        server_id: state.server_id.clone(),
        additional_users: Vec::new(),
        play_state: PlayState {
            repeat_mode: "RepeatNone".to_string(),
            playback_order: "Default".to_string(),
            can_seek: false,
            is_paused: false,
            is_muted: false,
        },
        capabilities: SessionResponseCapabilities {
            playable_media_types: Vec::new(),
            supported_commands: Vec::new(),
            supports_persistent_identifier: true,
            supports_media_control: false,
        },
        now_playing_queue: Vec::new(),
        now_playing_queue_full_items: Vec::new(),
        supported_commands: Vec::new(),
        playable_media_types: Vec::new(),
    }
}

/// POST /Sessions/Capabilities
pub async fn sessions_capabilities() -> impl IntoResponse {
    StatusCode::NO_CONTENT
}

/// POST /Sessions/Capabilities/Full
pub async fn sessions_capabilities_full() -> impl IntoResponse {
    StatusCode::NO_CONTENT
}
