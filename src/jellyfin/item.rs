use axum::{
    extract::{Path as AxumPath, Query, State},
    http::StatusCode,
    response::Json,
    Extension,
};
use std::collections::HashMap;

use super::jellyfin::JellyfinState;
use super::jfitem::*;
use super::types::*;
use crate::collection::Item;
use crate::database::model;
use anyhow::{anyhow, bail, Result as AnyhowResult};

/// GET /Items/{item} - Get details for a specific item
pub async fn item_details(
    Extension(token): Extension<model::AccessToken>,
    State(state): State<JellyfinState>,
    AxumPath(item_id): AxumPath<String>,
) -> Result<Json<BaseItemDto>, StatusCode> {
    let response = make_jf_item_by_id(&state, &token.user_id, &item_id)
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
    let recursive = query_params.get("recursive").map(|v| v == "true").unwrap_or(false);
    let ids = query_params.get("ids");

    let mut items = Vec::new();

    if let Some(id_list) = ids {
        // Direct ID lookup - bypass parent filters
        for id in id_list.split(',') {
            if let Ok(item) = make_jf_item_by_id(&state, &token.user_id, id).await {
                items.push(item);
            }
        }
    } else if let Some(ref pid) = parent_id {
        if search_term.is_none() {
            items = get_jf_items_by_parent_id(&state, &token.user_id, pid)
                .await
                .map_err(|_| StatusCode::NOT_FOUND)?;
        }
    } else {
        if !recursive {
            items = make_jf_collection_root_overview(&state, &token.user_id)
                .await
                .map_err(|_| StatusCode::NOT_FOUND)?;
        } else {
            items = get_jf_items_all(&state, &token.user_id)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        }
    }

    if let Some(ref st) = search_term {
        let found_ids = state.collections.search(st);
        let mut search_items = Vec::new();
        for id in found_ids {
            if let Some((_, item)) = state.collections.get_item_by_id(&id) {
                if let Ok(dto) = make_jf_item(&state, &token.user_id, &item).await {
                    search_items.push(dto);
                }
            }
        }

        if items.is_empty() {
            items = search_items;
        } else {
            items.retain(|i| i.name.to_lowercase().contains(&st.to_lowercase()));
        }
    }

    let items = apply_items_filter(items, &query_params);
    let total_item_count = items.len() as i32;

    let sorted_items = apply_item_sorting(items, &query_params);
    let (paged_items, start_index) = apply_item_pagination(sorted_items, &query_params);

    Ok(Json(UserItemsResponse {
        items: paged_items,
        start_index,
        total_record_count: total_item_count,
    }))
}

/// GET /Items/Latest - Get latest items
pub async fn items_latest(
    Extension(token): Extension<model::AccessToken>,
    State(state): State<JellyfinState>,
    Query(query_params): Query<HashMap<String, String>>,
) -> Result<Json<Vec<BaseItemDto>>, StatusCode> {
    let parent_id = query_params.get("parentId").cloned();

    let mut items = if let Some(ref pid) = parent_id {
        get_jf_items_by_parent_id(&state, &token.user_id, pid)
            .await
            .map_err(|_| StatusCode::NOT_FOUND)?
    } else {
        get_jf_items_all(&state, &token.user_id)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    };

    items = apply_items_filter(items, &query_params);

    // Sort by premiere date descending
    items.sort_by(|a, b| b.premiere_date.cmp(&a.premiere_date));

    // Default limit to 20 for latest if not provided
    let mut qp = query_params.clone();
    if !qp.contains_key("limit") {
        qp.insert("limit".to_string(), "20".to_string());
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
        if let Some((_, item)) = state.collections.get_item_by_id(&id) {
            if let Ok(dto) = make_jf_item(&state, &token.user_id, &item).await {
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
        if let Some((_, item)) = state.collections.get_item_by_id(&id) {
            if let Ok(dto) = make_jf_item(&state, &token.user_id, &item).await {
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

/// GET /Search/Hints - Get search hints
pub async fn search_hints(
    Extension(token): Extension<model::AccessToken>,
    State(state): State<JellyfinState>,
    Query(query_params): Query<HashMap<String, String>>,
) -> Result<Json<SearchHintsResponse>, StatusCode> {
    if let Some(parent_id) = query_params.get("parentId") {
        if is_jf_collection_playlist_id(parent_id) {
            let items = make_jf_playlist_overview(&state, &token.user_id)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            return Ok(Json(SearchHintsResponse {
                search_hints: items,
                total_record_count: 0,
            }));
        }
    }

    let search_term = query_params.get("searchTerm").cloned().unwrap_or_default();
    let found_ids = state.collections.search(&search_term);

    let mut items = Vec::new();
    for id in found_ids {
        if let Some((_, item)) = state.collections.get_item_by_id(&id) {
            if let Ok(dto) = make_jf_item(&state, &token.user_id, &item).await {
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
        make_jf_item_collection(&state, &collection.id).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let root_item = make_jf_item_root(&state, &token.user_id)
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
        .map(|g| GenreItem {
            name: g.clone(),
            id: crate::idhash::id_hash(&g),
        })
        .collect();

    Json(ItemFilter2Response {
        genres,
        tags: details.tags,
    })
}

// Helpers

pub async fn make_jf_item_by_id(state: &JellyfinState, user_id: &str, item_id: &str) -> AnyhowResult<BaseItemDto> {
    if is_jf_root_id(item_id) {
        return make_jf_item_root(state, user_id).await;
    }
    if is_jf_collection_favorites_id(item_id) {
        return make_jf_item_collection_favorites(state, user_id).await;
    }
    if is_jf_collection_playlist_id(item_id) {
        return make_jf_item_collection_playlist(state, user_id).await;
    }
    if is_jf_collection_id(item_id) {
        return make_jf_item_collection(state, trim_prefix(item_id));
    }

    let internal_id = trim_prefix(item_id);
    if let Some((_, item)) = state.collections.get_item_by_id(internal_id) {
        return make_jf_item(state, user_id, &item).await;
    }

    bail!("Item not found")
}

async fn get_jf_items_by_parent_id(
    state: &JellyfinState,
    user_id: &str,
    parent_id: &str,
) -> AnyhowResult<Vec<BaseItemDto>> {
    if is_jf_collection_favorites_id(parent_id) {
        return make_jf_item_favorites_overview(state, user_id).await;
    }
    if is_jf_collection_playlist_id(parent_id) {
        return make_jf_playlist_overview(state, user_id).await;
    }
    if is_jf_collection_id(parent_id) {
        let collection_id = trim_prefix(parent_id);
        if let Some(c) = state.collections.get_collection(collection_id) {
            let mut items = Vec::new();
            for item in &c.items {
                if let Ok(dto) = make_jf_item(state, user_id, item).await {
                    items.push(dto);
                }
            }
            return Ok(items);
        }
    }

    let internal_id = trim_prefix(parent_id);
    if let Some((_, item)) = state.collections.get_item_by_id(internal_id) {
        match item {
            Item::Show(show) => {
                return make_jf_seasons_overview(state, user_id, &show).await;
            }
            Item::Season(season) => {
                if let Some((_, show, _)) = state.collections.get_season_by_id(&season.id) {
                    return make_jf_episodes_overview(state, user_id, &season, &show).await;
                }
            }
            _ => {}
        }
    }

    bail!("Parent not found or unsupported")
}

async fn get_jf_items_all(state: &JellyfinState, user_id: &str) -> AnyhowResult<Vec<BaseItemDto>> {
    let mut items = Vec::new();
    for c in state.collections.get_collections() {
        for item in &c.items {
            if let Ok(dto) = make_jf_item(state, user_id, item).await {
                items.push(dto);
            }
        }
    }
    Ok(items)
}

pub async fn make_jf_item_root(state: &JellyfinState, user_id: &str) -> AnyhowResult<BaseItemDto> {
    let mut dto = BaseItemDto::default();
    dto.name = "Media Folders".to_string();
    dto.id = make_jf_root_id(COLLECTION_ROOT_ID);
    dto.server_id = state.server_id.clone();
    dto.item_type = ITEM_TYPE_USER_ROOT_FOLDER.to_string();
    dto.is_folder = Some(true);

    let collections = make_jf_collection_root_overview(state, user_id).await?;
    dto.child_count = Some(collections.len() as i32);

    Ok(dto)
}

async fn make_jf_collection_root_overview(state: &JellyfinState, user_id: &str) -> AnyhowResult<Vec<BaseItemDto>> {
    let mut items = Vec::new();
    for c in state.collections.get_collections() {
        if let Ok(dto) = make_jf_item_collection(state, &c.id) {
            items.push(dto);
        }
    }

    if let Ok(favs) = make_jf_item_collection_favorites(state, user_id).await {
        items.push(favs);
    }
    if let Ok(playlists) = make_jf_item_collection_playlist(state, user_id).await {
        items.push(playlists);
    }

    Ok(items)
}

pub fn make_jf_item_collection(state: &JellyfinState, collection_id: &str) -> AnyhowResult<BaseItemDto> {
    let c = state
        .collections
        .get_collection(collection_id)
        .ok_or_else(|| anyhow!("Collection not found"))?;
    let mut dto = BaseItemDto::default();
    dto.name = c.name.clone();
    dto.id = make_jf_collection_id(&c.id);
    dto.server_id = state.server_id.clone();
    dto.item_type = ITEM_TYPE_COLLECTION_FOLDER.to_string();
    dto.is_folder = Some(true);
    dto.child_count = Some(c.items.len() as i32);

    match c.collection_type {
        crate::collection::CollectionType::Movies => dto.collection_type = Some("movies".to_string()),
        crate::collection::CollectionType::Shows => dto.collection_type = Some("tvshows".to_string()),
    }

    Ok(dto)
}

async fn make_jf_item_collection_favorites(state: &JellyfinState, user_id: &str) -> AnyhowResult<BaseItemDto> {
    let favorites = state.repo.get_favorites(user_id).await.unwrap_or_default();
    let mut dto = BaseItemDto::default();
    dto.name = "Favorites".to_string();
    dto.id = make_jf_collection_favorites_id(FAVORITES_COLLECTION_ID);
    dto.server_id = state.server_id.clone();
    dto.item_type = ITEM_TYPE_USER_VIEW.to_string();
    dto.is_folder = Some(true);
    dto.child_count = Some(favorites.len() as i32);
    dto.collection_type = Some("playlists".to_string());

    Ok(dto)
}

async fn make_jf_item_collection_playlist(state: &JellyfinState, user_id: &str) -> AnyhowResult<BaseItemDto> {
    let playlist_ids = state.repo.get_playlists(user_id).await.unwrap_or_default();
    let mut dto = BaseItemDto::default();
    dto.name = "Playlists".to_string();
    dto.id = make_jf_collection_playlist_id(PLAYLIST_COLLECTION_ID);
    dto.server_id = state.server_id.clone();
    dto.item_type = ITEM_TYPE_USER_VIEW.to_string();
    dto.is_folder = Some(true);
    dto.child_count = Some(playlist_ids.len() as i32);
    dto.collection_type = Some("playlists".to_string());

    Ok(dto)
}

async fn make_jf_item_favorites_overview(state: &JellyfinState, user_id: &str) -> AnyhowResult<Vec<BaseItemDto>> {
    let favorites = state.repo.get_favorites(user_id).await.unwrap_or_default();
    let mut items = Vec::new();
    for id in favorites {
        if let Some((_, item)) = state.collections.get_item_by_id(&id) {
            if let Ok(dto) = make_jf_item(state, user_id, &item).await {
                items.push(dto);
            }
        }
    }
    Ok(items)
}

async fn make_jf_playlist_overview(state: &JellyfinState, user_id: &str) -> AnyhowResult<Vec<BaseItemDto>> {
    let playlist_ids = state.repo.get_playlists(user_id).await.unwrap_or_default();
    let mut items = Vec::new();
    for id in playlist_ids {
        if let Ok(p) = state.repo.get_playlist(user_id, &id).await {
            let mut dto = BaseItemDto::default();
            dto.name = p.name;
            dto.id = make_jf_playlist_id(&p.id);
            dto.server_id = state.server_id.clone();
            dto.item_type = ITEM_TYPE_PLAYLIST.to_string();
            dto.is_folder = Some(false);
            items.push(dto);
        }
    }
    Ok(items)
}

pub async fn make_jf_item(state: &JellyfinState, user_id: &str, item: &Item) -> AnyhowResult<BaseItemDto> {
    let user_data = state.repo.get_user_data(user_id, &item.id()).await.ok();

    match item {
        Item::Movie(movie) => Ok(convert_movie_to_dto(movie, &state.server_id, user_data.as_ref())),
        Item::Show(show) => Ok(convert_show_to_dto(show, &state.server_id, user_data.as_ref())),
        Item::Season(season) => {
            if let Some((_, show, _)) = state.collections.get_season_by_id(&season.id) {
                Ok(convert_season_to_dto(
                    season,
                    &show,
                    &state.server_id,
                    user_data.as_ref(),
                ))
            } else {
                bail!("Show not found for season")
            }
        }
        Item::Episode(episode) => {
            if let Some((_, show, _, _)) = state.collections.get_episode_by_id(&episode.id) {
                Ok(convert_episode_to_dto(
                    episode,
                    &show,
                    &state.server_id,
                    user_data.as_ref(),
                ))
            } else {
                bail!("Show not found for episode")
            }
        }
    }
}

pub async fn make_jf_seasons_overview(
    state: &JellyfinState,
    user_id: &str,
    show: &crate::collection::item::Show,
) -> AnyhowResult<Vec<BaseItemDto>> {
    let mut seasons = Vec::new();
    for s in &show.seasons {
        let user_data = state.repo.get_user_data(user_id, &s.id).await.ok();
        seasons.push(convert_season_to_dto(s, show, &state.server_id, user_data.as_ref()));
    }
    seasons.sort_by_key(|s| s.index_number.unwrap_or(0));
    Ok(seasons)
}

pub async fn make_jf_episodes_overview(
    state: &JellyfinState,
    user_id: &str,
    season: &crate::collection::item::Season,
    show: &crate::collection::item::Show,
) -> AnyhowResult<Vec<BaseItemDto>> {
    let mut episodes = Vec::new();
    for e in &season.episodes {
        let user_data = state.repo.get_user_data(user_id, &e.id).await.ok();
        episodes.push(convert_episode_to_dto(e, show, &state.server_id, user_data.as_ref()));
    }
    episodes.sort_by_key(|e| e.index_number.unwrap_or(0));
    Ok(episodes)
}

fn apply_items_filter(items: Vec<BaseItemDto>, query_params: &HashMap<String, String>) -> Vec<BaseItemDto> {
    items
        .into_iter()
        .filter(|i| apply_item_filter(i, query_params))
        .collect()
}

fn apply_item_filter(i: &BaseItemDto, query_params: &HashMap<String, String>) -> bool {
    if let Some(types) = query_params.get("includeItemTypes") {
        let type_list: Vec<&str> = types.split(',').collect();
        if !type_list.contains(&i.item_type.as_str()) {
            return false;
        }
    }

    if let Some(types) = query_params.get("excludeItemTypes") {
        let type_list: Vec<&str> = types.split(',').collect();
        if type_list.contains(&i.item_type.as_str()) {
            return false;
        }
    }

    if let Some(ids) = query_params.get("ids") {
        let id_list: Vec<&str> = ids.split(',').collect();
        if !id_list.contains(&i.id.as_str()) {
            return false;
        }
    }

    if let Some(fav) = query_params.get("isFavorite") {
        let is_fav = fav == "true";
        if let Some(ud) = &i.user_data {
            if ud.is_favorite != is_fav {
                return false;
            }
        } else if is_fav {
            return false;
        }
    }

    if let Some(hd) = query_params.get("isHd") {
        let is_hd = hd == "true";
        if i.is_hd != Some(is_hd) {
            return false;
        }
    }

    if let Some(k4) = query_params.get("is4k") {
        let is_4k = k4 == "true";
        if i.is_4k != Some(is_4k) {
            return false;
        }
    }

    if let Some(term) = query_params.get("searchTerm") {
        if !i.name.to_lowercase().contains(&term.to_lowercase()) {
            return false;
        }
    }

    true
}

fn apply_item_sorting(mut items: Vec<BaseItemDto>, query_params: &HashMap<String, String>) -> Vec<BaseItemDto> {
    let sort_by = query_params.get("sortBy").map(|s| s.as_str()).unwrap_or("SortName");
    let sort_order = query_params.get("sortOrder").map(|s| s.as_str()).unwrap_or("Ascending");
    let descending = sort_order == "Descending";

    match sort_by {
        "SortName" => {
            items.sort_by(|a, b| {
                let res = a
                    .sort_name
                    .as_ref()
                    .unwrap_or(&a.name)
                    .cmp(b.sort_name.as_ref().unwrap_or(&b.name));
                if descending {
                    res.reverse()
                } else {
                    res
                }
            });
        }
        "PremiereDate" => {
            items.sort_by(|a, b| {
                let res = a.premiere_date.cmp(&b.premiere_date);
                if descending {
                    res.reverse()
                } else {
                    res
                }
            });
        }
        "DateCreated" => {
            items.sort_by(|a, b| {
                let res = a.date_created.cmp(&b.date_created);
                if descending {
                    res.reverse()
                } else {
                    res
                }
            });
        }
        "CommunityRating" => {
            items.sort_by(|a, b| {
                let res = a
                    .community_rating
                    .partial_cmp(&b.community_rating)
                    .unwrap_or(std::cmp::Ordering::Equal);
                if descending {
                    res.reverse()
                } else {
                    res
                }
            });
        }
        _ => {
            items.sort_by(|a, b| {
                let res = a
                    .sort_name
                    .as_ref()
                    .unwrap_or(&a.name)
                    .cmp(b.sort_name.as_ref().unwrap_or(&b.name));
                if descending {
                    res.reverse()
                } else {
                    res
                }
            });
        }
    }
    items
}

fn apply_item_pagination(items: Vec<BaseItemDto>, query_params: &HashMap<String, String>) -> (Vec<BaseItemDto>, i32) {
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
