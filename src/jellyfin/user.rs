use super::auth::{create_new_token, create_user, make_session_info, parse_auth_header, JellyfinAuthState};
use super::jellyfin::JellyfinState;
use super::types::*;
use crate::database::{model, ImageMetadata};
use crate::identicon::generate_identicon;
use crate::idhash::id_hash;
use axum::{
    extract::{Path as AxumPath, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json, Response},
    Extension,
};
use bcrypt::{hash, verify, DEFAULT_COST};
use tracing::error;

#[derive(serde::Deserialize)]
pub struct UserNewRequest {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Password")]
    pub password: String,
}

#[derive(serde::Deserialize)]
pub struct UserPasswordRequest {
    #[serde(rename = "CurrentPw", default)]
    pub current_pw: String,
    #[serde(rename = "NewPw", default)]
    pub new_pw: String,
}

/// POST /Users/AuthenticateByName
pub async fn authenticate_by_name(
    State(state): State<JellyfinAuthState>,
    headers: HeaderMap,
    Json(request): Json<AuthenticateUserByNameRequest>,
) -> Result<Json<AuthenticateByNameResponse>, StatusCode> {
    if request.username.is_empty() || request.pw.is_empty() {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let username = request.username.to_lowercase();

    // Try to get user from database
    let mut user = state.repo.get_user(&username).await.ok();

    // Check if user exists or needs creation
    if let Some(db_user) = user.take() {
         // Verify password
        if !verify(&request.pw, &db_user.password).unwrap_or(false) {
            return Err(StatusCode::UNAUTHORIZED);
        } else {
            user = Some(db_user);
        }
    } else if state.auto_register {
        // Auto-register user
        user = create_user(&state.repo, &username, &request.pw).await.ok();
        if user.is_none() {
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    } else {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let mut user = user.unwrap();

    // Update last login
    user.last_login = chrono::Utc::now();
    user.last_used = chrono::Utc::now();
    if let Err(_) = state.repo.upsert_user(&user).await {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    // Parse auth header
    let emby_header = parse_auth_header(&headers);

    let device_id = emby_header.as_ref().map(|h| h.device_id.as_str()).unwrap_or("");

    // Reuse existing token for the same device if one exists
    let access_token = if !device_id.is_empty() {
        if let Ok(mut existing) = state.repo.get_access_token_by_device_id(device_id).await {
            // Update last_used and details, reuse the token string
            existing.user_id = user.id.clone();
            existing.last_used = chrono::Utc::now();
            if let Some(ref h) = emby_header {
                existing.device_name = h.device.clone();
                existing.application_name = h.client.clone();
                existing.application_version = h.client_version.clone();
            }
            let _ = state.repo.upsert_access_token(&existing).await;
            existing
        } else {
            create_new_token(&state.repo, &user.id, emby_header.as_ref()).await?
        }
    } else {
        create_new_token(&state.repo, &user.id, emby_header.as_ref()).await?
    };

    let response = AuthenticateByNameResponse {
        access_token: access_token.token.clone(),
        session_info: make_session_info(&access_token, &user.username, &state.server_id),
        server_id: state.server_id.clone(),
        user: make_user(&user, &state.server_id),
    };

    Ok(Json(response))
}

/// POST /Users/AuthenticateWithQuickConnect
pub async fn authenticate_with_quick_connect(
    State(state): State<JellyfinAuthState>,
    headers: HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> axum::response::Response {
    use axum::response::IntoResponse;
    if !state.quick_connect {
        return StatusCode::FORBIDDEN.into_response();
    }

    let secret = match body.get("Secret").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => return StatusCode::BAD_REQUEST.into_response(),
    };

    let qc = match state.repo.get_quick_connect_by_secret(&secret).await {
        Ok(q) => q,
        Err(_) => return StatusCode::UNAUTHORIZED.into_response(),
    };

    if !qc.authorized || qc.user_id.is_empty() {
        return StatusCode::UNAUTHORIZED.into_response();
    }

    // Check not expired
    let age = chrono::Utc::now() - qc.created;
    if age.num_minutes() > 10 {
        return StatusCode::GONE.into_response();
    }

    let user = match state.repo.get_user_by_id(&qc.user_id).await {
        Ok(u) => u,
        Err(_) => return StatusCode::UNAUTHORIZED.into_response(),
    };

    let emby_header = parse_auth_header(&headers);
    let access_token = match create_new_token(&state.repo, &user.id, emby_header.as_ref()).await {
        Ok(t) => t,
        Err(s) => return s.into_response(),
    };

    let response = AuthenticateByNameResponse {
        access_token: access_token.token.clone(),
        session_info: make_session_info(&access_token, &user.username, &state.server_id),
        server_id: state.server_id.clone(),
        user: make_user(&user, &state.server_id),
    };

    Json(response).into_response()
}

/// GET /Users - Get all users
pub async fn users_all(
    Extension(token): Extension<model::AccessToken>,
    State(state): State<JellyfinState>,
) -> Result<Json<Vec<User>>, StatusCode> {
    let current = state
        .repo
        .get_user_by_id(&token.user_id)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    let all_users = state
        .repo
        .get_all_users()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let filtered: Vec<&model::User> = all_users
        .iter()
        .filter(|u| {
            // Admins see everyone; others only see non-hidden users
            current.properties.admin || !u.properties.is_hidden || u.id == current.id
        })
        .collect();

    let mut result = Vec::with_capacity(filtered.len());
    for u in filtered {
        result.push(make_user_full(&state, u).await);
    }

    Ok(Json(result))
}

/// GET /Users/Me - Get current user
pub async fn users_me(
    Extension(token): Extension<model::AccessToken>,
    State(state): State<JellyfinState>,
) -> Result<Json<User>, StatusCode> {
    let user: model::User = state
        .repo
        .get_user_by_id(&token.user_id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    Ok(Json(make_user_full(&state, &user).await))
}

/// GET /Users/{id} - Get user by ID
pub async fn users_by_id(
    Extension(token): Extension<model::AccessToken>,
    State(state): State<JellyfinState>,
    AxumPath(user_id): AxumPath<String>,
) -> Result<Json<User>, StatusCode> {
    let current = state
        .repo
        .get_user_by_id(&token.user_id)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // Allow: own profile, or admin accessing any user
    if user_id != token.user_id && !current.properties.admin {
        return Err(StatusCode::FORBIDDEN);
    }

    let user: model::User = state
        .repo
        .get_user_by_id(&user_id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    Ok(Json(make_user_full(&state, &user).await))
}

/// GET /Users/Public - Get public (non-hidden) users
pub async fn users_public(State(state): State<JellyfinState>) -> Json<Vec<User>> {
    match state.repo.get_all_users().await {
        Ok(users) => {
            let filtered: Vec<&model::User> = users
                .iter()
                .filter(|u| !u.properties.is_hidden && !u.properties.disabled)
                .collect();
            let mut result = Vec::with_capacity(filtered.len());
            for u in filtered {
                result.push(make_user_full(&state, u).await);
            }
            Json(result)
        }
        Err(_) => Json(Vec::new()),
    }
}

/// POST /Users - Update a user (name, userId query param)
pub async fn users_update(
    Extension(token): Extension<model::AccessToken>,
    State(state): State<JellyfinState>,
    Query(params): Query<std::collections::HashMap<String, String>>,
    Json(body): Json<serde_json::Value>,
) -> StatusCode {
    let target_id = params
        .get("userId")
        .cloned()
        .unwrap_or_else(|| token.user_id.clone());

    let current = match state.repo.get_user_by_id(&token.user_id).await {
        Ok(u) => u,
        Err(_) => return StatusCode::UNAUTHORIZED,
    };
    if target_id != token.user_id && !current.properties.admin {
        return StatusCode::FORBIDDEN;
    }

    let mut user = match state.repo.get_user_by_id(&target_id).await {
        Ok(u) => u,
        Err(_) => return StatusCode::NOT_FOUND,
    };

    if let Some(name) = body.get("Name").and_then(|v| v.as_str()) {
        user.username = name.to_string();
    }

    match state.repo.upsert_user(&user).await {
        Ok(_) => StatusCode::NO_CONTENT,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

/// POST /Users/New - Create a new user (admin only)
pub async fn users_new(
    Extension(token): Extension<model::AccessToken>,
    State(state): State<JellyfinState>,
    Json(body): Json<UserNewRequest>,
) -> Response {
    let current = match state.repo.get_user_by_id(&token.user_id).await {
        Ok(u) => u,
        Err(_) => return StatusCode::UNAUTHORIZED.into_response(),
    };
    if !current.properties.admin {
        return StatusCode::FORBIDDEN.into_response();
    }

    let hashed = match hash(&body.password, DEFAULT_COST) {
        Ok(h) => h,
        Err(e) => {
            error!("Failed to hash password: {}", e);
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let user = model::User {
        id: id_hash(&body.name),
        username: body.name.clone(),
        password: hashed,
        created: chrono::Utc::now(),
        last_login: chrono::Utc::now(),
        last_used: chrono::Utc::now(),
        properties: model::UserProperties::default(),
    };

    match state.repo.upsert_user(&user).await {
        Ok(_) => {
            // Auto-generate identicon avatar
            let png = generate_identicon(&user.id);
            if !png.is_empty() {
                let meta = ImageMetadata {
                    mime_type: "image/png".to_string(),
                    file_size: png.len() as i64,
                    etag: crate::idhash::hash_bytes(&png),
                    updated: chrono::Utc::now(),
                };
                let _ = state.repo.store_image(&user.id, "Primary", &meta, &png).await;
            }
            Json(make_user_full(&state, &user).await).into_response()
        }
        Err(e) => {
            error!("Failed to create user '{}': {}", body.name, e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

/// POST /Users/Password - Change password
pub async fn users_password(
    Extension(token): Extension<model::AccessToken>,
    State(state): State<JellyfinState>,
    Query(params): Query<std::collections::HashMap<String, String>>,
    Json(body): Json<UserPasswordRequest>,
) -> StatusCode {
    let target_id = params
        .get("userId")
        .cloned()
        .unwrap_or_else(|| token.user_id.clone());

    let current = match state.repo.get_user_by_id(&token.user_id).await {
        Ok(u) => u,
        Err(_) => return StatusCode::UNAUTHORIZED,
    };

    if target_id != token.user_id && !current.properties.admin {
        return StatusCode::FORBIDDEN;
    }

    let mut target = match state.repo.get_user_by_id(&target_id).await {
        Ok(u) => u,
        Err(_) => return StatusCode::NOT_FOUND,
    };

    // Verify current password if changing own password
    if target_id == token.user_id && !current.properties.admin {
        let valid = bcrypt::verify(&body.current_pw, &target.password).unwrap_or(false);
        if !valid {
            return StatusCode::FORBIDDEN;
        }
    }

    match hash(&body.new_pw, DEFAULT_COST) {
        Ok(hashed) => {
            target.password = hashed;
            if state.repo.upsert_user(&target).await.is_ok() {
                StatusCode::NO_CONTENT
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        }
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

/// DELETE /Users/{id} - Delete user (admin only)
pub async fn users_delete(
    Extension(token): Extension<model::AccessToken>,
    State(state): State<JellyfinState>,
    AxumPath(user_id): AxumPath<String>,
) -> StatusCode {
    let current = match state.repo.get_user_by_id(&token.user_id).await {
        Ok(u) => u,
        Err(_) => return StatusCode::UNAUTHORIZED,
    };
    if !current.properties.admin {
        return StatusCode::FORBIDDEN;
    }
    match state.repo.delete_user(&user_id).await {
        Ok(_) => StatusCode::NO_CONTENT,
        Err(_) => StatusCode::NOT_FOUND,
    }
}

/// POST /Users/{id}/Configuration - Update user configuration
pub async fn users_configuration_post(
    Extension(token): Extension<model::AccessToken>,
    State(state): State<JellyfinState>,
    AxumPath(user_id): AxumPath<String>,
    Json(body): Json<UserConfiguration>,
) -> StatusCode {
    let current = match state.repo.get_user_by_id(&token.user_id).await {
        Ok(u) => u,
        Err(_) => return StatusCode::UNAUTHORIZED,
    };
    if user_id != token.user_id && !current.properties.admin {
        return StatusCode::FORBIDDEN;
    }
    let mut user = match state.repo.get_user_by_id(&user_id).await {
        Ok(u) => u,
        Err(_) => return StatusCode::NOT_FOUND,
    };
    user.properties.ordered_views = body.ordered_views;
    user.properties.my_media_excludes = body.my_media_excludes;
    match state.repo.upsert_user(&user).await {
        Ok(_) => StatusCode::NO_CONTENT,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

/// POST /Users/{id}/Policy - Update user policy (admin only)
pub async fn users_policy_post(
    Extension(token): Extension<model::AccessToken>,
    State(state): State<JellyfinState>,
    AxumPath(user_id): AxumPath<String>,
    Json(body): Json<UserPolicy>,
) -> StatusCode {
    let current = match state.repo.get_user_by_id(&token.user_id).await {
        Ok(u) => u,
        Err(_) => return StatusCode::UNAUTHORIZED,
    };
    if !current.properties.admin {
        return StatusCode::FORBIDDEN;
    }
    let mut user = match state.repo.get_user_by_id(&user_id).await {
        Ok(u) => u,
        Err(_) => return StatusCode::NOT_FOUND,
    };
    user.properties.admin = body.is_administrator;
    user.properties.disabled = body.is_disabled;
    user.properties.is_hidden = body.is_hidden;
    user.properties.enable_downloads = body.enable_content_downloading;
    user.properties.enable_all_folders = body.enable_all_folders;
    user.properties.enabled_folders = body.enabled_folders;
    user.properties.allow_tags = body.allowed_tags;
    user.properties.block_tags = body.blocked_tags;
    match state.repo.upsert_user(&user).await {
        Ok(_) => StatusCode::NO_CONTENT,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

/// Helper: Make User from database model
pub fn make_user(user: &model::User, server_id: &str) -> User {
    let p = &user.properties;
    User {
        name: user.username.clone(),
        server_id: server_id.to_string(),
        id: user.id.clone(),
        has_password: true,
        has_configured_password: true,
        has_configured_easy_password: false,
        enable_auto_login: false,
        last_login_date: user.last_login,
        last_activity_date: user.last_used,
        configuration: UserConfiguration {
            ordered_views: p.ordered_views.clone(),
            my_media_excludes: p.my_media_excludes.clone(),
            ..UserConfiguration::default()
        },
        policy: UserPolicy {
            is_administrator: p.admin,
            is_hidden: p.is_hidden,
            is_disabled: p.disabled,
            enable_remote_access: true,
            enable_media_playback: true,
            enable_audio_playback_transcoding: true,
            enable_video_playback_transcoding: true,
            enable_content_deletion: p.admin,
            enable_content_downloading: p.enable_downloads,
            enable_all_devices: true,
            enable_all_folders: p.enable_all_folders,
            blocked_tags: p.block_tags.clone(),
            enabled_folders: p.enabled_folders.clone(),
            allowed_tags: p.allow_tags.clone(),
            enable_collection_management: p.admin,
            enable_subtitle_management: p.admin,
            enable_lyric_management: false,
            enable_user_preference_access: true,
            access_schedules: vec![],
            block_unrated_items: vec![],
            enable_remote_control_of_other_users: p.admin,
            enable_shared_device_control: false,
            enable_live_tv_management: false,
            enable_live_tv_access: false,
            enable_playback_remuxing: true,
            force_remote_source_transcoding: false,
            enable_content_deletion_from_folders: vec![],
            enable_sync_transcoding: false,
            enable_media_conversion: false,
            enabled_devices: vec![],
            enabled_channels: vec![],
            enable_all_channels: false,
            invalid_login_attempt_count: 0,
            login_attempts_before_lockout: 0,
            max_active_sessions: 0,
            enable_public_sharing: false,
            blocked_media_folders: vec![],
            blocked_channels: vec![],
            remote_client_bitrate_limit: 0,
            authentication_provider_id: "DefaultAuthenticationProvider".to_string(),
            password_reset_provider_id: "DefaultPasswordResetProvider".to_string(),
            sync_play_access: "CreateAndJoinGroups".to_string(),
        },
        primary_image_tag: None,
    }
}

/// Async version: checks DB for profile image and sets primary_image_tag.
pub async fn make_user_full(state: &JellyfinState, user: &model::User) -> User {
    let mut dto = make_user(user, &state.server_id);
    if state.repo.has_image(&user.id, "Primary").await.ok().flatten().is_some() {
        dto.primary_image_tag = Some(user.id.clone());
    }
    dto
}
