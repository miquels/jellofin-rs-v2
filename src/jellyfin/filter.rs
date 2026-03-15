use axum::{
    extract::State,
    response::Json,
    Extension,
};

use super::jellyfin::JellyfinState;
use super::types::*;
use crate::database::model::AccessToken;
use crate::idhash::*;

/// GET /Users/{userId}/Items/Filters - Get item filters
pub async fn item_filters(
    Extension(_token): Extension<AccessToken>,
    State(state): State<JellyfinState>,
) -> Json<ItemFilterResponse> {
    let details = state.collections.details();
    Json(ItemFilterResponse {
        genres: details.genres,
        tags: details.tags,
        official_ratings: details.official_ratings,
        years: details.years,
    })
}

/// GET /Users/{userId}/Items/Filters2 - Get item filters version 2
pub async fn item_filters2(
    Extension(_token): Extension<AccessToken>,
    State(state): State<JellyfinState>,
) -> Json<ItemFilter2Response> {
    let details = state.collections.details();
    let genres = details
        .genres
        .into_iter()
        .map(|g| NameGuidPair {
            name: g.clone(),
            id: id_hash_prefix(ITEM_PREFIX_GENRE, &g),
        })
        .collect();

    Json(ItemFilter2Response {
        genres,
        tags: details.tags,
    })
}
