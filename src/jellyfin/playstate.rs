use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
    Extension,
};
use chrono::Utc;
use tracing::{debug, error, info};

use super::jellyfin::JellyfinState;
use super::types::*;
use crate::database::{AccessToken, UserData as DbUserData};

const TICKS_TO_SECONDS: i64 = 10_000_000;

/// POST /Users/{user}/PlayedItems/{item}
/// POST /UserPlayedItems/{item}
pub async fn users_played_items_post(
    Extension(token): Extension<AccessToken>,
    State(state): State<JellyfinState>,
    Path(params): Path<(String, String)>,
) -> StatusCode {
    let item_id = &params.1;
    match user_data_update(&state, &token.user_id, item_id, 0, true).await {
        Ok(_) => StatusCode::OK,
        Err(e) => { error!("users_played_items_post: {}", e); StatusCode::INTERNAL_SERVER_ERROR }
    }
}

pub async fn users_played_items_post_simple(
    Extension(token): Extension<AccessToken>,
    State(state): State<JellyfinState>,
    Path(item_id): Path<String>,
) -> StatusCode {
    match user_data_update(&state, &token.user_id, &item_id, 0, true).await {
        Ok(_) => StatusCode::OK,
        Err(e) => { error!("users_played_items_post_simple: {}", e); StatusCode::INTERNAL_SERVER_ERROR }
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
    match user_data_update(&state, &token.user_id, item_id, 0, false).await {
        Ok(_) => StatusCode::OK,
        Err(e) => { error!("users_played_items_delete: {}", e); StatusCode::INTERNAL_SERVER_ERROR }
    }
}

pub async fn users_played_items_delete_simple(
    Extension(token): Extension<AccessToken>,
    State(state): State<JellyfinState>,
    Path(item_id): Path<String>,
) -> StatusCode {
    match user_data_update(&state, &token.user_id, &item_id, 0, false).await {
        Ok(_) => StatusCode::OK,
        Err(e) => { error!("users_played_items_delete_simple: {}", e); StatusCode::INTERNAL_SERVER_ERROR }
    }
}

/// POST /Sessions/Playing
pub async fn sessions_playing(
    Extension(token): Extension<AccessToken>,
    State(state): State<JellyfinState>,
    Json(req): Json<UpdatePlayStateRequest>,
) -> StatusCode {
    match user_data_update(&state, &token.user_id, &req.item_id, req.position_ticks, false).await {
        Ok(_) => StatusCode::NO_CONTENT,
        Err(e) => { error!("sessions_playing: {}", e); StatusCode::INTERNAL_SERVER_ERROR }
    }
}

/// POST /Sessions/Playing/Progress
pub async fn sessions_playing_progress(
    Extension(token): Extension<AccessToken>,
    State(state): State<JellyfinState>,
    Json(req): Json<UpdatePlayStateRequest>,
) -> StatusCode {
    match user_data_update(&state, &token.user_id, &req.item_id, req.position_ticks, false).await {
        Ok(_) => StatusCode::NO_CONTENT,
        Err(e) => { error!("sessions_playing_progress: {}", e); StatusCode::INTERNAL_SERVER_ERROR }
    }
}

/// POST /Sessions/Playing/Stopped
pub async fn sessions_playing_stopped(
    Extension(token): Extension<AccessToken>,
    State(state): State<JellyfinState>,
    Json(req): Json<UpdatePlayStateRequest>,
) -> StatusCode {
    match user_data_update(&state, &token.user_id, &req.item_id, req.position_ticks, false).await {
        Ok(_) => StatusCode::NO_CONTENT,
        Err(e) => { error!("sessions_playing_stopped: {}", e); StatusCode::INTERNAL_SERVER_ERROR }
    }
}

async fn user_data_update(
    state: &JellyfinState,
    user_id: &str,
    item_id: &str,
    position_ticks: i64,
    mark_as_watched: bool,
) -> anyhow::Result<()> {
    // Ignore updates with zero position unless explicitly marking as watched.
    // Clients send position_ticks=0 on abrupt stop, which would erase real progress.
    if position_ticks == 0 && !mark_as_watched {
        debug!("userDataUpdate: ignoring zero-position update for itemID: {}", item_id);
        return Ok(());
    }

    let mut duration = 0;

    if let Some((_, item)) = state.collections.get_item_by_id(&item_id) {
        duration = item.duration().map(|d| d.as_secs() as i64).unwrap_or(0);
    }

    // If we don't have a duration, assume 1 hour
    if duration == 0 {
        duration = 3600;
    }

    let mut playstate = state
        .repo
        .get_user_data(user_id, &item_id)
        .await
        .unwrap_or_else(|_| DbUserData {
            position: 0,
            played_percentage: 0,
            play_count: 0,
            favorite: false,
            played: false,
            timestamp: Utc::now(),
        });

    let position = position_ticks / TICKS_TO_SECONDS;
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

    state.repo.update_user_data(user_id, &item_id, &playstate).await?;
    Ok(())
}
