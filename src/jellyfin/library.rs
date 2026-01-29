use axum::{extract::State, response::Json, Extension};

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
