use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    Extension,
};
use chrono::Utc;

use anyhow::anyhow;

use super::jellyfin::JellyfinState;
use super::jfitem::*;
use super::types::*;
use super::util::item::{apply_query_item_pagination, apply_query_item_sorting, apply_query_items_filter};
use crate::collection::Item;
use crate::database::{AccessToken, UserData as DbUserData};
use crate::idhash::*;

/// GET /Items/{item} - Gets an item from a user's library.
pub async fn item_details(
    Extension(token): Extension<AccessToken>,
    State(state): State<JellyfinState>,
    // Vec because this handler serves both /Items/{item_id} and /Users/{user_id}/Items/{item_id}.
    // Single-param route: path = [item_id]. Two-param route: path = [user_id, item_id].
    Path(path): Path<Vec<String>>,
) -> Result<Json<BaseItemDto>, StatusCode> {
    let item_id = path.last().ok_or(StatusCode::BAD_REQUEST)?;
    let response = make_jfitem_by_id(&state, &token.user_id, &item_id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    Ok(Json(response))
}

/// GET /Items/Root - Get root folder item
pub async fn items_root(
    Extension(token): Extension<AccessToken>,
    State(state): State<JellyfinState>,
) -> Result<Json<BaseItemDto>, StatusCode> {
    let item = make_jfitem_root(&state, &token.user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(item))
}

/// GET /Items/Latest - Get latest items
pub async fn items_latest(
    Extension(token): Extension<AccessToken>,
    State(state): State<JellyfinState>,
    axum::extract::Query(query_params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<Vec<BaseItemDto>>, StatusCode> {
    let parent_id = query_params.get("parentId").cloned();

    let mut qitems = if let Some(ref pid) = parent_id {
        get_items_by_collection(&state, pid).map_err(|_| StatusCode::NOT_FOUND)?
    } else {
        get_items_all(&state)
    };

    if needs_user_data(&query_params) {
        load_user_data(&mut qitems, &state, &token.user_id).await;
    }

    let qitems = apply_query_items_filter(qitems, &query_params);

    // Sort by premiere date descending
    let mut qitems = qitems;
    let mut sort_params = std::collections::HashMap::new();
    sort_params.insert("sortBy".to_string(), "PremiereDate".to_string());
    sort_params.insert("sortOrder".to_string(), "Descending".to_string());
    apply_query_item_sorting(&mut qitems, &sort_params);

    // Default limit to 50 for latest if not provided
    let mut qp = query_params.clone();
    if !qp.contains_key("limit") {
        qp.insert("limit".to_string(), "50".to_string());
    }

    let (qitems, _) = apply_query_item_pagination(qitems, &qp);
    let items = convert_items_to_dtos(&qitems, &state, &token.user_id).await;
    Ok(Json(items))
}

/// GET /Items/{item}/Intros - Get item intros (not implemented)
pub async fn items_intros() -> Json<UserItemsResponse> {
    Json(UserItemsResponse {
        items: Vec::new(),
        total_record_count: 0,
        start_index: 0,
    })
}

/// GET /Items/{item}/LocalTrailers - Get local trailers (not implemented)
pub async fn items_local_trailers() -> Json<Vec<BaseItemDto>> {
    Json(Vec::new())
}

/// GET /Items/{item}/SpecialFeatures - Returns empty list (not implemented)
pub async fn items_special_features(
    Extension(_token): Extension<AccessToken>,
    State(_state): State<JellyfinState>,
    Path(_item_id): Path<String>,
) -> Json<Vec<BaseItemDto>> {
    Json(Vec::new())
}

/// POST /UserFavoriteItems/{item}
/// POST /Users/{user}/FavoriteItems/{item}
pub async fn user_favorite_items_post(
    Extension(token): Extension<AccessToken>,
    State(state): State<JellyfinState>,
    Path(params): Path<(String, String)>,
) -> Result<Json<UserItemDataDto>, StatusCode> {
    let item_id = &params.1;
    let mut playstate = state
        .repo
        .get_user_data(&token.user_id, item_id)
        .await
        .unwrap_or_else(|_| DbUserData {
            position: 0,
            played_percentage: 0,
            play_count: 0,
            favorite: false,
            played: false,
            timestamp: Utc::now(),
        });

    playstate.favorite = true;

    if state
        .repo
        .update_user_data(&token.user_id, item_id, &playstate)
        .await
        .is_ok()
    {
        Ok(Json(make_jf_userdata(&token.user_id, item_id, Some(&playstate))))
    } else {
        Err(StatusCode::INTERNAL_SERVER_ERROR)
    }
}

/// POST /UserFavoriteItems/{item}
pub async fn user_favorite_items_post_simple(
    Extension(token): Extension<AccessToken>,
    State(state): State<JellyfinState>,
    Path(item_id): Path<String>,
) -> Result<Json<UserItemDataDto>, StatusCode> {
    let mut playstate = state
        .repo
        .get_user_data(&token.user_id, &item_id)
        .await
        .unwrap_or_else(|_| DbUserData {
            position: 0,
            played_percentage: 0,
            play_count: 0,
            favorite: false,
            played: false,
            timestamp: Utc::now(),
        });

    playstate.favorite = true;

    if state
        .repo
        .update_user_data(&token.user_id, &item_id, &playstate)
        .await
        .is_ok()
    {
        Ok(Json(make_jf_userdata(&token.user_id, &item_id, Some(&playstate))))
    } else {
        Err(StatusCode::INTERNAL_SERVER_ERROR)
    }
}

/// DELETE /Users/{user}/FavoriteItems/{item}
pub async fn user_favorite_items_delete(
    Extension(token): Extension<AccessToken>,
    State(state): State<JellyfinState>,
    Path(params): Path<(String, String)>,
) -> Result<Json<UserItemDataDto>, StatusCode> {
    let item_id = &params.1;

    // TODO: move this to src/database/
    let mut playstate = state
        .repo
        .get_user_data(&token.user_id, item_id)
        .await
        .unwrap_or_else(|_| DbUserData {
            position: 0,
            played_percentage: 0,
            play_count: 0,
            favorite: false,
            played: false,
            timestamp: Utc::now(),
        });

    playstate.favorite = false;

    if state
        .repo
        .update_user_data(&token.user_id, item_id, &playstate)
        .await
        .is_ok()
    {
        Ok(Json(make_jf_userdata(&token.user_id, item_id, Some(&playstate))))
    } else {
        Err(StatusCode::INTERNAL_SERVER_ERROR)
    }
}

/// DELETE /UserFavoriteItems/{item}
pub async fn user_favorite_items_delete_simple(
    Extension(token): Extension<AccessToken>,
    State(state): State<JellyfinState>,
    Path(item_id): Path<String>,
) -> Result<Json<UserItemDataDto>, StatusCode> {
    let mut playstate = state
        .repo
        .get_user_data(&token.user_id, &item_id)
        .await
        .unwrap_or_else(|_| DbUserData {
            position: 0,
            played_percentage: 0,
            play_count: 0,
            favorite: false,
            played: false,
            timestamp: Utc::now(),
        });

    playstate.favorite = false;

    if state
        .repo
        .update_user_data(&token.user_id, &item_id, &playstate)
        .await
        .is_ok()
    {
        Ok(Json(make_jf_userdata(&token.user_id, &item_id, Some(&playstate))))
    } else {
        Err(StatusCode::INTERNAL_SERVER_ERROR)
    }
}

// make_jfitem_by_id creates a BaseItemDto based on the provided item_id.
async fn make_jfitem_by_id(state: &JellyfinState, user_id: &str, item_id: &str) -> anyhow::Result<BaseItemDto> {
    use crate::collection::{CollectionFolder, PlaylistItem, UserView};

    // Handle special items first
    if is_jf_root_id(item_id) {
        return make_jfitem_root(state, user_id).await;
    }

    // Try special collection items — construct native Item, then convert
    if is_jf_collection_favorites_id(item_id) {
        let fav_count = state.repo.get_favorites(user_id).await.map(|f| f.len() as i32).ok();
        let item = Item::UserView(UserView {
            id: String::from(FAVORITES_COLLECTION_ID),
            name: "Favorites".to_string(),
            collection_type: "playlists".to_string(),
            child_count: fav_count,
        });
        return make_jfitem(state, user_id, &item).await;
    }
    if is_jf_collection_playlist_id(item_id) {
        let mut item_count = 0i32;
        if let Ok(playlist_ids) = state.repo.get_playlists(user_id).await {
            for id in &playlist_ids {
                if let Ok(playlist) = state.repo.get_playlist(user_id, id).await {
                    item_count += playlist.item_ids.len() as i32;
                }
            }
        }
        let item = Item::UserView(UserView {
            id: String::from(PLAYLIST_COLLECTION_ID),
            name: "Playlists".to_string(),
            collection_type: "playlists".to_string(),
            child_count: Some(item_count),
        });
        return make_jfitem(state, user_id, &item).await;
    }
    if is_jf_collection_id(item_id) {
        let c = state
            .collections
            .get_collection(item_id)
            .ok_or_else(|| anyhow!("collection not found"))?;
        let item = Item::CollectionFolder(CollectionFolder {
            id: c.id.clone(),
            name: c.name.clone(),
            collection_type: c.collection_type,
            child_count: c.items.len() as i32,
            genres: c.details().genres,
        });
        return make_jfitem(state, user_id, &item).await;
    }
    if is_jf_playlist_id(item_id) {
        let playlist = state.repo.get_playlist(user_id, item_id).await?;
        let item = Item::Playlist(PlaylistItem {
            id: playlist.id.clone(),
            name: playlist.name.clone(),
            child_count: playlist.item_ids.len() as i32,
        });
        return make_jfitem(state, user_id, &item).await;
    }

    // Try to fetch individual item: movie, show, season, episode
    let (_, item) = state
        .collections
        .get_item_by_id(item_id)
        .ok_or_else(|| anyhow!("item not found"))?;
    make_jfitem(state, user_id, &item).await
}
