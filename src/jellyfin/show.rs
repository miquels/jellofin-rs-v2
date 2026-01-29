use axum::{
    extract::{Path as AxumPath, Query, State},
    http::StatusCode,
    response::Json,
    Extension,
};
use std::collections::HashMap;

use super::item::*;
use super::jellyfin::JellyfinState;
use super::jfitem::*;
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
    let season_id = query_params.get("seasonId").cloned();

    let mut items = if let Some(sid) = season_id {
        let internal_sid = trim_prefix(&sid);
        if let Some((_, show, season)) = state.collections.get_season_by_id(internal_sid) {
            make_jf_episodes_overview(&state, &token.user_id, &season, &show)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        } else {
            return Err(StatusCode::NOT_FOUND);
        }
    } else {
        if let Some((_, Item::Show(show))) = state.collections.get_item_by_id(internal_id) {
            let mut all_episodes = Vec::new();
            for season in &show.seasons {
                let episodes = make_jf_episodes_overview(&state, &token.user_id, season, &show)
                    .await
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                all_episodes.extend(episodes);
            }
            all_episodes
        } else {
            return Err(StatusCode::NOT_FOUND);
        }
    };

    // Jellyfin episodes endpoint has some default sorting
    items.sort_by(|a, b| {
        let s_res = a.parent_index_number.cmp(&b.parent_index_number);
        if s_res != std::cmp::Ordering::Equal {
            return s_res;
        }
        a.index_number.cmp(&b.index_number)
    });

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
) -> Result<Json<UserItemsResponse>, StatusCode> {
    let internal_id = trim_prefix(&show_id);
    if let Some((_, Item::Show(show))) = state.collections.get_item_by_id(internal_id) {
        let items = make_jf_seasons_overview(&state, &token.user_id, &show)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
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
        if let Some((_, show, _season, episode)) = state.collections.get_episode_by_id(&id) {
            let user_data = state.repo.get_user_data(&token.user_id, &id).await.ok();
            items.push(convert_episode_to_dto(
                &episode,
                &show,
                &state.server_id,
                user_data.as_ref(),
            ));
        }
    }

    let total_count = items.len() as i32;
    let limit = query_params
        .get("limit")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(items.len());
    if items.len() > limit {
        items.truncate(limit);
    }

    Json(QueryResult {
        items,
        start_index: 0,
        total_record_count: total_count,
    })
}
