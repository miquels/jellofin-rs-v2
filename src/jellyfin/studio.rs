use axum::{
    extract::{Path, Query, State},
    response::Json,
    Extension,
};
use std::collections::HashMap;

use super::jellyfin::JellyfinState;
use super::jfitem::*;
use super::types::*;
use crate::database::model::AccessToken;

/// GET /Studios
pub async fn studios_all(
    Extension(_token): Extension<AccessToken>,
    State(state): State<JellyfinState>,
    Query(query_params): Query<HashMap<String, String>>,
) -> Json<UserItemsResponse> {
    let parent_id = query_params.get("parentId").cloned();

    let studios = if let Some(pid) = parent_id {
        let internal_pid = trim_prefix(&pid);
        if let Some(collection) = state.collections.get_collection(internal_pid) {
            collection.details().studios.clone()
        } else {
            Vec::new()
        }
    } else {
        state.collections.details().studios.clone()
    };

    let items: Vec<BaseItemDto> = studios
        .into_iter()
        .map(|s| make_jf_item_studio(&s, &state.server_id))
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
    Json(make_jf_item_studio(&name, &state.server_id))
}
