use axum::{
    extract::{Path as AxumPath, Query, State},
    http::StatusCode,
    response::Json,
    Extension,
};
use std::collections::HashMap;

use super::item::{apply_item_filter, apply_item_pagination, apply_item_sorting};
use super::jellyfin::JellyfinState;
use super::jfitem2::*;
use super::types::*;
use crate::collection::Item;
use crate::database::model;

/// GET /Shows/{id}/Episodes - Get episodes for a show
pub async fn show_episodes(
    Extension(token): Extension<model::AccessToken>,
    State(state): State<JellyfinState>,
    AxumPath(show_id): AxumPath<String>,
    Query(query_params): Query<HashMap<String, String>>,
) -> Result<Json<UserItemsResponse>, StatusCode> {
    let internal_id = trim_prefix(&show_id);

    let show = match state.collections.get_item_by_id(internal_id) {
        Some((_, Item::Show(s))) => s,
        _ => return Err(StatusCode::NOT_FOUND),
    };

    // Always fetch all episodes, filtering (e.g. seasonId) is handled by apply_item_filter
    let mut items = Vec::new();
    for season in &show.seasons {
        if let Ok(episodes) = make_jfitem_episodes_overview(&state, &token.user_id, season).await {
            items.extend(episodes);
        }
    }

    // Apply filtering (handles seasonId, includeItemTypes, etc.)
    items.retain(|i| apply_item_filter(i, &query_params));

    // Apply sorting
    items = apply_item_sorting(items, &query_params);

    let total_count = items.len() as i32;

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
    let internal_id = trim_prefix(&show_id);
    if let Some((_, Item::Show(show))) = state.collections.get_item_by_id(internal_id) {
        let mut items = make_jfitem_seasons_overview(&state, &token.user_id, &show)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        // Apply filtering
        items.retain(|i| apply_item_filter(i, &query_params));

        // Always sort seasons by index number (specials/season 99 end up last)
        items.sort_by(|a, b| a.index_number.cmp(&b.index_number));

        let total_count = items.len() as i32;
        Ok(Json(UserItemsResponse {
            items,
            start_index: 0,
            total_record_count: total_count,
        }))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

/// GET /Shows/NextUp - Get next up episodes
pub async fn shows_next_up(
    Extension(token): Extension<model::AccessToken>,
    State(state): State<JellyfinState>,
    Query(query_params): Query<HashMap<String, String>>,
) -> Json<QueryResult<BaseItemDto>> {
    let watched_episodes = state
        .repo
        .get_recently_watched(&token.user_id, true)
        .await
        .unwrap_or_default();
    let next_up_ids = state.collections.next_up(&watched_episodes);

    let mut items = Vec::new();
    for id in next_up_ids {
        if let Some((_, _show, _season, episode)) = state.collections.get_episode_by_id(&id) {
            if let Ok(dto) = make_jfitem_episode(&state, &token.user_id, &episode).await {
                if apply_item_filter(&dto, &query_params) {
                    items.push(dto);
                }
            }
        }
    }

    // Apply sorting
    items = apply_item_sorting(items, &query_params);

    let total_count = items.len() as i32;
    let (items, start_index) = apply_item_pagination(items, &query_params);

    Json(QueryResult {
        items,
        start_index,
        total_record_count: total_count,
    })
}
