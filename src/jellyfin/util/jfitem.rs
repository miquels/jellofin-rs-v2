use std::collections::HashMap;

use anyhow::{anyhow, Result};
use chrono::Utc;
use tracing::warn;

use super::jellyfin::JellyfinState;
use super::types::*;
use crate::collection::item::{CollectionFolder, Episode, Movie, PlaylistItem, Season, Show, UserView};
use crate::collection::{CollectionType, Item};
use crate::database::UserData as DbUserData;
use crate::idhash::*;

const COLLECTION_TYPE_MOVIES: &str = "movies";
const COLLECTION_TYPE_TVSHOWS: &str = "tvshows";
const COLLECTION_TYPE_PLAYLISTS: &str = "playlists";

const ITEM_TYPE_USER_ROOT_FOLDER: &str = "UserRootFolder";
const ITEM_TYPE_COLLECTION_FOLDER: &str = "CollectionFolder";
const ITEM_TYPE_USER_VIEW: &str = "UserView";
const ITEM_TYPE_MOVIE: &str = "Movie";
const ITEM_TYPE_SHOW: &str = "Series";
const ITEM_TYPE_SEASON: &str = "Season";
const ITEM_TYPE_EPISODE: &str = "Episode";
const ITEM_TYPE_PLAYLIST: &str = "Playlist";

const TICKS_TO_SECONDS: i64 = 10_000_000;

// ---------------------------------------------------------------------------
// Item query helpers — collect and convert native Items for the pipeline
// ---------------------------------------------------------------------------

/// Check query params to determine if user_data needs to be loaded for filtering/sorting.
pub fn needs_user_data(query_params: &HashMap<String, String>) -> bool {
    if query_params.contains_key("isPlayed") || query_params.contains_key("isFavorite") {
        return true;
    }
    if let Some(filters) = query_params.get("filters") {
        if filters.contains("IsFavorite") || filters.contains("IsFavoriteOrLikes") {
            return true;
        }
    }
    if let Some(sort_by) = query_params.get("sortBy") {
        let lower = sort_by.to_lowercase();
        if lower.contains("dateplayed")
            || lower.contains("isplayed")
            || lower.contains("isunplayed")
            || lower.contains("isfavoriteorliked")
        {
            return true;
        }
    }
    false
}

/// Load user_data from the database for each Item that doesn't already have it.
pub async fn load_user_data(items: &mut [Item], state: &JellyfinState, user_id: &str) {
    for item in items.iter_mut() {
        if item.get_user_data().is_none() {
            if let Ok(ud) = state.repo.get_user_data(user_id, &item.id()).await {
                item.set_user_data(ud);
            }
        }
    }
}

/// Look up items by a list of IDs across all collections.
pub fn get_items_by_ids(state: &JellyfinState, ids: Vec<&str>) -> Result<Vec<Item>> {
    let mut items = Vec::new();
    for id in ids {
        if let Some((_, item)) = state.collections.get_item_by_id(id) {
            items.push(item);
        }
    }
    Ok(items)
}

/// Collect all items from a specific collection.
/// When `recursive` is true, Shows are flattened to include their Seasons and Episodes.
pub fn get_items_by_collection(
    state: &JellyfinState,
    collection_id: &str,
    recursive: bool,
) -> Result<Vec<Item>> {
    let c = state
        .collections
        .get_collection(collection_id)
        .ok_or_else(|| anyhow!("could not find collection"))?;
    if !recursive {
        return Ok(c.items);
    }
    let mut items = Vec::new();
    for item in c.items {
        match item {
            Item::Show(show) => {
                for season in &show.seasons {
                    items.push(Item::Season(season.clone()));
                    for episode in &season.episodes {
                        items.push(Item::Episode(episode.clone()));
                    }
                }
                items.push(Item::Show(show));
            }
            other => items.push(other),
        }
    }
    Ok(items)
}

/// Collect all items across all collections.
pub fn get_items_all(state: &JellyfinState) -> Vec<Item> {
    let mut items = Vec::new();
    for c in state.collections.get_collections() {
        items.extend(c.items);
    }
    items
}

/// Get root overview items (collections + favorites + playlists) as native Items.
pub async fn get_root_overview_items(state: &JellyfinState, user_id: &str) -> Vec<Item> {
    let mut items = Vec::new();

    for c in state.collections.get_collections() {
        items.push(Item::CollectionFolder(CollectionFolder {
            id: c.id.clone(),
            name: c.name.clone(),
            collection_type: c.collection_type,
            child_count: c.items.len() as i32,
            genres: c.details().genres,
        }));
    }

    // Favorites
    let fav_count = state
        .repo
        .get_favorites(user_id)
        .await
        .map(|f| f.len() as i32)
        .ok();
    items.push(Item::UserView(UserView {
        id: String::from(FAVORITES_COLLECTION_ID),
        name: "Favorites".to_string(),
        collection_type: COLLECTION_TYPE_PLAYLISTS.to_string(),
        child_count: fav_count,
    }));

    // Playlists
    let mut playlist_item_count = 0i32;
    if let Ok(playlist_ids) = state.repo.get_playlists(user_id).await {
        for id in &playlist_ids {
            if let Ok(playlist) = state.repo.get_playlist(user_id, id).await {
                playlist_item_count += playlist.item_ids.len() as i32;
            }
        }
    }
    items.push(Item::UserView(UserView {
        id: String::from(PLAYLIST_COLLECTION_ID),
        name: "Playlists".to_string(),
        collection_type: COLLECTION_TYPE_PLAYLISTS.to_string(),
        child_count: Some(playlist_item_count),
    }));

    items
}

/// Get favorite items as native Items.
pub async fn get_favorites_items(state: &JellyfinState, user_id: &str) -> Vec<Item> {
    let favorite_ids = match state.repo.get_favorites(user_id).await {
        Ok(ids) => ids,
        Err(_) => return Vec::new(),
    };
    let mut items = Vec::new();
    for item_id in &favorite_ids {
        if let Some((_, item)) = state.collections.get_item_by_id(item_id) {
            match &item {
                Item::Movie(_) | Item::Show(_) => items.push(item),
                _ => {}
            }
        }
    }
    items
}

/// Get all playlists for a user as native Items.
pub async fn get_playlist_overview_items(state: &JellyfinState, user_id: &str) -> Vec<Item> {
    let playlist_ids = match state.repo.get_playlists(user_id).await {
        Ok(ids) => ids,
        Err(_) => return Vec::new(),
    };
    let mut items = Vec::new();
    for id in &playlist_ids {
        if let Ok(playlist) = state.repo.get_playlist(user_id, id).await {
            items.push(Item::Playlist(PlaylistItem {
                id: playlist.id.clone(),
                name: playlist.name.clone(),
                child_count: playlist.item_ids.len() as i32,
            }));
        }
    }
    items
}

/// Get items in a specific playlist as native Items.
pub async fn get_playlist_items_native(
    state: &JellyfinState,
    user_id: &str,
    playlist_id: &str,
) -> Result<Vec<Item>> {
    let playlist = state.repo.get_playlist(user_id, playlist_id).await?;
    let mut items = Vec::new();
    for item_id in &playlist.item_ids {
        if let Some((_, item)) = state.collections.get_item_by_id(item_id) {
            items.push(item);
        }
    }
    Ok(items)
}

/// Get seasons of a show as native Items.
pub fn get_seasons_items(state: &JellyfinState, show_id: &str) -> Result<Vec<Item>> {
    match state.collections.get_item_by_id(show_id) {
        Some((_, Item::Show(show))) => Ok(show.seasons.iter().map(|s| Item::Season(s.clone())).collect()),
        _ => Err(anyhow!("show not found")),
    }
}

/// Get episodes of a season as native Items.
pub fn get_episodes_items(state: &JellyfinState, season_id: &str) -> Result<Vec<Item>> {
    match state.collections.get_season_by_id(season_id) {
        Some((_collection, _show, season)) => {
            Ok(season.episodes.iter().map(|e| Item::Episode(e.clone())).collect())
        }
        _ => Err(anyhow!("season not found")),
    }
}

/// Get all episodes across all seasons of a show as native Items.
pub fn get_show_all_episodes(state: &JellyfinState, show_id: &str) -> Result<Vec<Item>> {
    match state.collections.get_item_by_id(show_id) {
        Some((_, Item::Show(show))) => {
            let mut items = Vec::new();
            for season in &show.seasons {
                for episode in &season.episodes {
                    items.push(Item::Episode(episode.clone()));
                }
            }
            Ok(items)
        }
        _ => Err(anyhow!("show not found")),
    }
}

/// Convert a slice of Items to BaseItemDtos.
pub async fn convert_items_to_dtos(items: &[Item], state: &JellyfinState, user_id: &str) -> Vec<BaseItemDto> {
    let mut dtos = Vec::with_capacity(items.len());
    for item in items {
        match make_jfitem(state, user_id, item).await {
            Ok(dto) => dtos.push(dto),
            Err(e) => warn!("convert_items_to_dtos: {}", e),
        }
    }
    dtos
}

/// make_jfitem dispatches to the correct make function based on item type.
pub async fn make_jfitem(state: &JellyfinState, user_id: &str, item: &Item) -> Result<BaseItemDto> {
    match item {
        Item::Movie(m) => make_jfitem_movie(state, user_id, m).await,
        Item::Show(s) => make_jfitem_show(state, user_id, s).await,
        Item::Season(s) => make_jfitem_season(state, user_id, s).await,
        Item::Episode(e) => make_jfitem_episode(state, user_id, e).await,
        Item::CollectionFolder(cf) => Ok(make_jfitem_from_collection_folder(state, cf)),
        Item::UserView(uv) => Ok(make_jfitem_from_user_view(state, uv)),
        Item::Playlist(pl) => Ok(make_jfitem_from_playlist(state, pl)),
    }
}

// ---------------------------------------------------------------------------
// Collection / root functions
// ---------------------------------------------------------------------------

/// make_jfitem_root creates the root folder item.
pub async fn make_jfitem_root(state: &JellyfinState, user_id: &str) -> Result<BaseItemDto> {
    let child_count = Some(get_root_overview_items(state, user_id).await.len() as i32);

    let genres = state.collections.details().genres;

    #[rustfmt::skip]
    let item = BaseItemDto {
        name:                        "Media Folders".to_string(),
        id:                          String::from(COLLECTION_ROOT_ID),
        server_id:                   state.server_id.clone(),
        item_type:                   ITEM_TYPE_USER_ROOT_FOLDER.to_string(),
        etag:                        Some(id_hash(COLLECTION_ROOT_ID)),
        date_created:                Some(Utc::now()),
        is_folder:                   true,
        can_delete:                  Some(false),
        can_download:                Some(false),
        sort_name:                   Some("media folders".to_string()),
        path:                        Some("/root".to_string()),
        enable_media_source_display: Some(true),
        genre_items:                 make_jf_genre_items(&genres),
        genres,
        child_count:                 child_count,
        display_preferences_id:      Some(make_jf_display_preferences_id(COLLECTION_ROOT_ID)),
        primary_image_aspect_ratio:  Some(1.7777777777777777),
        location_type:               Some("FileSystem".to_string()),
        media_type:                  Some("Unknown".to_string()),
        ..Default::default()
    };
    Ok(item)
}

/// Convert a native CollectionFolder to a BaseItemDto.
fn make_jfitem_from_collection_folder(state: &JellyfinState, cf: &CollectionFolder) -> BaseItemDto {
    let coll_type = match cf.collection_type {
        CollectionType::Movies => COLLECTION_TYPE_MOVIES,
        CollectionType::Shows => COLLECTION_TYPE_TVSHOWS,
    };

    #[rustfmt::skip]
    let item = BaseItemDto {
        name:                        cf.name.clone(),
        server_id:                   state.server_id.clone(),
        id:                          cf.id.clone(),
        parent_id:                   Some(String::from(COLLECTION_ROOT_ID)),
        etag:                        Some(id_hash(&cf.id)),
        date_created:                Some(Utc::now()),
        premiere_date:               Some(Utc::now()),
        item_type:                   ITEM_TYPE_COLLECTION_FOLDER.to_string(),
        is_folder:                   true,
        location_type:               Some("FileSystem".to_string()),
        path:                        Some("/collection".to_string()),
        lock_data:                   Some(false),
        media_type:                  Some("Unknown".to_string()),
        can_delete:                  Some(false),
        can_download:                Some(true),
        display_preferences_id:      Some(make_jf_display_preferences_id(&cf.id)),
        play_access:                 Some("Full".to_string()),
        enable_media_source_display: Some(true),
        primary_image_aspect_ratio:  Some(1.7777777777777777),
        child_count:                 Some(cf.child_count),
        genre_items:                 make_jf_genre_items(&cf.genres),
        genres:                      cf.genres.clone(),
        sort_name:                   Some(coll_type.to_string()),
        collection_type:             Some(coll_type.to_string()),
        ..Default::default()
    };
    item
}

/// Convert a native UserView to a BaseItemDto.
fn make_jfitem_from_user_view(state: &JellyfinState, uv: &UserView) -> BaseItemDto {
    #[rustfmt::skip]
    let item = BaseItemDto {
        name:                        uv.name.clone(),
        server_id:                   state.server_id.clone(),
        id:                          uv.id.clone(),
        parent_id:                   Some(String::from(COLLECTION_ROOT_ID)),
        etag:                        Some(id_hash(&uv.id)),
        date_created:                Some(Utc::now()),
        premiere_date:               Some(Utc::now()),
        collection_type:             Some(uv.collection_type.clone()),
        sort_name:                   Some(uv.collection_type.clone()),
        item_type:                   ITEM_TYPE_USER_VIEW.to_string(),
        is_folder:                   true,
        enable_media_source_display: Some(true),
        child_count:                 uv.child_count,
        display_preferences_id:      Some(make_jf_display_preferences_id(&uv.id)),
        play_access:                 Some("Full".to_string()),
        primary_image_aspect_ratio:  Some(1.7777777777777777),
        location_type:               Some("FileSystem".to_string()),
        path:                        Some("/collection".to_string()),
        lock_data:                   Some(false),
        media_type:                  Some("Unknown".to_string()),
        can_delete:                  Some(false),
        can_download:                Some(true),
        ..Default::default()
    };
    item
}

/// Convert a native PlaylistItem to a BaseItemDto.
fn make_jfitem_from_playlist(state: &JellyfinState, pl: &PlaylistItem) -> BaseItemDto {
    #[rustfmt::skip]
    let item = BaseItemDto {
        item_type:                   ITEM_TYPE_PLAYLIST.to_string(),
        id:                          pl.id.clone(),
        parent_id:                   Some(String::from(PLAYLIST_COLLECTION_ID)),
        server_id:                   state.server_id.clone(),
        name:                        pl.name.clone(),
        sort_name:                   Some(pl.name.clone()),
        is_folder:                   true,
        path:                        Some("/playlist".to_string()),
        etag:                        Some(id_hash(&pl.id)),
        date_created:                Some(Utc::now()),
        can_delete:                  Some(true),
        can_download:                Some(true),
        play_access:                 Some("Full".to_string()),
        recursive_item_count:        Some(pl.child_count as i64),
        child_count:                 Some(pl.child_count),
        location_type:               Some("FileSystem".to_string()),
        media_type:                  Some("Video".to_string()),
        display_preferences_id:      Some(make_jf_display_preferences_id(PLAYLIST_COLLECTION_ID)),
        enable_media_source_display: Some(true),
        ..Default::default()
    };
    item
}

// ---------------------------------------------------------------------------
// Item conversion functions
// ---------------------------------------------------------------------------

/// make_jfitem_movie creates a movie item.
async fn make_jfitem_movie(state: &JellyfinState, user_id: &str, movie: &Movie) -> Result<BaseItemDto> {
    let genres = movie.metadata.genres.clone();
    let genre_items = make_jf_genre_items(&genres);

    // Metadata might have a better title
    let name = movie
        .metadata
        .title
        .as_deref()
        .filter(|t| !t.is_empty())
        .unwrap_or(&movie.name)
        .to_string();

    // Set premiere date from metadata if available, else from file timestamp
    let premiere_date = movie.metadata.premiered.unwrap_or(movie.created);

    let media_sources = make_media_source(&movie.id, &movie.file_name, movie.file_size, &movie.metadata);
    let media_streams = media_sources
        .first()
        .map(|s| s.media_streams.clone())
        .unwrap_or_default();

    // Image tags
    let mut image_tags = HashMap::new();
    if !movie.poster.is_empty() {
        image_tags.insert("Primary".to_string(), movie.id.clone());
    }
    if !movie.fanart.is_empty() {
        image_tags.insert("Backdrop".to_string(), movie.id.clone());
    }
    if !movie.banner.is_empty() {
        image_tags.insert("Banner".to_string(), movie.id.clone());
    }

    let user_data = Some(get_user_data(state, user_id, &movie.id).await);

    #[rustfmt::skip]
    let item = BaseItemDto {
        name,
        id:                          movie.id.clone(),
        server_id:                   state.server_id.clone(),
        item_type:                   ITEM_TYPE_MOVIE.to_string(),
        parent_id:                   Some(movie.collection_id.clone()),
        original_title:              Some(movie.name.clone()),
        sort_name:                   Some(movie.sort_name.clone()),
        forced_sort_name:            Some(movie.sort_name.clone()),
        genres,
        genre_items,
        studios:                     make_jf_studio_pairs(&movie.metadata.studios),
        is_hd:                       item_is_hd(&movie.metadata),
        is_4k:                       item_is_4k(&movie.metadata),
        run_time_ticks:              make_runtime_ticks_from_metadata(&movie.metadata),
        location_type:               Some("FileSystem".to_string()),
        path:                        Some("file.mp4".to_string()),
        etag:                        Some(id_hash(&movie.id)),
        media_type:                  Some("Video".to_string()),
        video_type:                  Some("VideoFile".to_string()),
        container:                   Some("mov,mp4,m4a".to_string()),
        date_created:                Some(movie.created),
        premiere_date:               Some(premiere_date),
        primary_image_aspect_ratio:  Some(0.6666666666666666),
        can_delete:                  Some(false),
        can_download:                Some(true),
        play_access:                 Some("Full".to_string()),
        image_tags,
        backdrop_image_tags:         vec![movie.id.clone()],
        width:                       movie.metadata.video_width,
        height:                      movie.metadata.video_height,
        overview:                    movie.metadata.plot.clone(),
        official_rating:             movie.metadata.official_rating.clone(),
        community_rating:            movie.metadata.rating,
        production_year:             movie.metadata.year,
        taglines:                    movie.metadata.taglines.clone(),
        has_subtitles:               !movie.srt_subs.is_empty() || !movie.vtt_subs.is_empty(),
        media_sources,
        media_streams,
        user_data,
        ..Default::default()
    };
    Ok(item)
}

/// make_jfitem_show creates a show item.
async fn make_jfitem_show(state: &JellyfinState, user_id: &str, show: &Show) -> Result<BaseItemDto> {
    let genres = show.metadata.genres.clone();
    let genre_items = make_jf_genre_items(&genres);

    // Metadata might have a better title
    let name = show
        .metadata
        .title
        .as_deref()
        .filter(|t| !t.is_empty())
        .unwrap_or(&show.name)
        .to_string();

    // Set premiere date from metadata if available, else from first video timestamp
    let premiere_date = show.metadata.premiered.unwrap_or(show.first_video);

    // Image tags
    let mut image_tags = HashMap::new();
    if !show.poster.is_empty() {
        image_tags.insert("Primary".to_string(), show.id.clone());
    }
    if !show.fanart.is_empty() {
        image_tags.insert("Backdrop".to_string(), show.id.clone());
    }
    if !show.banner.is_empty() {
        image_tags.insert("Banner".to_string(), show.id.clone());
    }
    if !show.logo.is_empty() {
        image_tags.insert("Logo".to_string(), show.id.clone());
    }

    let child_count = show.seasons.len() as i32;

    // Calculate recursive item count (total episodes)
    let mut recursive_item_count: i64 = 0;
    for s in &show.seasons {
        recursive_item_count += s.episodes.len() as i64;
    }

    // Calculate user play state across all episodes
    let user_data = {
        let mut ud = get_user_data(state, user_id, &show.id).await;

        if child_count > 0 {
            let mut played_episodes = 0i32;
            let mut total_episodes = 0i32;
            let mut latest_played = chrono::DateTime::<Utc>::default();

            for s in &show.seasons {
                for e in &s.episodes {
                    total_episodes += 1;
                    if let Ok(ep_data) = state.repo.get_user_data(user_id, &e.id).await {
                        if ep_data.played {
                            played_episodes += 1;
                            if ep_data.timestamp > latest_played {
                                latest_played = ep_data.timestamp;
                            }
                        }
                    }
                }
            }

            if total_episodes > 0 {
                ud.unplayed_item_count = Some(total_episodes - played_episodes);
                ud.played_percentage = Some(100.0 * played_episodes as f64 / total_episodes as f64);
                ud.last_played_date = Some(latest_played);
                ud.key = show.id.clone();
                if played_episodes == total_episodes {
                    ud.played = true;
                }
            }
        }
        Some(ud)
    };

    #[rustfmt::skip]
    let item = BaseItemDto {
        name,
        id:                          show.id.clone(),
        server_id:                   state.server_id.clone(),
        item_type:                   ITEM_TYPE_SHOW.to_string(),
        parent_id:                   Some(show.collection_id.clone()),
        original_title:              Some(show.name.clone()),
        sort_name:                   Some(show.sort_name.clone()),
        forced_sort_name:            Some(show.sort_name.clone()),
        genres,
        genre_items,
        studios:                     make_jf_studio_pairs(&show.metadata.studios),
        is_folder:                   true,
        etag:                        Some(id_hash(&show.id)),
        date_created:                Some(show.first_video),
        premiere_date:               Some(premiere_date),
        primary_image_aspect_ratio:  Some(0.6666666666666666),
        can_delete:                  Some(false),
        can_download:                Some(true),
        play_access:                 Some("Full".to_string()),
        image_tags,
        backdrop_image_tags:         vec![show.id.clone()],
        overview:                    show.metadata.plot.clone(),
        official_rating:             show.metadata.official_rating.clone(),
        community_rating:            show.metadata.rating,
        production_year:             show.metadata.year,
        taglines:                    show.metadata.taglines.clone(),
        child_count:                 Some(child_count),
        recursive_item_count:        Some(recursive_item_count),
        user_data,
        ..Default::default()
    };
    Ok(item)
}

/// make_jfitem_season creates a season item.
async fn make_jfitem_season(state: &JellyfinState, user_id: &str, season: &Season) -> Result<BaseItemDto> {
    // Look up the full season + show context
    let (_collection, show, season) = state
        .collections
        .get_season_by_id(&season.id)
        .ok_or_else(|| anyhow!("could not find season"))?;

    let child_count = season.episodes.len() as i32;

    // Season numbering: 0 = specials (displayed as season 99 for sorting)
    let season_number = season.season_no;
    let (index_number, name, sort_name) = if season_number != 0 {
        (
            season_number,
            make_season_name(season_number),
            format!("{:04}", season_number),
        )
    } else {
        (99, make_season_name(0), "9999".to_string())
    };

    // Season premiere date from first episode
    let premiere_date = season.episodes.first().and_then(|e| e.metadata.premiered);

    // Image tags
    let mut image_tags = HashMap::new();
    if !season.poster().is_empty() {
        image_tags.insert("Primary".to_string(), season.id.clone());
    }

    // Get playstate of the season itself
    let mut user_data = get_user_data(state, user_id, &season.id).await;

    // Calculate the number of played episodes in the season
    let mut played_episodes = 0i32;
    let mut latest_played = chrono::DateTime::<Utc>::default();
    for e in &season.episodes {
        if let Ok(ep_data) = state.repo.get_user_data(user_id, &e.id).await {
            if ep_data.played {
                played_episodes += 1;
                if ep_data.timestamp > latest_played {
                    latest_played = ep_data.timestamp;
                }
            }
        }
    }

    // Populate playstate fields
    user_data.unplayed_item_count = Some(child_count - played_episodes);
    if child_count > 0 {
        user_data.played_percentage = Some(100.0 * played_episodes as f64 / child_count as f64);
    }
    user_data.last_played_date = Some(latest_played);
    if played_episodes == child_count && child_count > 0 {
        user_data.played = true;
    }

    #[rustfmt::skip]
    let item = BaseItemDto {
        name,
        id:                     String::from(&season.id),
        server_id:              state.server_id.clone(),
        item_type:              ITEM_TYPE_SEASON.to_string(),
        series_id:              Some(show.id.clone()),
        series_name:            Some(show.name.clone()),
        parent_id:              Some(show.id.clone()),
        parent_logo_item_id:    Some(show.id.clone()),
        is_folder:              true,
        location_type:          Some("FileSystem".to_string()),
        etag:                   Some(id_hash(&season.id)),
        media_type:             Some("Unknown".to_string()),
        child_count:            Some(child_count),
        recursive_item_count:   Some(child_count as i64),
        date_created:           Some(Utc::now()),
        premiere_date,
        can_delete:             Some(false),
        can_download:           Some(true),
        play_access:            Some("Full".to_string()),
        index_number:           Some(index_number),
        sort_name:              Some(sort_name),
        image_tags,
        user_data:              Some(user_data),
        ..Default::default()
    };
    Ok(item)
}

/// make_jfitem_episode creates an episode item.
pub async fn make_jfitem_episode(
    state: &JellyfinState,
    user_id: &str,
    episode: &Episode,
) -> Result<BaseItemDto> {
    // Look up the full episode + season + show context
    let (_collection, show, season, episode) = state
        .collections
        .get_episode_by_id(&episode.id)
        .ok_or_else(|| anyhow!("could not find episode"))?;

    // Metadata might have a better title
    let name = episode
        .metadata
        .title
        .as_deref()
        .filter(|t| !t.is_empty())
        .unwrap_or(&episode.name)
        .to_string();

    // Get genres from episode, if not available use show genres
    let genres = if episode.metadata.genres.is_empty() {
        show.metadata.genres.clone()
    } else {
        episode.metadata.genres.clone()
    };
    let genre_items = make_jf_genre_items(&genres);

    // Get studios from episode, if not available use show studios
    let studios = if episode.metadata.studios.is_empty() {
        &show.metadata.studios
    } else {
        &episode.metadata.studios
    };

    let premiere_date = if show.metadata.premiered.is_some() {
        show.metadata.premiered
    } else {
        Some(episode.created)
    };

    let media_sources = make_media_source(
        &episode.id,
        &episode.file_name,
        episode.file_size,
        &episode.metadata,
    );
    let media_streams = media_sources
        .first()
        .map(|ms| ms.media_streams.clone())
        .unwrap_or_default();

    // Image tags
    let mut image_tags = HashMap::new();
    if !episode.thumb.is_empty() {
        image_tags.insert("Primary".to_string(), episode.id.clone());
    }

    let user_data = get_user_data(state, user_id, &episode.id).await;

    #[rustfmt::skip]
    let item = BaseItemDto {
        name,
        id:                     episode.id.clone(),
        server_id:              state.server_id.clone(),
        item_type:              ITEM_TYPE_EPISODE.to_string(),
        season_id:              Some(season.id.clone()),
        season_name:            Some(make_season_name(season.season_no)),
        series_id:              Some(show.id.clone()),
        series_name:            Some(show.name.clone()),
        parent_logo_item_id:    Some(show.id.clone()),
        parent_index_number:    Some(season.season_no),
        index_number:           Some(episode.episode_no),
        overview:               episode.metadata.plot.clone(),
        is_hd:                  item_is_hd(&episode.metadata),
        is_4k:                  item_is_4k(&episode.metadata),
        run_time_ticks:         make_runtime_ticks_from_metadata(&episode.metadata),
        location_type:          Some("FileSystem".to_string()),
        path:                   Some("episode.mp4".to_string()),
        etag:                   Some(id_hash(&episode.id)),
        media_type:             Some("Video".to_string()),
        video_type:             Some("VideoFile".to_string()),
        container:              Some("mov,mp4,m4a".to_string()),
        date_created:           Some(episode.created),
        premiere_date,
        has_subtitles:          !episode.srt_subs.is_empty() || !episode.vtt_subs.is_empty(),
        can_delete:             Some(false),
        can_download:           Some(true),
        play_access:            Some("Full".to_string()),
        width:                  episode.metadata.video_width,
        height:                 episode.metadata.video_height,
        production_year:        episode.metadata.year,
        community_rating:       episode.metadata.rating,
        genres,
        genre_items,
        studios:                make_jf_studio_pairs(studios),
        image_tags,
        media_sources,
        media_streams,
        user_data:              Some(user_data),
        ..Default::default()
    };
    Ok(item)
}

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------

/// get_user_data fetches user data from the database, returning a default if not found.
async fn get_user_data(state: &JellyfinState, user_id: &str, item_id: &str) -> UserItemDataDto {
    let db_data = state.repo.get_user_data(user_id, item_id).await.ok();
    make_jf_userdata(user_id, item_id, db_data.as_ref())
}

/// make_jf_genre_items converts a list of genre names into NameGuidPairs.
fn make_jf_genre_items(genres: &[String]) -> Vec<NameGuidPair> {
    genres
        .iter()
        .map(|g| NameGuidPair {
            name: g.clone(),
            id: id_hash_prefix(ITEM_PREFIX_GENRE, g),
        })
        .collect()
}

/// make_jf_studio_pairs converts a list of studio names into NameGuidPairs.
fn make_jf_studio_pairs(studios: &[String]) -> Vec<NameGuidPair> {
    studios
        .iter()
        .map(|s| NameGuidPair {
            name: s.clone(),
            id: id_hash_prefix(ITEM_PREFIX_STUDIO, s),
        })
        .collect()
}

/// make_jf_userdata creates a UserItemDataDto, populating from DbUserData if provided.
pub fn make_jf_userdata(user_id: &str, item_id: &str, data: Option<&DbUserData>) -> UserItemDataDto {
    let mut ud = UserItemDataDto {
        key: format!("{}/{}", user_id, item_id),
        item_id: "00000000000000000000000000000000".to_string(),
        ..Default::default()
    };
    if let Some(p) = data {
        ud.is_favorite = p.favorite;
        ud.last_played_date = Some(p.timestamp);
        ud.playback_position_ticks = p.position * TICKS_TO_SECONDS;
        ud.played_percentage = Some(p.played_percentage as f64);
        ud.played = p.played;
    }
    ud
}

/// make_media_source creates the media source info for an item.
pub(crate) fn make_media_source(
    item_id: &str,
    file_name: &str,
    file_size: i64,
    metadata: &crate::collection::Metadata,
) -> Vec<MediaSourceInfo> {
    let container = file_name.rsplit('.').next().unwrap_or("mp4").to_string();

    let runtime_ticks = metadata.runtime_ticks();
    let bitrate = metadata
        .video_bitrate
        .unwrap_or(0)
        .checked_add(metadata.audio_bitrate.unwrap_or(0));
    let media_streams = make_jf_media_streams(metadata);

    vec![MediaSourceInfo {
        id: item_id.to_string(),
        etag: Some(id_hash(file_name)),
        name: file_name.to_string(),
        path: file_name.to_string(),
        r#type: "Default".to_string(),
        container,
        protocol: "File".to_string(),
        video_type: Some("VideoFile".to_string()),
        size: file_size,
        is_remote: false,
        supports_transcoding: false,
        supports_direct_stream: true,
        supports_direct_play: true,
        supports_external_stream: true,
        is_infinite_stream: false,
        requires_opening: false,
        requires_closing: false,
        requires_looping: false,
        supports_probing: Some(true),
        transcoding_sub_protocol: Some("http".to_string()),
        run_time_ticks: runtime_ticks,
        bitrate,
        media_streams,
        default_audio_stream_index: Some(1),
        formats: Vec::new(),
        ..Default::default()
    }]
}

/// make_jf_media_streams creates media stream information from metadata.
fn make_jf_media_streams(metadata: &crate::collection::Metadata) -> Vec<MediaStream> {
    // Video stream
    let video_codec = metadata
        .video_codec
        .as_deref()
        .unwrap_or("unknown")
        .to_lowercase();
    let (codec, codec_tag) = match video_codec.as_str() {
        "avc" | "x264" | "h264" => ("h264", Some("avc1")),
        "x265" | "h265" | "hevc" => ("hevc", Some("hvc1")),
        "vc1" => ("vc1", Some("wvc1")),
        _ => ("unknown", Some("unknown")),
    };
    let video_title = codec.to_uppercase();
    let video_display_title = format!("{} - SDR", video_title);

    let video_stream = MediaStream {
        index: 0,
        stream_type: "Video".to_string(),
        is_default: true,
        language: metadata.audio_language.clone(),
        average_frame_rate: metadata.video_frame_rate.map(|f| f as f32),
        real_frame_rate: metadata.video_frame_rate.map(|f| f as f32),
        ref_frames: Some(1),
        time_base: Some("1/16000".to_string()),
        height: metadata.video_height,
        width: metadata.video_width,
        codec: codec.to_string(),
        codec_tag: codec_tag.map(|s| s.to_string()),
        aspect_ratio: Some("2.35:1".to_string()),
        video_range: Some("SDR".to_string()),
        video_range_type: Some("SDR".to_string()),
        profile: Some("High".to_string()),
        is_anamorphic: Some(false),
        bit_depth: Some(8),
        bit_rate: metadata.video_bitrate,
        audio_spatial_format: Some("None".to_string()),
        title: Some(video_title),
        display_title: Some(video_display_title),
        is_interlaced: false,
        is_forced: false,
        is_external: false,
        is_text_subtitle_stream: false,
        supports_external_stream: false,
        ..Default::default()
    };

    // Audio stream
    let audio_channels = metadata.audio_channels.unwrap_or(2);
    let (audio_title, channel_layout) = match audio_channels {
        1 => ("Mono", "mono"),
        2 => ("Stereo", "stereo"),
        3 => ("2.1 Channel", "3.0"),
        4 => ("3.1 Channel", "4.0"),
        5 => ("4.1 Channel", "5.0"),
        6 => ("5.1 Channel", "5.1"),
        8 => ("7.1 Channel", "7.1"),
        _ => ("Unknown", "unknown"),
    };

    let audio_codec_str = metadata
        .audio_codec
        .as_deref()
        .unwrap_or("unknown")
        .to_lowercase();
    let (a_codec, a_codec_tag) = match audio_codec_str.as_str() {
        "ac3" => ("ac3", Some("ac-3")),
        "aac" => ("aac", Some("mp4a")),
        "wma" => ("wmapro", None),
        _ => ("unknown", None),
    };
    let audio_display_title = format!("{} - {}", audio_title, a_codec.to_uppercase());

    let audio_stream = MediaStream {
        index: 1,
        stream_type: "Audio".to_string(),
        is_default: true,
        language: metadata.audio_language.clone(),
        time_base: Some("1/48000".to_string()),
        sample_rate: Some(48000),
        audio_spatial_format: Some("None".to_string()),
        localized_default: Some("Default".to_string()),
        localized_external: Some("External".to_string()),
        is_interlaced: false,
        is_avc: Some(false),
        video_range: Some("Unknown".to_string()),
        video_range_type: Some("Unknown".to_string()),
        profile: Some("LC".to_string()),
        bit_rate: metadata.audio_bitrate,
        channels: Some(audio_channels),
        channel_layout: Some(channel_layout.to_string()),
        codec: a_codec.to_string(),
        codec_tag: a_codec_tag.map(|s| s.to_string()),
        title: Some(audio_title.to_string()),
        display_title: Some(audio_display_title),
        is_forced: false,
        is_external: false,
        is_text_subtitle_stream: false,
        supports_external_stream: false,
        ..Default::default()
    };

    vec![video_stream, audio_stream]
}

/// make_runtime_ticks_from_metadata converts metadata duration to Jellyfin runtime ticks.
fn make_runtime_ticks_from_metadata(metadata: &crate::collection::Metadata) -> Option<i64> {
    metadata.runtime_ticks()
}

/// make_season_name returns a human-readable season name.
fn make_season_name(season_no: i32) -> String {
    if season_no != 0 {
        format!("Season {}", season_no)
    } else {
        "Specials".to_string()
    }
}

/// item_is_hd checks if the item is HD (720p or higher).
fn item_is_hd(metadata: &crate::collection::Metadata) -> bool {
    metadata.video_height.map(|h| h >= 720).unwrap_or(false)
}

/// item_is_4k checks if the item is 4K (2160p or higher).
fn item_is_4k(metadata: &crate::collection::Metadata) -> bool {
    metadata.video_height.map(|h| h >= 1500).unwrap_or(false)
}
