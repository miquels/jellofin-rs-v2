use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    Extension,
};
use std::collections::HashMap;

use super::jellyfin::JellyfinState;
use super::jfitem::*;
use super::types::*;
use crate::database::model::AccessToken;

/// GET /Genres
pub async fn genres_all(
    Extension(_token): Extension<AccessToken>,
    State(state): State<JellyfinState>,
    Query(query_params): Query<HashMap<String, String>>,
) -> Json<UserItemsResponse> {
    let parent_id = query_params.get("parentId").cloned();

    let genres = if let Some(pid) = parent_id {
        let internal_pid = trim_prefix(&pid);
        if let Some(collection) = state.collections.get_collection(internal_pid) {
            collection.details().genres.clone()
        } else {
            Vec::new()
        }
    } else {
        state.collections.details().genres.clone()
    };

    let items: Vec<BaseItemDto> = genres
        .into_iter()
        .map(|g| make_jf_item_genre(&g, &state.server_id))
        .collect();

    let total_count = items.len() as i32;

    Json(UserItemsResponse {
        items,
        total_record_count: total_count,
        start_index: 0,
    })
}

/// GET /Genres/{name}
pub async fn genre_details(
    Extension(_token): Extension<AccessToken>,
    State(state): State<JellyfinState>,
    Path(name): Path<String>,
) -> Result<Json<BaseItemDto>, StatusCode> {
    let genres = state.collections.details().genres;
    if genres.contains(&name) {
        Ok(Json(make_jf_item_genre(&name, &state.server_id)))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}
