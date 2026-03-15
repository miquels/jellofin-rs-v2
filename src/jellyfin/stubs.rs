/// Unimplemented API endpoints.
///
/// These endpoints are not implemented yet.
///
/// For this reason, they are in this module instead of the module that they
/// would otherwise be in.
///
use axum::http::StatusCode;
use axum::{extract::State, response::IntoResponse, Extension, Json};

use super::types::*;
use crate::database::model::AccessToken;
use crate::jellyfin::JellyfinState;

//
// OpenApi tag: Branding.
//

/// GET /Branding/Configuration - Get branding configuration
pub async fn branding_configuration() -> Json<BrandingConfiguration> {
    Json(BrandingConfiguration {
        login_disclaimer: String::new(),
        custom_css: String::new(),
        splashscreen_enabled: false,
    })
}

/// GET /Branding/Css
/// GET /Branding/Css.css
pub async fn branding_css() -> &'static str {
    ""
}

//
// OpenApi tag: MediaSegments.
//

/// GET /MediaSegments - Get media segments (stub)
pub async fn media_segments_handler() -> impl IntoResponse {
    let response = UserItemsResponse {
        items: vec![],
        total_record_count: 0,
        start_index: 0,
    };
    Json(response)
}

//
// OpenApi tag: ItemRefresh.
//

/// POST /Items/{item}/Refresh - Queue item refresh (not implemented)
pub async fn items_refresh() -> StatusCode {
    StatusCode::NO_CONTENT
}

//
// OpenApi tag: ItemLookup.
//

/// GET /Items/{item}/RemoteImages - Get remote images (not implemented)
pub async fn items_remote_images() -> Json<ItemRemoteImagesResponse> {
    Json(ItemRemoteImagesResponse {
        images: Vec::new(),
        total_record_count: 0,
        providers: Vec::new(),
    })
}

//
// OpenApi tag: SyncPlay.
//

/// GET /SyncPlay/List - List SyncPlay groups (stub)
pub async fn sync_play_list() -> Json<Vec<serde_json::Value>> {
    Json(Vec::new())
}

/// POST /SyncPlay/New - Create SyncPlay group (not implemented)
pub async fn sync_play_new() -> StatusCode {
    StatusCode::UNAUTHORIZED
}

//
// OpenApi tag: Suggestions.
//

// GET /Users/{userId}/Items/Suggestions - Get item suggestions
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
