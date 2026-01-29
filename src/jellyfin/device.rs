use super::error::apierror;
use super::jellyfin::JellyfinState;
use super::types::*;
use crate::database::model::AccessToken;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Extension,
};

#[derive(serde::Deserialize)]
pub struct DeviceIdQuery {
    pub id: Option<String>,
}

/// GET /Devices
pub async fn devices_get(
    Extension(token): Extension<AccessToken>,
    State(state): State<JellyfinState>,
) -> impl IntoResponse {
    let user = match state.repo.get_user_by_id(&token.user_id).await {
        Ok(u) => u,
        Err(_) => return apierror(StatusCode::NOT_FOUND, "User not found").into_response(),
    };

    let access_tokens = match state.repo.get_access_tokens(&token.user_id).await {
        Ok(tokens) => tokens,
        Err(_) => return apierror(StatusCode::INTERNAL_SERVER_ERROR, "Error retrieving devices").into_response(),
    };

    let mut devices = Vec::new();
    for t in access_tokens {
        devices.push(make_jf_device_item(&t, &user.username));
    }

    let count = devices.len();
    axum::Json(QueryResult {
        items: devices,
        start_index: 0,
        total_record_count: count as i32,
    })
    .into_response()
}

/// DELETE /Devices
pub async fn devices_delete(
    Extension(token): Extension<AccessToken>,
    Query(query): Query<DeviceIdQuery>,
    State(state): State<JellyfinState>,
) -> impl IntoResponse {
    let id = match query.id {
        Some(id) => id,
        None => return apierror(StatusCode::BAD_REQUEST, "device id missing").into_response(),
    };

    let access_tokens = match state.repo.get_access_tokens(&token.user_id).await {
        Ok(tokens) => tokens,
        Err(_) => return apierror(StatusCode::INTERNAL_SERVER_ERROR, "Error retrieving sessions").into_response(),
    };

    for t in access_tokens {
        if t.device_id == id {
            if let Err(_) = state.repo.delete_access_token(&t.token).await {
                return apierror(StatusCode::INTERNAL_SERVER_ERROR, "Error deleting device").into_response();
            }
            return StatusCode::NO_CONTENT.into_response();
        }
    }

    apierror(StatusCode::NOT_FOUND, "device not found").into_response()
}

/// GET /Devices/Info
pub async fn devices_info(
    Extension(token): Extension<AccessToken>,
    Query(query): Query<DeviceIdQuery>,
    State(state): State<JellyfinState>,
) -> impl IntoResponse {
    let id = match query.id {
        Some(id) => id,
        None => return apierror(StatusCode::BAD_REQUEST, "device id missing").into_response(),
    };

    let user = match state.repo.get_user_by_id(&token.user_id).await {
        Ok(u) => u,
        Err(_) => return apierror(StatusCode::NOT_FOUND, "User not found").into_response(),
    };

    let access_tokens = match state.repo.get_access_tokens(&token.user_id).await {
        Ok(tokens) => tokens,
        Err(_) => return apierror(StatusCode::INTERNAL_SERVER_ERROR, "Error retrieving sessions").into_response(),
    };

    for t in access_tokens {
        if t.device_id == id {
            return axum::Json(make_jf_device_item(&t, &user.username)).into_response();
        }
    }

    apierror(StatusCode::NOT_FOUND, "Device not found").into_response()
}

/// GET /Devices/Options
pub async fn devices_options(
    Extension(token): Extension<AccessToken>,
    Query(query): Query<DeviceIdQuery>,
) -> impl IntoResponse {
    let id = match query.id {
        Some(id) => id,
        None => return apierror(StatusCode::BAD_REQUEST, "Device id missing").into_response(),
    };

    axum::Json(DevicesOptionsResponse {
        device_id: id,
        custom_name: token.device_name.clone(),
        disable_auto_login: false,
    })
    .into_response()
}

fn make_jf_device_item(token: &AccessToken, username: &str) -> DeviceItem {
    DeviceItem {
        id: token.device_id.clone(),
        last_user_id: token.user_id.clone(),
        last_user_name: username.to_string(),
        name: token.device_name.clone(),
        app_name: token.application_name.clone(),
        app_version: token.application_version.clone(),
        capabilities: SessionResponseCapabilities {
            playable_media_types: Vec::new(),
            supported_commands: Vec::new(),
            supports_media_control: false,
            supports_persistent_identifier: true,
        },
        date_last_activity: token.last_used,
    }
}
