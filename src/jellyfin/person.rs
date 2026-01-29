use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    Extension,
};
use urlencoding::decode;

use super::jellyfin::JellyfinState;
use super::types::*;
use crate::database::model::AccessToken;

/// GET /Persons
pub async fn persons_all(
    Extension(_token): Extension<AccessToken>,
    State(_state): State<JellyfinState>,
) -> Json<UserItemsResponse> {
    // Go implementation returns empty list
    Json(UserItemsResponse {
        items: Vec::new(),
        total_record_count: 0,
        start_index: 0,
    })
}

/// GET /Persons/{name}
pub async fn person_details(
    Extension(token): Extension<AccessToken>,
    State(state): State<JellyfinState>,
    Path(name): Path<String>,
) -> Result<Json<BaseItemDto>, StatusCode> {
    let decoded_name = decode(&name).map_err(|_| StatusCode::BAD_REQUEST)?;

    let db_person = state
        .repo
        .get_person(&decoded_name, &token.user_id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    Ok(Json(crate::jellyfin::make_jf_item_person(&db_person, &state.server_id)))
}
