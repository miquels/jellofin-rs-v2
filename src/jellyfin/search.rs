use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
    Extension,
};
use std::collections::HashMap;

use super::jellyfin::JellyfinState;
use super::jfitem::*;
use super::types::*;
use super::util::item::{apply_query_item_pagination, apply_query_item_sorting, apply_query_items_filter};
use crate::database::model;
use crate::idhash::{is_jf_collection_id, is_jf_collection_playlist_id};

/// GET /Search/Hints - Get search hints
pub async fn search_hints(
    Extension(token): Extension<model::AccessToken>,
    State(state): State<JellyfinState>,
    Query(query_params): Query<HashMap<String, String>>,
) -> Result<Json<SearchHintsResponse>, StatusCode> {
    if let Some(parent_id) = query_params.get("parentId") {
        if is_jf_collection_playlist_id(parent_id) {
            let qitems = get_playlist_overview_items(&state, &token.user_id).await;
            let items = convert_items_to_dtos(&qitems, &state, &token.user_id).await;
            return Ok(Json(SearchHintsResponse {
                search_hints: items,
                total_record_count: qitems.len() as i32,
            }));
        }
    }

    // Determine if we should scope search to a specific collection
    let search_collection_id = query_params.get("parentId").and_then(|pid| {
        if is_jf_collection_id(pid) {
            Some(pid.to_string())
        } else {
            None
        }
    });

    let mut qitems = if let Some(ref scid) = search_collection_id {
        get_items_by_collection(&state, scid).map_err(|_| StatusCode::NOT_FOUND)?
    } else {
        get_items_all(&state)
    };

    if needs_user_data(&query_params) {
        load_user_data(&mut qitems, &state, &token.user_id).await;
    }

    let qitems = apply_query_items_filter(qitems, &query_params);
    let total_count = qitems.len() as i32;
    let mut qitems = qitems;
    apply_query_item_sorting(&mut qitems, &query_params);
    let (qitems, _) = apply_query_item_pagination(qitems, &query_params);

    let items = convert_items_to_dtos(&qitems, &state, &token.user_id).await;

    Ok(Json(SearchHintsResponse {
        search_hints: items,
        total_record_count: total_count,
    }))
}
