use axum::{
    extract::{Path as AxumPath, Query, State},
    http::StatusCode,
    response::Json,
    Extension,
};
use std::collections::{HashMap, HashSet};
use tracing::warn;

use super::jellyfin::JellyfinState;
use super::jfitem::*;
use super::types::*;
use super::util::item::{
    apply_query_item_pagination, apply_query_item_sorting, apply_query_items_filter,
};
use crate::collection::Item;
use crate::database::model;
use crate::idhash::*;

/// GET /Items - Get list of items based upon provided query params
///
/// Unified pipeline: all item types (native + virtual) flow through:
///   get Items → load user_data → filter → sort → paginate → convert to DTO → fields filter
pub async fn items_query(
    Extension(token): Extension<model::AccessToken>,
    State(state): State<JellyfinState>,
    Query(query_params): Query<HashMap<String, String>>,
) -> Result<Json<UserItemsResponse>, StatusCode> {
    let parent_id = query_params.get("parentId").cloned();
    let recursive = query_params.get("recursive").map(|v| v == "true").unwrap_or(false);

    // Get native Items based on the request type
    let mut qitems = match &parent_id {
        None if recursive => get_items_all(&state),
        None => {
            // No parentId, not recursive → root overview
            get_root_overview_items(&state, &token.user_id).await
        }
        Some(pid) if is_jf_collection_favorites_id(pid) => {
            get_favorites_items(&state, &token.user_id).await
        }
        Some(pid) if is_jf_collection_playlist_id(pid) => {
            get_playlist_overview_items(&state, &token.user_id).await
        }
        Some(pid) if is_jf_playlist_id(pid) => {
            get_playlist_items_native(&state, &token.user_id, pid)
                .await
                .map_err(|_| StatusCode::NOT_FOUND)?
        }
        Some(pid) if is_jf_collection_id(pid) && !is_jf_root_id(pid) => {
            get_items_by_collection(&state, pid).map_err(|_| StatusCode::NOT_FOUND)?
        }
        Some(pid) if is_jf_genre_id(pid) => get_items_by_genre(&state, pid),
        Some(pid) if is_jf_studio_id(pid) => get_items_by_studio(&state, pid),
        Some(pid) => {
            // Check if parent_id is a show (→ seasons) or season (→ episodes)
            match state.collections.get_item_by_id(pid) {
                Some((_, Item::Show(_))) => {
                    get_seasons_items(&state, pid).map_err(|_| StatusCode::NOT_FOUND)?
                }
                Some((_, Item::Season(_))) => {
                    get_episodes_items(&state, pid).map_err(|_| StatusCode::NOT_FOUND)?
                }
                _ => {
                    warn!("items_query: unsupported parent_id {}", pid);
                    return Err(StatusCode::NOT_FOUND);
                }
            }
        }
    };

    // Load user_data only if filters/sorts need it
    if needs_user_data(&query_params) {
        load_user_data(&mut qitems, &state, &token.user_id).await;
    }

    let qitems = apply_query_items_filter(qitems, &query_params);
    let total_item_count = qitems.len() as i32;
    let mut qitems = qitems;
    apply_query_item_sorting(&mut qitems, &query_params);
    let (qitems, start_index) = apply_query_item_pagination(qitems, &query_params);

    // Convert only the final page to BaseItemDto
    let mut items = convert_items_to_dtos(&qitems, &state, &token.user_id).await;
    apply_fields_filter(&mut items, &query_params);

    Ok(Json(UserItemsResponse {
        items,
        start_index,
        total_record_count: total_item_count,
    }))
}

/// GET /Items/Resume - Get resume items
pub async fn items_resume(
    Extension(token): Extension<model::AccessToken>,
    State(state): State<JellyfinState>,
    Query(query_params): Query<HashMap<String, String>>,
) -> Result<Json<UsersItemsResumeResponse>, StatusCode> {
    let resume_ids = state
        .repo
        .get_recently_watched(
            &token.user_id,
            false,
            query_params.get("limit").and_then(|v| v.parse().ok()).unwrap_or(20),
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut qitems: Vec<Item> = Vec::new();
    for id in resume_ids {
        if let Some((_, item)) = state.collections.get_item_by_id(&id) {
            qitems.push(item);
        }
    }

    // Resume items always need user_data for display
    //
    // TODO: figure out if filtering and sorting is dependent on user_data.
    // TODO: if not, we can load user_data after filtering and sorting.
    load_user_data(&mut qitems, &state, &token.user_id).await;

    let qitems = apply_query_items_filter(qitems, &query_params);
    let total_count = qitems.len() as i32;
    let mut qitems = qitems;
    apply_query_item_sorting(&mut qitems, &query_params);
    let (qitems, start_index) = apply_query_item_pagination(qitems, &query_params);

    let items = convert_items_to_dtos(&qitems, &state, &token.user_id).await;

    Ok(Json(UsersItemsResumeResponse {
        items,
        start_index,
        total_record_count: total_count,
    }))
}

/// POST /Items/{item}/Refresh - Queue item refresh (not implemented)
pub async fn items_refresh() -> StatusCode {
    StatusCode::NO_CONTENT
}

/// GET /Items/{item}/RemoteImages - Get remote images (not implemented)
pub async fn items_remote_images() -> Json<ItemRemoteImagesResponse> {
    Json(ItemRemoteImagesResponse {
        images: Vec::new(),
        total_record_count: 0,
        providers: Vec::new(),
    })
}

/// GET /SyncPlay/List - List SyncPlay groups (stub)
pub async fn sync_play_list() -> Json<Vec<serde_json::Value>> {
    Json(Vec::new())
}

/// POST /SyncPlay/New - Create SyncPlay group (not implemented)
pub async fn sync_play_new() -> StatusCode {
    StatusCode::UNAUTHORIZED
}

/// GET /Users/{user}/Items/{item}/UserData
/// GET /UserItems/{item}/UserData
pub async fn users_item_userdata(
    Extension(token): Extension<model::AccessToken>,
    State(state): State<JellyfinState>,
    AxumPath(params): AxumPath<(String, String)>,
) -> Json<UserItemDataDto> {
    let item_id = &params.1;
    let playstate = state.repo.get_user_data(&token.user_id, item_id).await.ok();

    Json(make_jf_userdata(&token.user_id, item_id, playstate.as_ref()))
}

// Support for /UserItems/{item}/UserData which only has one path param
pub async fn users_item_userdata_simple(
    Extension(token): Extension<model::AccessToken>,
    State(state): State<JellyfinState>,
    AxumPath(item_id): AxumPath<String>,
) -> Json<UserItemDataDto> {
    let playstate = state.repo.get_user_data(&token.user_id, &item_id).await.ok();

    Json(make_jf_userdata(&token.user_id, &item_id, playstate.as_ref()))
}

// ---------------------------------------------------------------------------
// Fields filtering
// ---------------------------------------------------------------------------

/// Strips optional fields from items that were not requested via the `fields` query parameter.
/// Strip non-base fields from items, matching real Jellyfin behavior.
/// Always runs — optional fields are only included when explicitly requested
/// via the `fields` query parameter. Empty arrays are kept (not omitted) for
/// client compatibility.
fn apply_fields_filter(items: &mut Vec<BaseItemDto>, query_params: &HashMap<String, String>) {
    let fields: HashSet<&str> = query_params
        .get("fields")
        .map(|f| f.split(',').map(|s| s.trim()).collect())
        .unwrap_or_default();

    for item in items.iter_mut() {
        if !fields.contains("Overview") {
            item.overview = None;
        }
        if !fields.contains("Genres") {
            item.genres.clear();
            item.genre_items.clear();
        }
        if !fields.contains("Studios") {
            item.studios.clear();
        }
        if !fields.contains("People") {
            item.people.clear();
        }
        if !fields.contains("MediaSources") {
            item.media_sources.clear();
            item.media_streams.clear();
        }
        if !fields.contains("ProviderIds") {
            item.provider_ids.clear();
        }
        if !fields.contains("Tags") {
            item.tags.clear();
        }
        if !fields.contains("SortName") {
            item.sort_name = None;
            item.forced_sort_name = None;
        }
        if !fields.contains("DateCreated") {
            item.date_created = None;
        }
        if !fields.contains("Etag") {
            item.etag = None;
        }
        if !fields.contains("Path") {
            item.path = None;
        }
        if !fields.contains("Chapters") {
            item.chapters.clear();
        }
        if !fields.contains("ExternalUrls") {
            item.external_urls.clear();
        }
        if !fields.contains("Taglines") {
            item.taglines.clear();
        }
        if !fields.contains("ChildCount") {
            item.child_count = None;
        }
        if !fields.contains("RecursiveItemCount") {
            item.recursive_item_count = None;
        }
        if !fields.contains("ProductionLocations") {
            item.production_locations.clear();
        }
        if !fields.contains("OriginalTitle") {
            item.original_title = None;
        }
        if !fields.contains("CanDelete") {
            item.can_delete = None;
        }
        if !fields.contains("CanDownload") {
            item.can_download = None;
        }
        if !fields.contains("DisplayPreferencesId") {
            item.display_preferences_id = None;
        }
        if !fields.contains("ParentId") {
            item.parent_id = None;
        }
        if !fields.contains("PrimaryImageAspectRatio") {
            item.primary_image_aspect_ratio = None;
        }
        if !fields.contains("PlayAccess") {
            item.play_access = None;
        }
        if !fields.contains("EnableMediaSourceDisplay") {
            item.enable_media_source_display = None;
        }
        // Additional non-base fields
        if !fields.contains("PremiereDate") {
            item.premiere_date = None;
        }
        if !fields.contains("Width") {
            item.width = None;
        }
        if !fields.contains("Height") {
            item.height = None;
        }
        item.locked_fields.clear();
        item.critic_rating = None;
        item.lock_data = None;
    }
}

/// Collect items matching a genre ID across all collections.
fn get_items_by_genre(state: &JellyfinState, genre_id: &str) -> Vec<Item> {
    let mut items = Vec::new();
    for c in state.collections.get_collections() {
        for item in c.items {
            let matches = item
                .genres()
                .iter()
                .any(|g| id_hash_prefix(ITEM_PREFIX_GENRE, g) == genre_id);
            if matches {
                items.push(item);
            }
        }
    }
    items
}

/// Collect items matching a studio ID across all collections.
fn get_items_by_studio(state: &JellyfinState, studio_id: &str) -> Vec<Item> {
    let mut items = Vec::new();
    for c in state.collections.get_collections() {
        for item in c.items {
            let matches = item
                .studios()
                .iter()
                .any(|s| id_hash_prefix(ITEM_PREFIX_STUDIO, s) == studio_id);
            if matches {
                items.push(item);
            }
        }
    }
    items
}

