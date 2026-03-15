use axum::{
    extract::{Path as AxumPath, Query, State},
    http::StatusCode,
    response::Json,
    Extension,
};
use std::collections::HashMap;

use super::jellyfin::JellyfinState;
use super::jfitem::*;
use super::types::*;
use super::util::item::{
    apply_query_item_pagination, apply_query_item_sorting, apply_query_items_filter,
};
use crate::collection::Item;
use crate::database::model;

/// GET /Shows/{id}/Episodes - Get episodes for a show
pub async fn show_episodes(
    Extension(token): Extension<model::AccessToken>,
    State(state): State<JellyfinState>,
    AxumPath(show_id): AxumPath<String>,
    Query(query_params): Query<HashMap<String, String>>,
) -> Result<Json<UserItemsResponse>, StatusCode> {
    // Get all episodes across all seasons as native Items
    let mut qitems =
        get_show_all_episodes(&state, &show_id).map_err(|_| StatusCode::NOT_FOUND)?;

    if needs_user_data(&query_params) {
        load_user_data(&mut qitems, &state, &token.user_id).await;
    }

    // Apply filtering (handles seasonId, includeItemTypes, etc.)
    let qitems = apply_query_items_filter(qitems, &query_params);
    let total_count = qitems.len() as i32;
    let mut qitems = qitems;
    apply_query_item_sorting(&mut qitems, &query_params);

    let items = convert_items_to_dtos(&qitems, &state, &token.user_id).await;

    Ok(Json(UserItemsResponse {
        items,
        start_index: 0,
        total_record_count: total_count,
    }))
}

/// GET /Shows/{id}/Seasons - Get seasons for a show
pub async fn show_seasons(
    Extension(token): Extension<model::AccessToken>,
    State(state): State<JellyfinState>,
    AxumPath(show_id): AxumPath<String>,
    Query(query_params): Query<HashMap<String, String>>,
) -> Result<Json<UserItemsResponse>, StatusCode> {
    let mut qitems =
        get_seasons_items(&state, &show_id).map_err(|_| StatusCode::NOT_FOUND)?;

    if needs_user_data(&query_params) {
        load_user_data(&mut qitems, &state, &token.user_id).await;
    }

    let qitems = apply_query_items_filter(qitems, &query_params);

    // Sort seasons by index number (specials/season 0 → index 99, end up last)
    let mut qitems = qitems;
    apply_query_item_sorting(&mut qitems, &HashMap::from([
        ("sortBy".to_string(), "IndexNumber".to_string()),
    ]));

    let total_count = qitems.len() as i32;
    let items = convert_items_to_dtos(&qitems, &state, &token.user_id).await;

    Ok(Json(UserItemsResponse {
        items,
        start_index: 0,
        total_record_count: total_count,
    }))
}

/// GET /Shows/NextUp - Get next up episodes
pub async fn shows_next_up(
    Extension(token): Extension<model::AccessToken>,
    State(state): State<JellyfinState>,
    Query(query_params): Query<HashMap<String, String>>,
) -> Json<QueryResult<BaseItemDto>> {
    let watched_episodes = state
        .repo
        .get_recently_watched(&token.user_id, true, 500)
        .await
        .unwrap_or_default();
    let next_up_ids = state.collections.next_up(&watched_episodes);

    let mut qitems: Vec<Item> = Vec::new();
    for id in next_up_ids {
        if let Some((_, _show, _season, episode)) = state.collections.get_episode_by_id(&id) {
            qitems.push(Item::Episode(episode));
        }
    }

    if needs_user_data(&query_params) {
        load_user_data(&mut qitems, &state, &token.user_id).await;
    }

    let qitems = apply_query_items_filter(qitems, &query_params);
    let total_count = qitems.len() as i32;
    let mut qitems = qitems;
    apply_query_item_sorting(&mut qitems, &query_params);
    let (qitems, start_index) = apply_query_item_pagination(qitems, &query_params);

    let items = convert_items_to_dtos(&qitems, &state, &token.user_id).await;

    Json(QueryResult {
        items,
        start_index,
        total_record_count: total_count,
    })
}
