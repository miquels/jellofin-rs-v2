use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    Extension,
};
use std::collections::HashMap;

use chrono::Utc;

use super::jellyfin::JellyfinState;
use super::types::*;
use crate::database::model::AccessToken;
use crate::idhash::*;

/// GET /Genres
pub async fn genres_all(
    Extension(_token): Extension<AccessToken>,
    State(state): State<JellyfinState>,
    Query(query_params): Query<HashMap<String, String>>,
) -> Json<UserItemsResponse> {
    let parent_id = query_params.get("parentId").cloned();

    let genres = if let Some(pid) = parent_id {
        if let Some(collection) = state.collections.get_collection(&pid) {
            collection.details().genres.clone()
        } else {
            Vec::new()
        }
    } else {
        state.collections.details().genres.clone()
    };

    let items: Vec<BaseItemDto> = genres.into_iter().map(|g| make_jfitem_genre(&state, &g)).collect();

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
        Ok(Json(make_jfitem_genre(&state, &name)))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

/// make_jfitem_genre creates a genre item.
fn make_jfitem_genre(state: &JellyfinState, genre: &str) -> BaseItemDto {
    let genre_id = id_hash_prefix(ITEM_PREFIX_GENRE, genre);

    // Try to get actual genre item count from collections
    let mut child_count = 1;
    for c in state.collections.get_collections() {
        let counts = c.genre_count();
        if let Some(&count) = counts.get(&genre_id) {
            child_count = count as i32;
        }
    }

    BaseItemDto {
        id: genre_id.clone(),
        server_id: state.server_id.clone(),
        item_type: "Genre".to_string(),
        name: genre.to_string(),
        sort_name: Some(genre.to_string()),
        etag: Some(genre_id),
        date_created: Some(Utc::now()),
        premiere_date: Some(Utc::now()),
        location_type: Some("FileSystem".to_string()),
        media_type: Some("Unknown".to_string()),
        child_count: Some(child_count),
        ..Default::default()
    }
}
