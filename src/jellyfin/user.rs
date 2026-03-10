use super::jellyfin::JellyfinState;
use super::jfitem2::*;
use super::types::*;
use crate::database::model;
use axum::{
    extract::{Path as AxumPath, Query, State},
    http::StatusCode,
    response::Json,
    Extension,
};

#[derive(serde::Deserialize)]
pub struct UserViewsQuery {
    #[serde(rename = "userId")]
    pub user_id: Option<String>,
}

/// GET /Users - Get all users (returns current user only)
pub async fn users_all(
    Extension(token): Extension<model::AccessToken>,
    State(state): State<JellyfinState>,
) -> Result<Json<Vec<User>>, StatusCode> {
    let user: model::User = state
        .repo
        .get_user_by_id(&token.user_id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    Ok(Json(vec![make_user(&user, &state.server_id)]))
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

    Ok(Json(make_user(&user, &state.server_id)))
}

/// GET /Users/{id} - Get user by ID
pub async fn users_by_id(
    Extension(token): Extension<model::AccessToken>,
    State(state): State<JellyfinState>,
    AxumPath(user_id): AxumPath<String>,
) -> Result<Json<User>, StatusCode> {
    if user_id != token.user_id {
        return Err(StatusCode::NOT_FOUND);
    }

    let user: model::User = state
        .repo
        .get_user_by_id(&token.user_id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    Ok(Json(make_user(&user, &state.server_id)))
}

/// GET /Users/Public - Get public users (returns empty list)
pub async fn users_public() -> Json<Vec<User>> {
    Json(Vec::new())
}

/// GET /Users/{id}/Views - Get user views (libraries)
pub async fn user_views(
    Extension(token): Extension<model::AccessToken>,
    State(state): State<JellyfinState>,
    AxumPath(_user_id): AxumPath<String>,
) -> Result<Json<QueryResult<BaseItemDto>>, StatusCode> {
    let items = make_jfcollection_root_overview(&state, &token.user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(QueryResult {
        total_record_count: items.len() as i32,
        start_index: 0,
        items,
    }))
}

/// GET /UserViews - Get user views (libraries) with query param
pub async fn user_views_query(
    Extension(token): Extension<model::AccessToken>,
    State(state): State<JellyfinState>,
    Query(_query): Query<UserViewsQuery>,
) -> Result<Json<QueryResult<BaseItemDto>>, StatusCode> {
    let items = make_jfcollection_root_overview(&state, &token.user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(QueryResult {
        total_record_count: items.len() as i32,
        start_index: 0,
        items,
    }))
}

/// GET /Users/{id}/GroupingOptions - Get grouping options
pub async fn user_grouping_options(
    Extension(_token): Extension<model::AccessToken>,
    State(state): State<JellyfinState>,
    AxumPath(_user_id): AxumPath<String>,
) -> Result<Json<Vec<NameGuidPair>>, StatusCode> {
    let mut options = Vec::new();
    for c in state.collections.get_collections() {
        if let Ok(item) = make_jfitem_collection(&state, &c.id) {
            options.push(NameGuidPair {
                name: item.name,
                id: item.id,
            });
        }
    }
    Ok(Json(options))
}


/// Helper: Make User from database model
pub fn make_user(user: &model::User, server_id: &str) -> User {
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
        configuration: UserConfiguration::default(),
        policy: UserPolicy {
            is_administrator: true,
            is_hidden: false,
            is_disabled: false,
            enable_remote_access: true,
            enable_media_playback: true,
            enable_audio_playback_transcoding: true,
            enable_video_playback_transcoding: true,
            enable_content_deletion: false,
            enable_content_downloading: true,
            enable_all_devices: true,
            enable_all_folders: true,
            blocked_tags: vec![],
            enabled_folders: vec![],
            enable_collection_management: false,
            enable_subtitle_management: false,
            enable_lyric_management: false,
            allowed_tags: vec![],
            enable_user_preference_access: false,
            access_schedules: vec![],
            block_unrated_items: vec![],
            enable_remote_control_of_other_users: false,
            enable_shared_device_control: false,
            enable_live_tv_management: false,
            enable_live_tv_access: false,
            enable_playback_remuxing: false,
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
