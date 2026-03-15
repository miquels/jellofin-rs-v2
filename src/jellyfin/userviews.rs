use super::jellyfin::JellyfinState;
use super::jfitem::*;
use super::types::*;
use axum::{
    extract::{Path as AxumPath, Query, State},
    http::StatusCode,
    response::Json,
    Extension,
};
use crate::database::model;

#[derive(serde::Deserialize)]
pub struct UserViewsQuery {
    #[serde(rename = "userId")]
    pub user_id: Option<String>,
}

/// GET /Users/{id}/Views - Get user views (libraries)
pub async fn user_views(
    Extension(token): Extension<model::AccessToken>,
    State(state): State<JellyfinState>,
    AxumPath(_user_id): AxumPath<String>,
) -> Result<Json<QueryResult<BaseItemDto>>, StatusCode> {
    let items = make_jfcollection_root_overview(&state, &token.user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(QueryResult {
        total_record_count: items.len() as i32,
        start_index: 0,
        items,
    }))
}

/// GET /UserViews - Get user views (libraries) with query param
pub async fn user_views_query(
    Extension(token): Extension<model::AccessToken>,
    State(state): State<JellyfinState>,
    Query(_query): Query<UserViewsQuery>,
) -> Result<Json<QueryResult<BaseItemDto>>, StatusCode> {
    let items = make_jfcollection_root_overview(&state, &token.user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(QueryResult {
        total_record_count: items.len() as i32,
        start_index: 0,
        items,
    }))
}

/// GET /Users/{id}/GroupingOptions - Get grouping options
pub async fn user_grouping_options(
    Extension(_token): Extension<model::AccessToken>,
    State(state): State<JellyfinState>,
    AxumPath(_user_id): AxumPath<String>,
) -> Result<Json<Vec<NameGuidPair>>, StatusCode> {
    let mut options = Vec::new();
    for c in state.collections.get_collections() {
        if let Ok(item) = make_jfitem_collection(&state, &c.id) {
            options.push(NameGuidPair {
                name: item.name,
                id: item.id,
            });
        }
    }
    Ok(Json(options))
}
