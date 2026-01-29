use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
    Extension,
};
use chrono::Utc;
use tracing::info;

use super::jellyfin::JellyfinState;
use super::jfitem::*;
use super::types::*;
use crate::database::{AccessToken, UserData as DbUserData};

const TICS_TO_SECONDS: i64 = 10_000_000;

/// GET /Users/{user}/Items/{item}/UserData
/// GET /UserItems/{item}/UserData
pub async fn users_item_userdata(
    Extension(token): Extension<AccessToken>,
    State(state): State<JellyfinState>,
    Path(params): Path<(String, String)>,
) -> Json<UserItemDataDto> {
    let _user_id = &params.0; // We use token.user_id for security
    let item_id = &params.1;
    let internal_id = trim_prefix(item_id);

    let playstate = state
        .repo
        .get_user_data(&token.user_id, internal_id)
        .await
        .unwrap_or_else(|_| DbUserData {
            position: 0,
            played_percentage: 0,
            play_count: 0,
            favorite: false,
            played: false,
            timestamp: Utc::now(),
        });

    Json(make_jf_user_data(&playstate, item_id))
}

// Support for /UserItems/{item}/UserData which only has one path param
pub async fn users_item_userdata_simple(
    Extension(token): Extension<AccessToken>,
    State(state): State<JellyfinState>,
    Path(item_id): Path<String>,
) -> Json<UserItemDataDto> {
    let internal_id = trim_prefix(&item_id);

    let playstate = state
        .repo
        .get_user_data(&token.user_id, internal_id)
        .await
        .unwrap_or_else(|_| DbUserData {
            position: 0,
            played_percentage: 0,
            play_count: 0,
            favorite: false,
            played: false,
            timestamp: Utc::now(),
        });

    Json(make_jf_user_data(&playstate, &item_id))
}

/// POST /Users/{user}/PlayedItems/{item}
/// POST /UserPlayedItems/{item}
pub async fn users_played_items_post(
    Extension(token): Extension<AccessToken>,
    State(state): State<JellyfinState>,
    Path(params): Path<(String, String)>,
) -> StatusCode {
    let item_id = &params.1;
    if user_data_update(&state, &token.user_id, item_id, 0, true).await.is_ok() {
        StatusCode::OK
    } else {
        StatusCode::INTERNAL_SERVER_ERROR
    }
}

pub async fn users_played_items_post_simple(
    Extension(token): Extension<AccessToken>,
    State(state): State<JellyfinState>,
    Path(item_id): Path<String>,
) -> StatusCode {
    if user_data_update(&state, &token.user_id, &item_id, 0, true)
        .await
        .is_ok()
    {
        StatusCode::OK
    } else {
        StatusCode::INTERNAL_SERVER_ERROR
    }
}

/// DELETE /Users/{user}/PlayedItems/{item}
/// DELETE /UserPlayedItems/{item}
pub async fn users_played_items_delete(
    Extension(token): Extension<AccessToken>,
    State(state): State<JellyfinState>,
    Path(params): Path<(String, String)>,
) -> StatusCode {
    let item_id = &params.1;
    if user_data_update(&state, &token.user_id, item_id, 0, false)
        .await
        .is_ok()
    {
        StatusCode::OK
    } else {
        StatusCode::INTERNAL_SERVER_ERROR
    }
}

pub async fn users_played_items_delete_simple(
    Extension(token): Extension<AccessToken>,
    State(state): State<JellyfinState>,
    Path(item_id): Path<String>,
) -> StatusCode {
    if user_data_update(&state, &token.user_id, &item_id, 0, false)
        .await
        .is_ok()
    {
        StatusCode::OK
    } else {
        StatusCode::INTERNAL_SERVER_ERROR
    }
}

/// POST /Sessions/Playing
pub async fn sessions_playing(
    Extension(token): Extension<AccessToken>,
    State(state): State<JellyfinState>,
    Json(req): Json<UpdatePlayStateRequest>,
) -> StatusCode {
    if user_data_update(&state, &token.user_id, &req.item_id, req.position_ticks, false)
        .await
        .is_ok()
    {
        StatusCode::NO_CONTENT
    } else {
        StatusCode::INTERNAL_SERVER_ERROR
    }
}

/// POST /Sessions/Playing/Progress
pub async fn sessions_playing_progress(
    Extension(token): Extension<AccessToken>,
    State(state): State<JellyfinState>,
    Json(req): Json<UpdatePlayStateRequest>,
) -> StatusCode {
    if user_data_update(&state, &token.user_id, &req.item_id, req.position_ticks, false)
        .await
        .is_ok()
    {
        StatusCode::NO_CONTENT
    } else {
        StatusCode::INTERNAL_SERVER_ERROR
    }
}

/// POST /Sessions/Playing/Stopped
pub async fn sessions_playing_stopped(
    Extension(token): Extension<AccessToken>,
    State(state): State<JellyfinState>,
    Json(req): Json<UpdatePlayStateRequest>,
) -> StatusCode {
    if user_data_update(&state, &token.user_id, &req.item_id, req.position_ticks, false)
        .await
        .is_ok()
    {
        StatusCode::NO_CONTENT
    } else {
        StatusCode::INTERNAL_SERVER_ERROR
    }
}

/// POST /UserFavoriteItems/{item}
/// POST /Users/{user}/FavoriteItems/{item}
pub async fn user_favorite_items_post(
    Extension(token): Extension<AccessToken>,
    State(state): State<JellyfinState>,
    Path(params): Path<(String, String)>,
) -> Result<Json<UserItemDataDto>, StatusCode> {
    let item_id = &params.1;
    let internal_id = trim_prefix(item_id);

    let mut playstate = state
        .repo
        .get_user_data(&token.user_id, internal_id)
        .await
        .unwrap_or_else(|_| DbUserData {
            position: 0,
            played_percentage: 0,
            play_count: 0,
            favorite: false,
            played: false,
            timestamp: Utc::now(),
        });

    playstate.favorite = true;

    if state
        .repo
        .update_user_data(&token.user_id, internal_id, &playstate)
        .await
        .is_ok()
    {
        Ok(Json(make_jf_user_data(&playstate, item_id)))
    } else {
        Err(StatusCode::INTERNAL_SERVER_ERROR)
    }
}

pub async fn user_favorite_items_post_simple(
    Extension(token): Extension<AccessToken>,
    State(state): State<JellyfinState>,
    Path(item_id): Path<String>,
) -> Result<Json<UserItemDataDto>, StatusCode> {
    let internal_id = trim_prefix(&item_id);

    let mut playstate = state
        .repo
        .get_user_data(&token.user_id, internal_id)
        .await
        .unwrap_or_else(|_| DbUserData {
            position: 0,
            played_percentage: 0,
            play_count: 0,
            favorite: false,
            played: false,
            timestamp: Utc::now(),
        });

    playstate.favorite = true;

    if state
        .repo
        .update_user_data(&token.user_id, internal_id, &playstate)
        .await
        .is_ok()
    {
        Ok(Json(make_jf_user_data(&playstate, &item_id)))
    } else {
        Err(StatusCode::INTERNAL_SERVER_ERROR)
    }
}

/// DELETE /UserFavoriteItems/{item}
/// DELETE /Users/{user}/FavoriteItems/{item}
pub async fn user_favorite_items_delete(
    Extension(token): Extension<AccessToken>,
    State(state): State<JellyfinState>,
    Path(params): Path<(String, String)>,
) -> Result<Json<UserItemDataDto>, StatusCode> {
    let item_id = &params.1;
    let internal_id = trim_prefix(item_id);

    let mut playstate = state
        .repo
        .get_user_data(&token.user_id, internal_id)
        .await
        .unwrap_or_else(|_| DbUserData {
            position: 0,
            played_percentage: 0,
            play_count: 0,
            favorite: false,
            played: false,
            timestamp: Utc::now(),
        });

    playstate.favorite = false;

    if state
        .repo
        .update_user_data(&token.user_id, internal_id, &playstate)
        .await
        .is_ok()
    {
        Ok(Json(make_jf_user_data(&playstate, item_id)))
    } else {
        Err(StatusCode::INTERNAL_SERVER_ERROR)
    }
}

pub async fn user_favorite_items_delete_simple(
    Extension(token): Extension<AccessToken>,
    State(state): State<JellyfinState>,
    Path(item_id): Path<String>,
) -> Result<Json<UserItemDataDto>, StatusCode> {
    let internal_id = trim_prefix(&item_id);

    let mut playstate = state
        .repo
        .get_user_data(&token.user_id, internal_id)
        .await
        .unwrap_or_else(|_| DbUserData {
            position: 0,
            played_percentage: 0,
            play_count: 0,
            favorite: false,
            played: false,
            timestamp: Utc::now(),
        });

    playstate.favorite = false;

    if state
        .repo
        .update_user_data(&token.user_id, internal_id, &playstate)
        .await
        .is_ok()
    {
        Ok(Json(make_jf_user_data(&playstate, &item_id)))
    } else {
        Err(StatusCode::INTERNAL_SERVER_ERROR)
    }
}

async fn user_data_update(
    state: &JellyfinState,
    user_id: &str,
    item_id: &str,
    position_ticks: i64,
    mark_as_watched: bool,
) -> anyhow::Result<()> {
    let internal_id = trim_prefix(item_id);
    let mut duration = 0;

    if let Some((_, item)) = state.collections.get_item_by_id(internal_id) {
        duration = item.duration().as_secs() as i64;
    }

    // If we don't have a duration, assume 1 hour
    if duration == 0 {
        duration = 3600;
    }

    let mut playstate = state
        .repo
        .get_user_data(user_id, internal_id)
        .await
        .unwrap_or_else(|_| DbUserData {
            position: 0,
            played_percentage: 0,
            play_count: 0,
            favorite: false,
            played: false,
            timestamp: Utc::now(),
        });

    let position = position_ticks / TICS_TO_SECONDS;
    let played_percentage = (100 * position / duration) as i32;

    info!(
        "userDataUpdate userID: {}, itemID: {}, Progress: {} sec, Duration: {} sec",
        user_id, item_id, position, duration
    );

    if mark_as_watched || played_percentage >= 98 {
        playstate.position = 0;
        playstate.played_percentage = 0;
        playstate.played = true;
    } else {
        playstate.position = position;
        playstate.played_percentage = played_percentage;
        playstate.played = false;
    }

    playstate.timestamp = Utc::now();

    state.repo.update_user_data(user_id, internal_id, &playstate).await?;
    Ok(())
}
