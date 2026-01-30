use super::jellyfin::{make_user, JellyfinState};
use super::types::*;
use crate::database::model;
use axum::{
    extract::{Path as AxumPath, Query, State},
    http::StatusCode,
    response::Json,
    Extension,
};

#[derive(serde::Deserialize)]
pub struct UserViewsQuery {
    #[serde(rename = "userId")]
    pub user_id: Option<String>,
}

/// GET /Users - Get all users (returns current user only)
pub async fn users_all(
    Extension(token): Extension<model::AccessToken>,
    State(state): State<JellyfinState>,
) -> Result<Json<Vec<User>>, StatusCode> {
    let user: model::User = state
        .repo
        .get_user_by_id(&token.user_id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    Ok(Json(vec![make_user(&user, &state.server_id)]))
}

/// GET /Users/Me - Get current user
pub async fn users_me(
    Extension(token): Extension<model::AccessToken>,
    State(state): State<JellyfinState>,
) -> Result<Json<User>, StatusCode> {
    let user: model::User = state
        .repo
        .get_user_by_id(&token.user_id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    Ok(Json(make_user(&user, &state.server_id)))
}

/// GET /Users/{id} - Get user by ID
pub async fn users_by_id(
    Extension(token): Extension<model::AccessToken>,
    State(state): State<JellyfinState>,
    AxumPath(user_id): AxumPath<String>,
) -> Result<Json<User>, StatusCode> {
    if user_id != token.user_id {
        return Err(StatusCode::NOT_FOUND);
    }

    let user: model::User = state
        .repo
        .get_user_by_id(&token.user_id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    Ok(Json(make_user(&user, &state.server_id)))
}

/// GET /Users/Public - Get public users (returns empty list)
pub async fn users_public() -> Json<Vec<User>> {
    Json(Vec::new())
}

/// GET /Users/{id}/Views - Get user views (libraries)
pub async fn user_views(
    Extension(_token): Extension<model::AccessToken>,
    State(state): State<JellyfinState>,
    AxumPath(_user_id): AxumPath<String>,
) -> Json<QueryResult<BaseItemDto>> {
    let collections = state.collections.get_collections();
    let mut items = Vec::new();

    for collection in collections {
        let mut dto = BaseItemDto::default();
        dto.name = collection.name.clone();
        dto.id = collection.id.clone();
        dto.server_id = state.server_id.clone();
        dto.item_type = "CollectionFolder".to_string();
        dto.collection_type = Some(collection.collection_type.as_str().to_string());

        // Map collection type to Jellyfin foldering
        let ctype = match collection.collection_type {
            crate::collection::CollectionType::Movies => "movies",
            crate::collection::CollectionType::Shows => "tvshows",
        };
        dto.collection_type = Some(ctype.to_string());

        items.push(dto);
    }

    Json(QueryResult {
        total_record_count: items.len() as i32,
        start_index: 0,
        items,
    })
}

/// GET /UserViews - Get user views (libraries) with query param
pub async fn user_views_query(
    Extension(_token): Extension<model::AccessToken>,
    State(state): State<JellyfinState>,
    Query(_query): Query<UserViewsQuery>,
) -> Json<QueryResult<BaseItemDto>> {
    let collections = state.collections.get_collections();
    let mut items = Vec::new();

    for collection in collections {
        let mut dto = BaseItemDto::default();
        dto.name = collection.name.clone();
        dto.id = collection.id.clone();
        dto.server_id = state.server_id.clone();
        dto.item_type = "CollectionFolder".to_string();
        dto.collection_type = Some(collection.collection_type.as_str().to_string());

        // Map collection type to Jellyfin foldering
        let ctype = match collection.collection_type {
            crate::collection::CollectionType::Movies => "movies",
            crate::collection::CollectionType::Shows => "tvshows",
        };
        dto.collection_type = Some(ctype.to_string());

        items.push(dto);
    }

    Json(QueryResult {
        total_record_count: items.len() as i32,
        start_index: 0,
        items,
    })
}
