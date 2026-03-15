use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    Extension,
};
use chrono::Utc;

use super::item::{apply_query_items_filter, apply_query_item_sorting, apply_query_item_pagination};
use super::jellyfin::JellyfinState;
use super::jfitem::*;
use super::types::*;
use crate::database::{AccessToken, UserData as DbUserData};

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
        get_items_by_collection(&state, pid)
            .map_err(|_| StatusCode::NOT_FOUND)?
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

/// DELETE /UserFavoriteItems/{item}
/// DELETE /Users/{user}/FavoriteItems/{item}
pub async fn user_favorite_items_delete(
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
