use axum::{
    extract::{Path, Request, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use std::path::PathBuf;
use tower::ServiceExt;
use tower_http::services::ServeFile;
use tracing::warn;

use crate::jellyfin::{JellyfinState, UserItemsResponse};

/// Handlers for /Videos/{item}/stream and related routes
pub async fn video_stream_handler(
    State(state): State<JellyfinState>,
    Path(params): Path<std::collections::HashMap<String, String>>,
    req: Request,
) -> Response {
    let item_id = match params.get("item") {
        Some(id) => id,
        None => return StatusCode::BAD_REQUEST.into_response(),
    };

    // Remove any prefix from item_id if present (compatibility with Go behavior)
    let clean_id = item_id.trim_start_matches("item_"); // Example prefix, adjust if needed based on Go's trimPrefix

    // Look up item
    let (collection, item) = match state.collections.get_item_by_id(clean_id) {
        Some(res) => res,
        None => return StatusCode::NOT_FOUND.into_response(),
    };

    let filename = match &item {
        crate::collection::Item::Movie(m) => &m.file_name,
        crate::collection::Item::Show(s) => &s.file_name,
        crate::collection::Item::Season(_) => return StatusCode::NOT_FOUND.into_response(),
        crate::collection::Item::Episode(e) => &e.file_name,
    };
    
    let path_str = match &item {
         crate::collection::Item::Movie(m) => &m.path,
         crate::collection::Item::Show(s) => &s.path,
         crate::collection::Item::Season(_) => return StatusCode::NOT_FOUND.into_response(),
         crate::collection::Item::Episode(e) => &e.path,
    };

    if filename.is_empty() {
         return StatusCode::NOT_FOUND.into_response();
    }

    // Construct full path
    let mut full_path = PathBuf::from(&collection.directory);
    full_path.push(path_str);
    full_path.push(filename);

    if !full_path.exists() {
        warn!("Video file not found: {:?}", full_path);
        return StatusCode::NOT_FOUND.into_response();
    }

    // Serve file using tower-http ServeFile which handles Range requests etc.
    match ServeFile::new(full_path).oneshot(req).await {
        Ok(res) => res.into_response(),
        Err(e) => {
            warn!("Failed to serve video file: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn media_segments_handler() -> impl IntoResponse {
    let response = UserItemsResponse {
        items: vec![],
        total_record_count: 0,
        start_index: 0,
    };
    Json(response)
}
