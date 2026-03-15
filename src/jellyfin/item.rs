use axum::{
    extract::{Path as AxumPath, Query, State},
    http::StatusCode,
    response::Json,
    Extension,
};
use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use rand::prelude::*;
use std::collections::{HashMap, HashSet};
use tracing::warn;

use super::jellyfin::JellyfinState;
use super::jfitem::*;
use super::types::*;
use crate::database::model;
use crate::idhash::*;

/// GET /Items - Get list of items based upon provided query params
pub async fn items_query(
    Extension(token): Extension<model::AccessToken>,
    State(state): State<JellyfinState>,
    Query(query_params): Query<HashMap<String, String>>,
) -> Result<Json<UserItemsResponse>, StatusCode> {
    let parent_id = query_params.get("parentId").cloned();
    let recursive = query_params.get("recursive").map(|v| v == "true").unwrap_or(false);

    // Determine if this request can use the QueryItem pipeline (native items)
    // or must fall back to the DTO path (virtual/hierarchical items).
    let use_query_pipeline = match &parent_id {
        None if recursive => true,
        Some(pid) if is_jf_collection_id(pid)
            && !is_jf_root_id(pid)
            && !is_jf_collection_favorites_id(pid)
            && !is_jf_collection_playlist_id(pid) => true,
        Some(pid) if is_jf_genre_id(pid) => true,
        Some(pid) if is_jf_studio_id(pid) => true,
        _ => false,
    };

    if use_query_pipeline {
        // --- QueryItem pipeline: filter/sort/paginate on native types, convert only the page ---
        let mut qitems = match &parent_id {
            None => get_query_items_all(&state),
            Some(pid) if is_jf_collection_id(pid) => {
                get_query_items_by_collection(&state, pid)
                    .map_err(|_| StatusCode::NOT_FOUND)?
            }
            Some(pid) if is_jf_genre_id(pid) => get_query_items_by_genre(&state, pid),
            Some(pid) if is_jf_studio_id(pid) => get_query_items_by_studio(&state, pid),
            _ => unreachable!(),
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
        let mut items = convert_query_items_to_dtos(&qitems, &state, &token.user_id).await;
        apply_fields_filter(&mut items, &query_params);

        Ok(Json(UserItemsResponse {
            items,
            start_index,
            total_record_count: total_item_count,
        }))
    } else {
        // --- DTO path: virtual items, hierarchical browsing (shows→seasons, seasons→episodes) ---
        let items = if let Some(ref pid) = parent_id {
            get_jfitems_by_parent_id(&state, &token.user_id, pid)
                .await
                .map_err(|_| StatusCode::NOT_FOUND)?
        } else {
            // !recursive, no parentId → root overview
            make_jfcollection_root_overview(&state, &token.user_id)
                .await
                .map_err(|_| StatusCode::NOT_FOUND)?
        };

        let items = apply_items_filter(items, &query_params);
        let total_item_count = items.len() as i32;
        let sorted_items = apply_item_sorting(items, &query_params);
        let (mut paged_items, start_index) = apply_item_pagination(sorted_items, &query_params);
        apply_fields_filter(&mut paged_items, &query_params);

        Ok(Json(UserItemsResponse {
            items: paged_items,
            start_index,
            total_record_count: total_item_count,
        }))
    }
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

    // Collect as QueryItems
    let mut qitems: Vec<QueryItem> = Vec::new();
    for id in resume_ids {
        if let Some((c, item)) = state.collections.get_item_by_id(&id) {
            qitems.push(QueryItem {
                item,
                collection_id: c.id.clone(),
                user_data: None,
            });
        }
    }

    // Resume items always need user_data for display
    load_user_data(&mut qitems, &state, &token.user_id).await;

    let qitems = apply_query_items_filter(qitems, &query_params);
    let total_count = qitems.len() as i32;
    let mut qitems = qitems;
    apply_query_item_sorting(&mut qitems, &query_params);
    let (qitems, start_index) = apply_query_item_pagination(qitems, &query_params);

    let items = convert_query_items_to_dtos(&qitems, &state, &token.user_id).await;

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
    let playstate = state
        .repo
        .get_user_data(&token.user_id, item_id)
        .await
        .ok();

    Json(make_jf_userdata(&token.user_id, item_id, playstate.as_ref()))
}

// Support for /UserItems/{item}/UserData which only has one path param
pub async fn users_item_userdata_simple(
    Extension(token): Extension<model::AccessToken>,
    State(state): State<JellyfinState>,
    AxumPath(item_id): AxumPath<String>,
) -> Json<UserItemDataDto> {
    let playstate = state
        .repo
        .get_user_data(&token.user_id, &item_id)
        .await
        .ok();

    Json(make_jf_userdata(&token.user_id, &item_id, playstate.as_ref()))
}

// ---------------------------------------------------------------------------
// Filtering
// ---------------------------------------------------------------------------

pub(crate) fn apply_items_filter(items: Vec<BaseItemDto>, query_params: &HashMap<String, String>) -> Vec<BaseItemDto> {
    items
        .into_iter()
        .filter(|i| apply_item_filter(i, query_params))
        .collect()
}

pub(super) fn apply_item_filter(i: &BaseItemDto, qp: &HashMap<String, String>) -> bool {
    // includeItemTypes
    if let Some(types) = qp.get("includeItemTypes") {
        let type_list: Vec<&str> = types.split(',').collect();
        if !type_list.contains(&i.item_type.as_str()) {
            return false;
        }
    }

    // excludeItemTypes
    if let Some(types) = qp.get("excludeItemTypes") {
        let type_list: Vec<&str> = types.split(',').collect();
        if type_list.contains(&i.item_type.as_str()) {
            return false;
        }
    }

    // isHd
    if let Some(hd) = qp.get("isHd") {
        let want_hd = hd.eq_ignore_ascii_case("true");
        if i.is_hd != want_hd {
            return false;
        }
    }

    // is4K
    if let Some(k4) = qp.get("is4K") {
        let want_4k = k4.eq_ignore_ascii_case("true");
        if i.is_4k != want_4k {
            return false;
        }
    }

    // ids
    if let Some(ids) = qp.get("ids") {
        let id_list: Vec<&str> = ids.split(',').collect();
        if !id_list.contains(&i.id.as_str()) {
            return false;
        }
    }

    // excludeItemIds
    if let Some(exclude_ids) = qp.get("excludeItemIds") {
        for eid in exclude_ids.split(',') {
            if i.id == eid {
                return false;
            }
        }
    }

    // genreIds (pipe-separated)
    if let Some(genre_ids) = qp.get("genreIds") {
        let mut keep = false;
        for gid in genre_ids.split('|') {
            for genre_item in &i.genre_items {
                if genre_item.id == gid {
                    keep = true;
                }
            }
        }
        if !keep {
            return false;
        }
    }

    // studioIds (pipe-separated)
    if let Some(studio_ids) = qp.get("studioIds") {
        let mut keep = false;
        for sid in studio_ids.split('|') {
            for studio in &i.studios {
                if studio.id == sid {
                    keep = true;
                }
            }
        }
        if !keep {
            return false;
        }
    }

    // seriesId
    if let Some(series_id) = qp.get("seriesId") {
        if i.series_id.as_deref() != Some(series_id.as_str()) {
            return false;
        }
    }

    // seasonId
    if let Some(season_id) = qp.get("seasonId") {
        if i.season_id.as_deref() != Some(season_id.as_str()) {
            return false;
        }
    }

    // parentIndexNumber
    if let Some(pin_str) = qp.get("parentIndexNumber") {
        if let Ok(pin) = pin_str.parse::<i32>() {
            if i.parent_index_number != Some(pin) {
                return false;
            }
        }
    }

    // indexNumber
    if let Some(in_str) = qp.get("indexNumber") {
        if let Ok(idx) = in_str.parse::<i32>() {
            if i.index_number != Some(idx) {
                return false;
            }
        }
    }

    // nameStartsWith (case-insensitive)
    if let Some(prefix) = qp.get("nameStartsWith") {
        let sort = i.sort_name.as_deref().unwrap_or(&i.name);
        if !sort.to_lowercase().starts_with(&prefix.to_lowercase()) {
            return false;
        }
    }

    // nameStartsWithOrGreater (case-insensitive)
    if let Some(bound) = qp.get("nameStartsWithOrGreater") {
        let sort = i.sort_name.as_deref().unwrap_or(&i.name);
        if sort.to_lowercase() < bound.to_lowercase() {
            return false;
        }
    }

    // nameLessThan (case-insensitive)
    if let Some(bound) = qp.get("nameLessThan") {
        let sort = i.sort_name.as_deref().unwrap_or(&i.name);
        if sort.to_lowercase() > bound.to_lowercase() {
            return false;
        }
    }

    // genres (by name, pipe-separated)
    if let Some(include_genres) = qp.get("genres") {
        let mut keep = false;
        for g in include_genres.split('|') {
            if i.genres.contains(&g.to_string()) {
                keep = true;
            }
        }
        if !keep {
            return false;
        }
    }

    // studios (by name, pipe-separated)
    if let Some(include_studios) = qp.get("studios") {
        let mut keep = false;
        for s in include_studios.split('|') {
            for studio in &i.studios {
                if studio.name == s {
                    keep = true;
                }
            }
        }
        if !keep {
            return false;
        }
    }

    // officialRatings (pipe-separated)
    if let Some(ratings) = qp.get("officialRatings") {
        let mut keep = false;
        for r in ratings.split('|') {
            if i.official_rating.as_deref() == Some(r) {
                keep = true;
            }
        }
        if !keep {
            return false;
        }
    }

    // minCommunityRating
    if let Some(min_str) = qp.get("minCommunityRating") {
        if let Ok(min) = min_str.parse::<f32>() {
            if i.community_rating.unwrap_or(0.0) < min {
                return false;
            }
        }
    }

    // minCriticRating
    if let Some(min_str) = qp.get("minCriticRating") {
        if let Ok(min) = min_str.parse::<f32>() {
            if i.critic_rating.unwrap_or(0.0) < min {
                return false;
            }
        }
    }

    // minPremiereDate
    if let Some(date_str) = qp.get("minPremiereDate") {
        if let Some(min_date) = parse_iso8601_date(date_str) {
            if let Some(ref pd) = i.premiere_date {
                if *pd < min_date {
                    return false;
                }
            } else {
                return false;
            }
        }
    }

    // maxPremiereDate
    if let Some(date_str) = qp.get("maxPremiereDate") {
        if let Some(max_date) = parse_iso8601_date(date_str) {
            if let Some(ref pd) = i.premiere_date {
                if *pd > max_date {
                    return false;
                }
            } else {
                return false;
            }
        }
    }

    // years (comma-separated)
    if let Some(years_str) = qp.get("years") {
        let mut keep = false;
        for y in years_str.split(',') {
            if let Ok(year) = y.parse::<i32>() {
                if i.production_year == Some(year) {
                    keep = true;
                }
            }
        }
        if !keep {
            return false;
        }
    }

    // isPlayed
    if let Some(played_str) = qp.get("isPlayed") {
        let want_played = played_str.eq_ignore_ascii_case("true");
        let is_played = i.user_data.as_ref().map(|ud| ud.played).unwrap_or(false);
        if want_played != is_played {
            return false;
        }
    }

    // isFavorite
    if let Some(fav_str) = qp.get("isFavorite") {
        let want_fav = fav_str.eq_ignore_ascii_case("true");
        let is_fav = i.user_data.as_ref().map(|ud| ud.is_favorite).unwrap_or(false);
        if want_fav != is_fav {
            return false;
        }
    }

    // filters (comma-separated, e.g. "IsFavorite", "IsFavoriteOrLikes")
    if let Some(filters) = qp.get("filters") {
        for f in filters.split(',') {
            match f {
                "IsFavorite" | "IsFavoriteOrLikes" => {
                    let is_fav = i.user_data.as_ref().map(|ud| ud.is_favorite).unwrap_or(false);
                    if !is_fav {
                        return false;
                    }
                }
                _ => {}
            }
        }
    }

    // searchTerm
    if let Some(term) = qp.get("searchTerm") {
        let media = is_jf_movie_id(&i.id) || is_jf_show_id(&i.id) || is_jf_season_id(&i.id) || is_jf_episode_id(&i.id);
        if media && !i.name.to_lowercase().contains(&term.to_lowercase()) {
            return false;
        }
    }

    true
}

// ---------------------------------------------------------------------------
// Sorting
// ---------------------------------------------------------------------------

pub(crate) fn apply_item_sorting(
    mut items: Vec<BaseItemDto>,
    query_params: &HashMap<String, String>,
) -> Vec<BaseItemDto> {
    let sort_by_raw = match query_params.get("sortBy") {
        Some(s) if !s.is_empty() => s.clone(),
        _ => return items,
    };
    let sort_fields: Vec<String> = sort_by_raw.split(',').map(|s| s.to_lowercase()).collect();

    let descending = query_params
        .get("sortOrder")
        .map(|s| s.eq_ignore_ascii_case("descending"))
        .unwrap_or(false);

    items.sort_by(|a, b| {
        let a_sort = a.sort_name.as_deref().unwrap_or(&a.name);
        let b_sort = b.sort_name.as_deref().unwrap_or(&b.name);

        for field in &sort_fields {
            let ord = match field.as_str() {
                "communityrating" => {
                    let ar = a.community_rating.unwrap_or(0.0);
                    let br = b.community_rating.unwrap_or(0.0);
                    ar.partial_cmp(&br).unwrap_or(std::cmp::Ordering::Equal)
                }
                "criticrating" => {
                    let ar = a.critic_rating.unwrap_or(0.0);
                    let br = b.critic_rating.unwrap_or(0.0);
                    ar.partial_cmp(&br).unwrap_or(std::cmp::Ordering::Equal)
                }
                "datecreated" | "datelastcontentadded" => a.date_created.cmp(&b.date_created),
                "dateplayed" => {
                    let ad = a.user_data.as_ref().and_then(|ud| ud.last_played_date);
                    let bd = b.user_data.as_ref().and_then(|ud| ud.last_played_date);
                    ad.cmp(&bd)
                }
                "indexnumber" => a.index_number.cmp(&b.index_number),
                "isfavoriteorliked" => {
                    let af = a.user_data.as_ref().map(|ud| ud.is_favorite).unwrap_or(false);
                    let bf = b.user_data.as_ref().map(|ud| ud.is_favorite).unwrap_or(false);
                    af.cmp(&bf)
                }
                "isfolder" => a.is_folder.cmp(&b.is_folder),
                "isplayed" => {
                    let ap = a.user_data.as_ref().map(|ud| ud.played).unwrap_or(false);
                    let bp = b.user_data.as_ref().map(|ud| ud.played).unwrap_or(false);
                    ap.cmp(&bp)
                }
                "isunplayed" => {
                    let ap = !a.user_data.as_ref().map(|ud| ud.played).unwrap_or(false);
                    let bp = !b.user_data.as_ref().map(|ud| ud.played).unwrap_or(false);
                    ap.cmp(&bp)
                }
                "officialrating" => a.official_rating.cmp(&b.official_rating),
                "parentindexnumber" => a.parent_index_number.cmp(&b.parent_index_number),
                "premieredate" => a.premiere_date.cmp(&b.premiere_date),
                "productionyear" => a.production_year.cmp(&b.production_year),
                "random" => {
                    let mut rng = rand::thread_rng();
                    if rng.gen_bool(0.5) {
                        std::cmp::Ordering::Less
                    } else {
                        std::cmp::Ordering::Greater
                    }
                }
                "runtime" => a.run_time_ticks.cmp(&b.run_time_ticks),
                "name" | "seriessortname" | "sortname" | "default" => a_sort.cmp(b_sort),
                other => {
                    warn!("apply_item_sorting: unknown sort field: {}", other);
                    std::cmp::Ordering::Equal
                }
            };
            if ord != std::cmp::Ordering::Equal {
                return if descending { ord.reverse() } else { ord };
            }
        }
        std::cmp::Ordering::Equal
    });
    items
}

// ---------------------------------------------------------------------------
// Pagination
// ---------------------------------------------------------------------------

pub(crate) fn apply_item_pagination(
    items: Vec<BaseItemDto>,
    query_params: &HashMap<String, String>,
) -> (Vec<BaseItemDto>, i32) {
    let start_index = query_params
        .get("startIndex")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(0);
    let limit = query_params.get("limit").and_then(|v| v.parse::<usize>().ok());

    let total = items.len();
    if start_index >= total {
        return (Vec::new(), start_index as i32);
    }

    let end = if let Some(l) = limit {
        std::cmp::min(start_index + l, total)
    } else {
        total
    };

    (items[start_index..end].to_vec(), start_index as i32)
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

// ---------------------------------------------------------------------------
// QueryItem-based filtering (operates on native types, not BaseItemDto)
// ---------------------------------------------------------------------------

pub(crate) fn apply_query_items_filter(items: Vec<QueryItem>, query_params: &HashMap<String, String>) -> Vec<QueryItem> {
    items
        .into_iter()
        .filter(|qi| apply_query_item_filter(qi, query_params))
        .collect()
}

fn apply_query_item_filter(qi: &QueryItem, qp: &HashMap<String, String>) -> bool {
    let item = &qi.item;

    // includeItemTypes
    if let Some(types) = qp.get("includeItemTypes") {
        let type_list: Vec<&str> = types.split(',').collect();
        if !type_list.contains(&item.jf_type()) {
            return false;
        }
    }

    // excludeItemTypes
    if let Some(types) = qp.get("excludeItemTypes") {
        let type_list: Vec<&str> = types.split(',').collect();
        if type_list.contains(&item.jf_type()) {
            return false;
        }
    }

    // isHd
    if let Some(hd) = qp.get("isHd") {
        let want_hd = hd.eq_ignore_ascii_case("true");
        if item.is_hd() != want_hd {
            return false;
        }
    }

    // is4K
    if let Some(k4) = qp.get("is4K") {
        let want_4k = k4.eq_ignore_ascii_case("true");
        if item.is_4k() != want_4k {
            return false;
        }
    }

    // ids
    if let Some(ids) = qp.get("ids") {
        let id = item.id();
        let id_list: Vec<&str> = ids.split(',').collect();
        if !id_list.contains(&id.as_str()) {
            return false;
        }
    }

    // excludeItemIds
    if let Some(exclude_ids) = qp.get("excludeItemIds") {
        let id = item.id();
        for eid in exclude_ids.split(',') {
            if id == eid {
                return false;
            }
        }
    }

    // genreIds (pipe-separated)
    if let Some(genre_ids) = qp.get("genreIds") {
        let item_genre_ids: Vec<String> = item
            .genres()
            .iter()
            .map(|g| id_hash_prefix(ITEM_PREFIX_GENRE, g))
            .collect();
        let mut keep = false;
        for gid in genre_ids.split('|') {
            if item_genre_ids.iter().any(|ig| ig == gid) {
                keep = true;
                break;
            }
        }
        if !keep {
            return false;
        }
    }

    // studioIds (pipe-separated)
    if let Some(studio_ids) = qp.get("studioIds") {
        let item_studio_ids: Vec<String> = item
            .studios()
            .iter()
            .map(|s| id_hash_prefix(ITEM_PREFIX_STUDIO, s))
            .collect();
        let mut keep = false;
        for sid in studio_ids.split('|') {
            if item_studio_ids.iter().any(|is| is == sid) {
                keep = true;
                break;
            }
        }
        if !keep {
            return false;
        }
    }

    // parentIndexNumber
    if let Some(pin_str) = qp.get("parentIndexNumber") {
        if let Ok(pin) = pin_str.parse::<i32>() {
            if item.parent_index_number() != Some(pin) {
                return false;
            }
        }
    }

    // indexNumber
    if let Some(in_str) = qp.get("indexNumber") {
        if let Ok(idx) = in_str.parse::<i32>() {
            if item.index_number() != Some(idx) {
                return false;
            }
        }
    }

    // nameStartsWith (case-insensitive)
    if let Some(prefix) = qp.get("nameStartsWith") {
        if !item.sort_name().to_lowercase().starts_with(&prefix.to_lowercase()) {
            return false;
        }
    }

    // nameStartsWithOrGreater (case-insensitive)
    if let Some(bound) = qp.get("nameStartsWithOrGreater") {
        if item.sort_name().to_lowercase() < bound.to_lowercase() {
            return false;
        }
    }

    // nameLessThan (case-insensitive)
    if let Some(bound) = qp.get("nameLessThan") {
        if item.sort_name().to_lowercase() > bound.to_lowercase() {
            return false;
        }
    }

    // genres (by name, pipe-separated)
    if let Some(include_genres) = qp.get("genres") {
        let item_genres = item.genres();
        let mut keep = false;
        for g in include_genres.split('|') {
            if item_genres.iter().any(|ig| ig == g) {
                keep = true;
                break;
            }
        }
        if !keep {
            return false;
        }
    }

    // studios (by name, pipe-separated)
    if let Some(include_studios) = qp.get("studios") {
        let item_studios = item.studios();
        let mut keep = false;
        for s in include_studios.split('|') {
            if item_studios.iter().any(|is| is == s) {
                keep = true;
                break;
            }
        }
        if !keep {
            return false;
        }
    }

    // officialRatings (pipe-separated)
    if let Some(ratings) = qp.get("officialRatings") {
        let mut keep = false;
        for r in ratings.split('|') {
            if item.official_rating() == Some(r) {
                keep = true;
                break;
            }
        }
        if !keep {
            return false;
        }
    }

    // minCommunityRating
    if let Some(min_str) = qp.get("minCommunityRating") {
        if let Ok(min) = min_str.parse::<f32>() {
            if item.community_rating().unwrap_or(0.0) < min {
                return false;
            }
        }
    }

    // minPremiereDate
    if let Some(date_str) = qp.get("minPremiereDate") {
        if let Some(min_date) = parse_iso8601_date(date_str) {
            match item.premiere_date() {
                Some(pd) if pd >= min_date => {}
                _ => return false,
            }
        }
    }

    // maxPremiereDate
    if let Some(date_str) = qp.get("maxPremiereDate") {
        if let Some(max_date) = parse_iso8601_date(date_str) {
            match item.premiere_date() {
                Some(pd) if pd <= max_date => {}
                _ => return false,
            }
        }
    }

    // years (comma-separated)
    if let Some(years_str) = qp.get("years") {
        let mut keep = false;
        for y in years_str.split(',') {
            if let Ok(year) = y.parse::<i32>() {
                if item.production_year() == Some(year) {
                    keep = true;
                    break;
                }
            }
        }
        if !keep {
            return false;
        }
    }

    // isPlayed (requires user_data)
    if let Some(played_str) = qp.get("isPlayed") {
        let want_played = played_str.eq_ignore_ascii_case("true");
        let is_played = qi.user_data.as_ref().map(|ud| ud.played).unwrap_or(false);
        if want_played != is_played {
            return false;
        }
    }

    // isFavorite (requires user_data)
    if let Some(fav_str) = qp.get("isFavorite") {
        let want_fav = fav_str.eq_ignore_ascii_case("true");
        let is_fav = qi.user_data.as_ref().map(|ud| ud.favorite).unwrap_or(false);
        if want_fav != is_fav {
            return false;
        }
    }

    // filters (comma-separated, e.g. "IsFavorite", "IsFavoriteOrLikes")
    if let Some(filters) = qp.get("filters") {
        for f in filters.split(',') {
            match f {
                "IsFavorite" | "IsFavoriteOrLikes" => {
                    let is_fav = qi.user_data.as_ref().map(|ud| ud.favorite).unwrap_or(false);
                    if !is_fav {
                        return false;
                    }
                }
                _ => {}
            }
        }
    }

    // searchTerm
    if let Some(term) = qp.get("searchTerm") {
        let id = item.id();
        let media = is_jf_movie_id(&id) || is_jf_show_id(&id) || is_jf_season_id(&id) || is_jf_episode_id(&id);
        if media && !item.name().to_lowercase().contains(&term.to_lowercase()) {
            return false;
        }
    }

    true
}

// ---------------------------------------------------------------------------
// QueryItem-based sorting
// ---------------------------------------------------------------------------

pub(crate) fn apply_query_item_sorting(
    items: &mut Vec<QueryItem>,
    query_params: &HashMap<String, String>,
) {
    let sort_by_raw = match query_params.get("sortBy") {
        Some(s) if !s.is_empty() => s.clone(),
        _ => return,
    };
    let sort_fields: Vec<String> = sort_by_raw.split(',').map(|s| s.to_lowercase()).collect();

    let descending = query_params
        .get("sortOrder")
        .map(|s| s.eq_ignore_ascii_case("descending"))
        .unwrap_or(false);

    items.sort_by(|a, b| {
        for field in &sort_fields {
            let ord = match field.as_str() {
                "communityrating" => {
                    let ar = a.item.community_rating().unwrap_or(0.0);
                    let br = b.item.community_rating().unwrap_or(0.0);
                    ar.partial_cmp(&br).unwrap_or(std::cmp::Ordering::Equal)
                }
                "datecreated" | "datelastcontentadded" => a.item.created().cmp(&b.item.created()),
                "dateplayed" => {
                    let ad = a.user_data.as_ref().map(|ud| ud.timestamp);
                    let bd = b.user_data.as_ref().map(|ud| ud.timestamp);
                    ad.cmp(&bd)
                }
                "indexnumber" => a.item.index_number().cmp(&b.item.index_number()),
                "isfavoriteorliked" => {
                    let af = a.user_data.as_ref().map(|ud| ud.favorite).unwrap_or(false);
                    let bf = b.user_data.as_ref().map(|ud| ud.favorite).unwrap_or(false);
                    af.cmp(&bf)
                }
                "isfolder" => a.item.is_folder().cmp(&b.item.is_folder()),
                "isplayed" => {
                    let ap = a.user_data.as_ref().map(|ud| ud.played).unwrap_or(false);
                    let bp = b.user_data.as_ref().map(|ud| ud.played).unwrap_or(false);
                    ap.cmp(&bp)
                }
                "isunplayed" => {
                    let ap = !a.user_data.as_ref().map(|ud| ud.played).unwrap_or(false);
                    let bp = !b.user_data.as_ref().map(|ud| ud.played).unwrap_or(false);
                    ap.cmp(&bp)
                }
                "officialrating" => a.item.official_rating().cmp(&b.item.official_rating()),
                "parentindexnumber" => a.item.parent_index_number().cmp(&b.item.parent_index_number()),
                "premieredate" => a.item.premiere_date().cmp(&b.item.premiere_date()),
                "productionyear" => a.item.production_year().cmp(&b.item.production_year()),
                "random" => {
                    let mut rng = rand::thread_rng();
                    if rng.gen_bool(0.5) {
                        std::cmp::Ordering::Less
                    } else {
                        std::cmp::Ordering::Greater
                    }
                }
                "runtime" => a.item.run_time_ticks().cmp(&b.item.run_time_ticks()),
                "name" | "seriessortname" | "sortname" | "default" => {
                    a.item.sort_name().cmp(b.item.sort_name())
                }
                other => {
                    warn!("apply_query_item_sorting: unknown sort field: {}", other);
                    std::cmp::Ordering::Equal
                }
            };
            if ord != std::cmp::Ordering::Equal {
                return if descending { ord.reverse() } else { ord };
            }
        }
        std::cmp::Ordering::Equal
    });
}

// ---------------------------------------------------------------------------
// QueryItem-based pagination
// ---------------------------------------------------------------------------

pub(crate) fn apply_query_item_pagination(
    items: Vec<QueryItem>,
    query_params: &HashMap<String, String>,
) -> (Vec<QueryItem>, i32) {
    let start_index = query_params
        .get("startIndex")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(0);
    let limit = query_params.get("limit").and_then(|v| v.parse::<usize>().ok());

    let total = items.len();
    if start_index >= total {
        return (Vec::new(), start_index as i32);
    }

    let end = if let Some(l) = limit {
        std::cmp::min(start_index + l, total)
    } else {
        total
    };

    // Move items out of the vec for the requested range
    let paged: Vec<QueryItem> = items.into_iter().skip(start_index).take(end - start_index).collect();
    (paged, start_index as i32)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Parse an ISO 8601 date string into a DateTime<Utc>.
/// Tries multiple formats: RFC3339, datetime, date-only, year-month, year.
pub fn parse_iso8601_date(input: &str) -> Option<DateTime<Utc>> {
    // Try RFC3339 / full datetime with timezone
    if let Ok(dt) = DateTime::parse_from_rfc3339(input) {
        return Some(dt.with_timezone(&Utc));
    }
    // Try "2006-01-02 15:04:05"
    if let Ok(ndt) = NaiveDateTime::parse_from_str(input, "%Y-%m-%d %H:%M:%S") {
        return Some(ndt.and_utc());
    }
    // Try "2006-01-02"
    if let Ok(nd) = NaiveDate::parse_from_str(input, "%Y-%m-%d") {
        return nd.and_hms_opt(0, 0, 0).map(|ndt| ndt.and_utc());
    }
    // Try "2006-01"
    if input.len() == 7 {
        if let Ok(nd) = NaiveDate::parse_from_str(&format!("{}-01", input), "%Y-%m-%d") {
            return nd.and_hms_opt(0, 0, 0).map(|ndt| ndt.and_utc());
        }
    }
    // Try "2006" (year only)
    if input.len() == 4 {
        if let Ok(year) = input.parse::<i32>() {
            return NaiveDate::from_ymd_opt(year, 1, 1)
                .and_then(|nd| nd.and_hms_opt(0, 0, 0))
                .map(|ndt| ndt.and_utc());
        }
    }
    None
}
