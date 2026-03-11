use axum::{extract::State, http::StatusCode, response::Json, Extension};

use super::jellyfin::JellyfinState;
use super::types::*;
use crate::database::model::AccessToken;

/// GET /Library/VirtualFolders
/// Returns the available collections as virtual folders.
pub async fn library_virtual_folders(
    Extension(_token): Extension<AccessToken>,
    State(state): State<JellyfinState>,
) -> Json<Vec<MediaLibrary>> {
    let mut response = Vec::new();

    for collection in state.collections.get_collections() {
        response.push(MediaLibrary {
            name: collection.name.clone(),
            item_id: Some(collection.id.clone()),
            primary_image_item_id: Some(collection.id.clone()),
            collection_type: Some(collection.collection_type.as_str().to_string()),
            locations: Some(vec!["/".to_string()]),
            ..MediaLibrary::default()
        });
    }

    Json(response)
}

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
