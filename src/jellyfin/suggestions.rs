use axum::{extract::State, response::Json, Extension};

use super::jellyfin::JellyfinState;
use super::types::*;
use crate::database::model::AccessToken;

/// GET /Users/{userId}/Items/Suggestions - Get item suggestions
/// GET /Items/Suggestions - Get item suggestions
pub async fn items_suggestions(
    Extension(_token): Extension<AccessToken>,
    State(_state): State<JellyfinState>,
) -> Json<UsersItemsSuggestionsResponse> {
    Json(UsersItemsSuggestionsResponse {
        items: Vec::new(),
        start_index: 0,
        total_record_count: 0,
    })
}
