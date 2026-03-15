use axum::{
    extract::{Path, Query, State},
    response::Json,
    Extension,
};
use std::collections::HashMap;

use chrono::Utc;

use super::jellyfin::JellyfinState;
use super::types::*;
use crate::database::model::AccessToken;
use crate::idhash::*;

/// GET /Studios
pub async fn studios_all(
    Extension(_token): Extension<AccessToken>,
    State(state): State<JellyfinState>,
    Query(query_params): Query<HashMap<String, String>>,
) -> Json<UserItemsResponse> {
    let parent_id = query_params.get("parentId").cloned();

    let studios = if let Some(pid) = parent_id {
        if let Some(collection) = state.collections.get_collection(&pid) {
            collection.details().studios.clone()
        } else {
            Vec::new()
        }
    } else {
        state.collections.details().studios.clone()
    };

    let items: Vec<BaseItemDto> = studios
        .into_iter()
        .map(|s| make_jfitem_studio(&state, &s))
        .collect();

    let total_count = items.len() as i32;

    Json(UserItemsResponse {
        items,
        total_record_count: total_count,
        start_index: 0,
    })
}

/// GET /Studios/{name}
pub async fn studio_details(
    Extension(_token): Extension<AccessToken>,
    State(state): State<JellyfinState>,
    Path(name): Path<String>,
) -> Json<BaseItemDto> {
    Json(make_jfitem_studio(&state, &name))
}

/// make_jfitem_studio creates a studio item.
pub fn make_jfitem_studio(state: &JellyfinState, studio: &str) -> BaseItemDto {
    let studio_id = id_hash_prefix(ITEM_PREFIX_STUDIO, studio);
    BaseItemDto {
        id: studio_id.clone(),
        server_id: state.server_id.clone(),
        item_type: "Studio".to_string(),
        name: studio.to_string(),
        sort_name: Some(studio.to_string()),
        etag: Some(studio_id),
        date_created: Some(Utc::now()),
        premiere_date: Some(Utc::now()),
        location_type: Some("FileSystem".to_string()),
        media_type: Some("Unknown".to_string()),
        user_data: Some(UserItemDataDto::default()),
        ..Default::default()
    }
}
