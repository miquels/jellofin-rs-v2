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
use super::jfitem2::*;
use super::types::*;
use crate::database::model;

/// GET /Items/{item} - Get details for a specific item
pub async fn item_details(
    Extension(token): Extension<model::AccessToken>,
    State(state): State<JellyfinState>,
    AxumPath(item_id): AxumPath<String>,
) -> Result<Json<BaseItemDto>, StatusCode> {
    let response = make_jfitem_by_id(&state, &token.user_id, &item_id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    Ok(Json(response))
}

/// GET /Items - Get list of items based upon provided query params
pub async fn items_query(
    Extension(token): Extension<model::AccessToken>,
    State(state): State<JellyfinState>,
    Query(query_params): Query<HashMap<String, String>>,
) -> Result<Json<UserItemsResponse>, StatusCode> {
    let parent_id = query_params.get("parentId").cloned();
    let search_term = query_params.get("searchTerm").cloned();
    let recursive = query_params
        .get("recursive")
        .map(|v| v == "true")
        .unwrap_or(false);

    let mut items = Vec::new();

    if let Some(ref pid) = parent_id {
        if search_term.is_none() {
            items = get_jfitems_by_parent_id(&state, &token.user_id, pid)
                .await
                .map_err(|_| StatusCode::NOT_FOUND)?;
        }
    } else if !recursive {
        items = make_jfcollection_root_overview(&state, &token.user_id)
            .await
            .map_err(|_| StatusCode::NOT_FOUND)?;
    } else {
        // Recursive query across all collections — pre-filter and pre-sort at the
        // collection level to avoid building DTOs for all items when only a few are needed.
        let result = query_items_presorted(&state, &token.user_id, &query_params).await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        return Ok(Json(result));
    }

    // If searchTerm is provided, search in whole collection
    if let Some(ref st) = search_term {
        let found_ids = state.collections.search(st);
        let mut search_items = Vec::new();
        for id in found_ids {
            if let Some((c, item)) = state.collections.get_item_by_id(&id) {
                if let Ok(dto) = make_jfitem(&state, &token.user_id, &item, &c.id).await {
                    search_items.push(dto);
                }
            }
        }
        items = search_items;
    }

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

/// Pre-filter and pre-sort items at the collection level, then build DTOs
/// only for the items that will actually be returned. This avoids building
/// DTOs for all 6990+ items when the client only needs 3.
async fn query_items_presorted(
    state: &JellyfinState,
    user_id: &str,
    query_params: &HashMap<String, String>,
) -> Result<UserItemsResponse, StatusCode> {
    use crate::collection::Item;

    // Collect type filter
    let type_filter: Option<Vec<&str>> = query_params
        .get("includeItemTypes")
        .map(|t| t.split(',').collect());

    // Genre filter: genreIds are "genre_<hash>" — we need to match by generating IDs from item genres
    let genre_ids: Option<Vec<&str>> = query_params
        .get("genreIds")
        .map(|g| g.split('|').collect());

    // Gather matching (item_ref, collection_id) pairs from all collections
    let collections = state.collections.get_collections();
    let mut matching: Vec<(&Item, &str)> = Vec::new();
    for c in &collections {
        for item in &c.items {
            // Type filter
            if let Some(ref types) = type_filter {
                if !types.contains(&item.jf_type()) {
                    continue;
                }
            }
            // Genre filter
            if let Some(ref gids) = genre_ids {
                let item_genre_ids: Vec<String> = item
                    .genres()
                    .iter()
                    .map(|g| super::jfitem2::make_jf_genre_id(g))
                    .collect();
                if !gids.iter().any(|gid| item_genre_ids.iter().any(|ig| ig == gid)) {
                    continue;
                }
            }
            matching.push((item, &c.id));
        }
    }

    let total_item_count = matching.len() as i32;

    // Sort at collection level
    let sort_by = query_params.get("sortBy").cloned().unwrap_or_default();
    let descending = query_params
        .get("sortOrder")
        .map(|s| s.eq_ignore_ascii_case("descending"))
        .unwrap_or(false);

    if !sort_by.is_empty() {
        let sort_fields: Vec<String> = sort_by.split(',').map(|s| s.to_lowercase()).collect();
        matching.sort_by(|(a, _), (b, _)| {
            for field in &sort_fields {
                let ord = match field.as_str() {
                    "datecreated" | "datelastcontentadded" => a.created().cmp(&b.created()),
                    "premieredate" => a.premiere_date().cmp(&b.premiere_date()),
                    "communityrating" => {
                        let ar = a.community_rating().unwrap_or(0.0);
                        let br = b.community_rating().unwrap_or(0.0);
                        ar.partial_cmp(&br).unwrap_or(std::cmp::Ordering::Equal)
                    }
                    "productionyear" => a.production_year().cmp(&b.production_year()),
                    "name" | "sortname" | "seriessortname" | "default" => {
                        a.sort_name().cmp(b.sort_name())
                    }
                    "random" => {
                        let mut rng = rand::thread_rng();
                        if rng.gen_bool(0.5) {
                            std::cmp::Ordering::Less
                        } else {
                            std::cmp::Ordering::Greater
                        }
                    }
                    _ => std::cmp::Ordering::Equal,
                };
                if ord != std::cmp::Ordering::Equal {
                    return if descending { ord.reverse() } else { ord };
                }
            }
            std::cmp::Ordering::Equal
        });
    }

    // Apply pagination
    let start_index = query_params
        .get("startIndex")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(0);
    let limit = query_params
        .get("limit")
        .and_then(|v| v.parse::<usize>().ok());

    let paged: Vec<(&Item, &str)> = matching
        .into_iter()
        .skip(start_index)
        .take(limit.unwrap_or(usize::MAX))
        .collect();

    // Build DTOs only for the selected items
    let mut items = Vec::with_capacity(paged.len());
    for (item, collection_id) in paged {
        match make_jfitem_light(state, user_id, item, collection_id).await {
            Ok(dto) => items.push(dto),
            Err(e) => warn!("query_items_presorted: {}", e),
        }
    }

    apply_fields_filter(&mut items, query_params);

    Ok(UserItemsResponse {
        items,
        start_index: start_index as i32,
        total_record_count: total_item_count,
    })
}

/// GET /Items/Latest - Get latest items
pub async fn items_latest(
    Extension(token): Extension<model::AccessToken>,
    State(state): State<JellyfinState>,
    Query(query_params): Query<HashMap<String, String>>,
) -> Result<Json<Vec<BaseItemDto>>, StatusCode> {
    let parent_id = query_params.get("parentId").cloned();

    let mut items = if let Some(ref pid) = parent_id {
        get_jfitems_by_parent_id(&state, &token.user_id, pid)
            .await
            .map_err(|_| StatusCode::NOT_FOUND)?
    } else {
        get_jfitems_all(&state, &token.user_id)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    };

    items = apply_items_filter(items, &query_params);

    // Sort by premiere date descending
    items.sort_by(|a, b| b.premiere_date.cmp(&a.premiere_date));

    // Default limit to 50 for latest if not provided
    let mut qp = query_params.clone();
    if !qp.contains_key("limit") {
        qp.insert("limit".to_string(), "50".to_string());
    }

    let (paged_items, _) = apply_item_pagination(items, &qp);
    Ok(Json(paged_items))
}

/// GET /Items/Counts - Get item counts
pub async fn items_counts(
    Extension(_token): Extension<model::AccessToken>,
    State(state): State<JellyfinState>,
) -> Json<ItemCountResponse> {
    let details = state.collections.details();

    Json(ItemCountResponse {
        movie_count: details.movie_count as i32,
        series_count: details.show_count as i32,
        episode_count: details.episode_count as i32,
        artist_count: 0,
        program_count: 0,
        trailer_count: 0,
        song_count: 0,
        album_count: 0,
        music_video_count: 0,
        box_set_count: 0,
        book_count: 0,
        item_count: (details.movie_count + details.show_count + details.episode_count) as i32,
    })
}

/// GET /Items/Resume - Get resume items
pub async fn items_resume(
    Extension(token): Extension<model::AccessToken>,
    State(state): State<JellyfinState>,
    Query(query_params): Query<HashMap<String, String>>,
) -> Result<Json<UsersItemsResumeResponse>, StatusCode> {
    let resume_ids = state
        .repo
        .get_recently_watched(&token.user_id, false)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut items = Vec::new();
    for id in resume_ids {
        if let Some((c, item)) = state.collections.get_item_by_id(&id) {
            if let Ok(dto) = make_jfitem(&state, &token.user_id, &item, &c.id).await {
                items.push(dto);
            }
        }
    }

    let items = apply_items_filter(items, &query_params);
    let total_count = items.len() as i32;
    let items = apply_item_sorting(items, &query_params);
    let (paged_items, start_index) = apply_item_pagination(items, &query_params);

    Ok(Json(UsersItemsResumeResponse {
        items: paged_items,
        start_index,
        total_record_count: total_count,
    }))
}

/// GET /Items/{item}/Similar - Get similar items
pub async fn items_similar(
    Extension(token): Extension<model::AccessToken>,
    State(state): State<JellyfinState>,
    AxumPath(item_id): AxumPath<String>,
    Query(query_params): Query<HashMap<String, String>>,
) -> Result<Json<UsersItemsSimilarResponse>, StatusCode> {
    let internal_id = trim_prefix(&item_id);
    let (collection, item) = state
        .collections
        .get_item_by_id(internal_id)
        .ok_or(StatusCode::NOT_FOUND)?;

    let similar_ids = state.collections.similar(&collection.id, &item.id()).await;

    let mut items = Vec::new();
    for id in similar_ids {
        if let Some((c, item)) = state.collections.get_item_by_id(&id) {
            if let Ok(dto) = make_jfitem(&state, &token.user_id, &item, &c.id).await {
                items.push(dto);
            }
        }
    }

    let items = apply_items_filter(items, &query_params);
    let total_count = items.len() as i32;
    let items = apply_item_sorting(items, &query_params);
    let (paged_items, start_index) = apply_item_pagination(items, &query_params);

    Ok(Json(UsersItemsSimilarResponse {
        items: paged_items,
        start_index,
        total_record_count: total_count,
    }))
}

/// GET /Items/{item}/SpecialFeatures - Returns empty list (not implemented)
pub async fn items_special_features(
    Extension(_token): Extension<model::AccessToken>,
    State(_state): State<JellyfinState>,
    AxumPath(_item_id): AxumPath<String>,
) -> Json<Vec<BaseItemDto>> {
    Json(Vec::new())
}

/// DELETE /Items/{item} - Not implemented, returns Forbidden
pub async fn items_delete(
    Extension(_token): Extension<model::AccessToken>,
    State(_state): State<JellyfinState>,
    AxumPath(_item_id): AxumPath<String>,
) -> StatusCode {
    StatusCode::FORBIDDEN
}

/// GET /Items/{item}/PlaybackInfo - Returns playback info including media sources
pub async fn items_playback_info(
    Extension(_token): Extension<model::AccessToken>,
    State(state): State<JellyfinState>,
    AxumPath(item_id): AxumPath<String>,
) -> Result<Json<PlaybackInfoResponse>, StatusCode> {
    let internal_id = trim_prefix(&item_id);
    let (_, item) = state
        .collections
        .get_item_by_id(internal_id)
        .ok_or(StatusCode::NOT_FOUND)?;

    use crate::collection::Item;
    let media_sources = match &item {
        Item::Movie(m) => make_media_source(&m.id, &m.file_name, m.file_size, &m.metadata),
        Item::Episode(e) => make_media_source(&e.id, &e.file_name, e.file_size, &e.metadata),
        _ => return Err(StatusCode::NOT_FOUND),
    };

    if media_sources.is_empty() {
        return Err(StatusCode::NOT_FOUND);
    }

    Ok(Json(PlaybackInfoResponse {
        media_sources,
        play_session_id: super::session::SESSION_ID.to_string(),
    }))
}

/// GET /Search/Hints - Get search hints
pub async fn search_hints(
    Extension(token): Extension<model::AccessToken>,
    State(state): State<JellyfinState>,
    Query(query_params): Query<HashMap<String, String>>,
) -> Result<Json<SearchHintsResponse>, StatusCode> {
    if let Some(parent_id) = query_params.get("parentId") {
        if is_jf_collection_playlist_id(parent_id) {
            let items = make_jfitem_playlist_overview(&state, &token.user_id)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            return Ok(Json(SearchHintsResponse {
                search_hints: items,
                total_record_count: 0,
            }));
        }
    }

    // Determine if we should scope search to a specific collection
    let search_collection_id = query_params.get("parentId").and_then(|pid| {
        if is_jf_collection_id(pid) {
            Some(trim_prefix(pid).to_string())
        } else {
            None
        }
    });

    let mut items = Vec::new();
    for c in state.collections.get_collections() {
        // Skip if we are searching in one particular collection
        if let Some(ref scid) = search_collection_id {
            if *scid != c.id {
                continue;
            }
        }
        for item in &c.items {
            if let Ok(dto) = make_jfitem(&state, &token.user_id, item, &c.id).await {
                items.push(dto);
            }
        }
    }

    let items = apply_items_filter(items, &query_params);
    let total_count = items.len() as i32;
    let items = apply_item_sorting(items, &query_params);
    let (paged_items, _) = apply_item_pagination(items, &query_params);

    Ok(Json(SearchHintsResponse {
        search_hints: paged_items,
        total_record_count: total_count,
    }))
}

/// GET /Items/{item}/Ancestors - Get ancestors for an item
pub async fn item_ancestors(
    Extension(token): Extension<model::AccessToken>,
    State(state): State<JellyfinState>,
    AxumPath(item_id): AxumPath<String>,
) -> Result<Json<Vec<BaseItemDto>>, StatusCode> {
    let internal_id = trim_prefix(&item_id);
    let (collection, _) = state
        .collections
        .get_item_by_id(internal_id)
        .ok_or(StatusCode::NOT_FOUND)?;

    let collection_item =
        make_jfitem_collection(&state, &collection.id).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let root_item = make_jfitem_root(&state, &token.user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(vec![collection_item, root_item]))
}

/// GET /Users/{userId}/Items/Suggestions - Get item suggestions
/// GET /Items/Suggestions - Get item suggestions
pub async fn items_suggestions(
    Extension(_token): Extension<model::AccessToken>,
    State(_state): State<JellyfinState>,
) -> Json<UsersItemsSuggestionsResponse> {
    Json(UsersItemsSuggestionsResponse {
        items: Vec::new(),
        start_index: 0,
        total_record_count: 0,
    })
}

/// GET /Users/{userId}/Items/Filters - Get item filters
pub async fn item_filters(
    Extension(_token): Extension<model::AccessToken>,
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
    Extension(_token): Extension<model::AccessToken>,
    State(state): State<JellyfinState>,
) -> Json<ItemFilter2Response> {
    let details = state.collections.details();
    let genres = details
        .genres
        .into_iter()
        .map(|g| NameGuidPair {
            name: g.clone(),
            id: crate::idhash::id_hash(&g),
        })
        .collect();

    Json(ItemFilter2Response {
        genres,
        tags: details.tags,
    })
}

// ---------------------------------------------------------------------------
// Filtering
// ---------------------------------------------------------------------------

fn apply_items_filter(items: Vec<BaseItemDto>, query_params: &HashMap<String, String>) -> Vec<BaseItemDto> {
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
        if !i.name.to_lowercase().contains(&term.to_lowercase()) {
            return false;
        }
    }

    true
}

// ---------------------------------------------------------------------------
// Sorting
// ---------------------------------------------------------------------------

pub(super) fn apply_item_sorting(mut items: Vec<BaseItemDto>, query_params: &HashMap<String, String>) -> Vec<BaseItemDto> {
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

pub(super) fn apply_item_pagination(
    items: Vec<BaseItemDto>,
    query_params: &HashMap<String, String>,
) -> (Vec<BaseItemDto>, i32) {
    let start_index = query_params
        .get("startIndex")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(0);
    let limit = query_params
        .get("limit")
        .and_then(|v| v.parse::<usize>().ok());

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
/// This matches the real Jellyfin API behavior where the `Fields` parameter controls
/// which additional fields are included in the response.
fn apply_fields_filter(items: &mut Vec<BaseItemDto>, query_params: &HashMap<String, String>) {
    let fields_str = match query_params.get("fields") {
        Some(f) => f,
        None => return, // No Fields parameter = return everything (backwards compat)
    };

    let fields: HashSet<&str> = fields_str
        .split(',')
        .map(|s| s.trim())
        .collect();

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
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Parse an ISO 8601 date string into a DateTime<Utc>.
/// Tries multiple formats: RFC3339, datetime, date-only, year-month, year.
fn parse_iso8601_date(input: &str) -> Option<DateTime<Utc>> {
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
        return nd
            .and_hms_opt(0, 0, 0)
            .map(|ndt| ndt.and_utc());
    }
    // Try "2006-01"
    if input.len() == 7 {
        if let Ok(nd) = NaiveDate::parse_from_str(&format!("{}-01", input), "%Y-%m-%d") {
            return nd
                .and_hms_opt(0, 0, 0)
                .map(|ndt| ndt.and_utc());
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
