use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
    Extension,
};
use std::collections::HashMap;

use super::jellyfin::JellyfinState;
use super::jfitem::*;
use super::types::*;
use crate::database::model;

/// GET /Items/{item}/PlaybackInfo - Returns playback info including media sources
pub async fn items_playback_info(
    Extension(_token): Extension<model::AccessToken>,
    State(state): State<JellyfinState>,
    Path(item_id): Path<String>,
) -> Result<Json<PlaybackInfoResponse>, StatusCode> {
    let (_, item) = state
        .collections
        .get_item_by_id(&item_id)
        .ok_or(StatusCode::NOT_FOUND)?;

    use crate::collection::Item;
    let media_sources = match &item {
        Item::Movie(m) => make_media_source(&m.id, &m.file_name, m.file_size, &m.metadata),
        Item::Episode(e) => make_media_source(&e.id, &e.file_name, e.file_size, &e.metadata),
        _ => return Err(StatusCode::NOT_FOUND),
    };

    if media_sources.is_empty() {
        return Err(StatusCode::NOT_FOUND);
    }

    Ok(Json(PlaybackInfoResponse {
        media_sources,
        play_session_id: super::session::SESSION_ID.to_string(),
    }))
}

/// GET /Playback/BitrateTest
pub async fn playback_bitrate_test(Query(params): Query<HashMap<String, String>>) -> Response {
    let size = params
        .get("size")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(0)
        .min(20 * 1024 * 1024); // cap at 20 MB
    let data = vec![0u8; size];
    (
        [(axum::http::header::CONTENT_TYPE, "application/octet-stream")],
        data,
    )
        .into_response()
}
