use axum::{
    extract::{Path as AxumPath, Query, State},
    http::StatusCode,
    response::Json,
    Extension,
};
use std::collections::HashMap;

use super::util::item::{apply_query_items_filter, apply_query_item_sorting};
use super::jellyfin::JellyfinState;
use super::jfitem::*;
use super::types::*;
use crate::database::model::AccessToken;

/// GET /Library/MediaFolders - Returns collections as media folders (same as VirtualFolders)
pub async fn library_media_folders(
    Extension(_token): Extension<AccessToken>,
    State(state): State<JellyfinState>,
) -> Json<UserItemsResponse> {
    // Re-use user_views logic: return collections as items
    let mut items = Vec::new();
    for collection in state.collections.get_collections() {
        items.push(BaseItemDto {
            id: collection.id.clone(),
            name: collection.name.clone(),
            collection_type: Some(collection.collection_type.as_str().to_string()),
            ..BaseItemDto::default()
        });
    }
    let count = items.len() as i32;
    Json(UserItemsResponse {
        items,
        total_record_count: count,
        start_index: 0,
    })
}

/// POST /Library/Refresh - Trigger library refresh (not implemented)
pub async fn library_refresh(Extension(_token): Extension<AccessToken>) -> StatusCode {
    StatusCode::NO_CONTENT
}

/// GET /Items/Counts - Get item counts
pub async fn items_counts(
    Extension(_token): Extension<AccessToken>,
    State(state): State<JellyfinState>,
) -> Json<ItemCountResponse> {
    let details = state.collections.details();

    Json(ItemCountResponse {
        movie_count: details.movie_count as i32,
        series_count: details.show_count as i32,
        episode_count: details.episode_count as i32,
        artist_count: 0,
        program_count: 0,
        trailer_count: 0,
        song_count: 0,
        album_count: 0,
        music_video_count: 0,
        box_set_count: 0,
        book_count: 0,
        item_count: (details.movie_count + details.show_count + details.episode_count) as i32,
    })
}

/// DELETE /Items/{item} - Not implemented, returns Forbidden
pub async fn items_delete(
    Extension(_token): Extension<AccessToken>,
    State(_state): State<JellyfinState>,
    AxumPath(_item_id): AxumPath<String>,
) -> StatusCode {
    StatusCode::FORBIDDEN
}

/// GET /Items/{item}/Ancestors - Get ancestors for an item
pub async fn item_ancestors(
    Extension(token): Extension<AccessToken>,
    State(state): State<JellyfinState>,
    AxumPath(item_id): AxumPath<String>,
) -> Result<Json<Vec<BaseItemDto>>, StatusCode> {
    let (collection, _) = state
        .collections
        .get_item_by_id(&item_id)
        .ok_or(StatusCode::NOT_FOUND)?;

    let collection_item =
        make_jfitem_collection(&state, &collection.id).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let root_item = make_jfitem_root(&state, &token.user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(vec![collection_item, root_item]))
}

/// GET /Items/{item}/ThemeMedia - Get theme media (not implemented)
pub async fn items_theme_media() -> Json<ItemThemeMediaResponse> {
    let empty = UserItemsResponse {
        items: Vec::new(),
        total_record_count: 0,
        start_index: 0,
    };
    Json(ItemThemeMediaResponse {
        theme_videos_result: empty.clone(),
        theme_songs_result: empty.clone(),
        soundtrack_songs_result: empty,
    })
}

/// GET /Items/{item}/Similar - Get similar items
pub async fn items_similar(
    Extension(token): Extension<AccessToken>,
    State(state): State<JellyfinState>,
    // Vec because this handler serves both /Items/{item_id}/Similar and /Users/{user_id}/Items/{item_id}/Similar.
    // Single-param route: path = [item_id]. Two-param route: path = [user_id, item_id].
    AxumPath(path): AxumPath<Vec<String>>,
    Query(query_params): Query<HashMap<String, String>>,
) -> Result<Json<UsersItemsSimilarResponse>, StatusCode> {
    let item_id = path.last().ok_or(StatusCode::BAD_REQUEST)?;
    let (collection, item) = state
        .collections
        .get_item_by_id(&item_id)
        .ok_or(StatusCode::NOT_FOUND)?;

    let limit = query_params
        .get("limit")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(10);
    let similar_ids = state.collections.similar(&collection.id, &item.id(), limit).await;

    let mut qitems: Vec<crate::collection::Item> = Vec::new();
    for id in similar_ids {
        if let Some((_, item)) = state.collections.get_item_by_id(&id) {
            qitems.push(item);
        }
    }

    if needs_user_data(&query_params) {
        load_user_data(&mut qitems, &state, &token.user_id).await;
    }

    let qitems = apply_query_items_filter(qitems, &query_params);
    let total_count = qitems.len() as i32;
    let mut qitems = qitems;
    apply_query_item_sorting(&mut qitems, &query_params);

    let items = convert_items_to_dtos(&qitems, &state, &token.user_id).await;

    Ok(Json(UsersItemsSimilarResponse {
        items,
        start_index: 0,
        total_record_count: total_count,
    }))
}
