use std::collections::HashMap;

use anyhow::{anyhow, bail, Context, Result};
use chrono::Utc;
use tracing::warn;

use super::id::Id;
use super::jellyfin::JellyfinState;
use super::types::*;
use crate::collection::item::{Episode, Movie, Season, Show};
use crate::collection::{CollectionType, Item};
use crate::database::UserData as DbUserData;
use crate::idhash::id_hash;

type JFItem = BaseItemDto;

// Top-level root ID, parent ID of all collections
const COLLECTION_ROOT_ID: &str = "e9d5075a555c1cbc394eec4cef295274";
// ID of dynamically generated Playlist collection
const PLAYLIST_COLLECTION_ID: &str = "2f0340563593c4d98b97c9bfa21ce23c";
// ID of dynamically generated favorites collection
const FAVORITES_COLLECTION_ID: &str = "f4a0b1c2d3e5c4b8a9e6f7d8e9a0b1c2";

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
const ITEM_TYPE_GENRE: &str = "Genre";
const ITEM_TYPE_STUDIO: &str = "Studio";
#[allow(dead_code)]
const ITEM_TYPE_PERSON: &str = "Person";

const TICKS_TO_SECONDS: i64 = 10_000_000;

// imagetag prefix will get HTTP-redirected
#[allow(dead_code)]
const TAG_PREFIX_REDIRECT: &str = "redirect_";
// imagetag prefix means we will serve the filename from local disk
#[allow(dead_code)]
const TAG_PREFIX_FILE: &str = "file_";

// ---------------------------------------------------------------------------
// Main API functions
// ---------------------------------------------------------------------------

/// get_jfitems_by_parent_id returns a list of all items with a specific parent_id.
pub async fn get_jfitems_by_parent_id(
    state: &JellyfinState,
    user_id: &str,
    parent_id: &str,
) -> Result<Vec<JFItem>> {
    // List favorites collection items requested?
    if is_jf_collection_favorites_id(parent_id) {
        return make_jfitem_favorites_overview(state, user_id)
            .await
            .with_context(|| "could not find favorites collection");
    }

    // List of playlists requested?
    if is_jf_collection_playlist_id(parent_id) {
        return make_jfitem_playlist_overview(state, user_id)
            .await
            .with_context(|| "could not find playlist collection");
    }

    // Specific playlist requested?
    if is_jf_playlist_id(parent_id) {
        let playlist_id = trim_prefix(parent_id);
        return make_jfitem_playlist_itemlist(state, user_id, playlist_id)
            .await
            .with_context(|| "could not find playlist");
    }

    // List by genre requested?
    if is_jf_genre_id(parent_id) {
        let items = get_jfitems_all(state, user_id)
            .await
            .with_context(|| "could not get all items")?;
        let mut genre_items = Vec::new();
        for item in &items {
            for genre in &item.genre_items {
                if genre.id == parent_id {
                    genre_items.push(item.clone());
                    break;
                }
            }
        }
        return Ok(genre_items);
    }

    // List by studio?
    if is_jf_studio_id(parent_id) {
        let items = get_jfitems_all(state, user_id)
            .await
            .with_context(|| "could not get all items")?;
        let mut studio_items = Vec::new();
        for item in &items {
            for studio in &item.studios {
                if studio.id == parent_id {
                    studio_items.push(item.clone());
                    break;
                }
            }
        }
        return Ok(studio_items);
    }

    // Specific collection requested?
    if is_jf_collection_id(parent_id) {
        let collection_id = trim_prefix(parent_id);
        let c = state
            .collections
            .get_collection(collection_id)
            .ok_or_else(|| anyhow!("could not find collection"))?;
        let mut items = Vec::new();
        for item in &c.items {
            match make_jfitem_light(state, user_id, item, &c.id).await {
                Ok(jfitem) => items.push(jfitem),
                Err(e) => warn!("get_jfitems_by_parent_id: {}", e),
            }
        }
        return Ok(items);
    }

    // Check if parent_id is a show or season to generate overviews
    let internal_id = trim_prefix(parent_id);
    if let Some((_, item)) = state.collections.get_item_by_id(internal_id) {
        match item {
            Item::Show(show) => {
                return make_jfitem_seasons_overview(state, user_id, &show)
                    .await
                    .with_context(|| "could not find parent show");
            }
            Item::Season(season) => {
                return make_jfitem_episodes_overview(state, user_id, &season)
                    .await
                    .with_context(|| "could not find season");
            }
            _ => {
                warn!(
                    "get_jfitems_by_parent_id: unsupported parent_id {}",
                    parent_id
                );
                bail!("unsupported parent_id type");
            }
        }
    }

    bail!("parent_id not found")
}

/// get_jfitems_all returns list of all items across all collections.
/// Uses lightweight DTOs — skips expensive per-item DB lookups (user_data, media_sources).
pub async fn get_jfitems_all(state: &JellyfinState, user_id: &str) -> Result<Vec<JFItem>> {
    let mut items = Vec::new();
    for c in state.collections.get_collections() {
        for item in &c.items {
            match make_jfitem_light(state, user_id, item, &c.id).await {
                Ok(jfitem) => items.push(jfitem),
                Err(e) => warn!("get_jfitems_all: {}", e),
            }
        }
    }
    Ok(items)
}

/// make_jfitem_by_id creates a JFItem based on the provided item_id.
pub async fn make_jfitem_by_id(
    state: &JellyfinState,
    user_id: &str,
    item_id: &str,
) -> Result<JFItem> {
    // Handle special items first
    if is_jf_root_id(item_id) {
        return make_jfitem_root(state, user_id).await;
    }
    // Try special collection items first, as they have the same prefix as regular collections
    if is_jf_collection_favorites_id(item_id) {
        return make_jfitem_collection_favorites(state, user_id).await;
    }
    if is_jf_collection_playlist_id(item_id) {
        return make_jfitem_collection_playlist(state, user_id).await;
    }
    if is_jf_collection_id(item_id) {
        return make_jfitem_collection(state, trim_prefix(item_id));
    }
    if is_jf_playlist_id(item_id) {
        return make_jfitem_playlist(state, user_id, trim_prefix(item_id)).await;
    }

    // Try to fetch individual item: movie, show, season, episode
    let internal_id = trim_prefix(item_id);
    let (c, item) = state
        .collections
        .get_item_by_id(internal_id)
        .ok_or_else(|| anyhow!("item not found"))?;
    make_jfitem(state, user_id, &item, &c.id).await
}

/// make_jfitem dispatches to the correct make function based on item type.
pub async fn make_jfitem(
    state: &JellyfinState,
    user_id: &str,
    item: &Item,
    parent_id: &str,
) -> Result<JFItem> {
    make_jfitem_inner(state, user_id, item, parent_id, false).await
}

/// make_jfitem_light builds a lightweight DTO that skips expensive computations
/// (user_data DB lookups, media source generation, per-episode iteration for shows).
/// Used for list queries where the client only needs base fields.
pub async fn make_jfitem_light(
    state: &JellyfinState,
    user_id: &str,
    item: &Item,
    parent_id: &str,
) -> Result<JFItem> {
    make_jfitem_inner(state, user_id, item, parent_id, true).await
}

async fn make_jfitem_inner(
    state: &JellyfinState,
    user_id: &str,
    item: &Item,
    parent_id: &str,
    lightweight: bool,
) -> Result<JFItem> {
    match item {
        Item::Movie(m) => make_jfitem_movie(state, user_id, m, parent_id, lightweight).await,
        Item::Show(s) => make_jfitem_show(state, user_id, s, parent_id, lightweight).await,
        Item::Season(s) => make_jfitem_season(state, user_id, s).await,
        Item::Episode(e) => make_jfitem_episode(state, user_id, e).await,
    }
}

// ---------------------------------------------------------------------------
// Collection / root functions
// ---------------------------------------------------------------------------

/// make_jfitem_root creates the root folder item.
pub async fn make_jfitem_root(state: &JellyfinState, user_id: &str) -> Result<JFItem> {
    let child_count = make_jfcollection_root_overview(state, user_id)
        .await
        .map(|c| c.len() as i32)
        .ok();

    let genres = state.collections.details().genres;

    #[rustfmt::skip]
    let item = JFItem {
        name:                        "Media Folders".to_string(),
        id:                          make_jf_root_id(COLLECTION_ROOT_ID),
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

/// make_jfcollection_root_overview creates a list of items representing
/// the collections available to the user.
pub async fn make_jfcollection_root_overview(
    state: &JellyfinState,
    user_id: &str,
) -> Result<Vec<JFItem>> {
    let mut items = Vec::new();
    for c in state.collections.get_collections() {
        if let Ok(item) = make_jfitem_collection(state, &c.id) {
            items.push(item);
        }
    }
    // Add favorites and playlist collections
    if let Ok(item) = make_jfitem_collection_favorites(state, user_id).await {
        items.push(item);
    }
    if let Ok(item) = make_jfitem_collection_playlist(state, user_id).await {
        items.push(item);
    }
    Ok(items)
}

/// make_jfitem_collection creates a collection folder item.
pub fn make_jfitem_collection(state: &JellyfinState, collection_id: &str) -> Result<JFItem> {
    let c = state
        .collections
        .get_collection(collection_id)
        .ok_or_else(|| anyhow!("collection not found"))?;

    let collection_genres = c.details().genres;
    let coll_type = match c.collection_type {
        CollectionType::Movies => COLLECTION_TYPE_MOVIES,
        CollectionType::Shows => COLLECTION_TYPE_TVSHOWS,
    };

    let id = Id::new(collection_id).as_jf_collection();
    let parent_id = make_jf_root_id(COLLECTION_ROOT_ID);

    #[rustfmt::skip]
    let item = JFItem {
        name:                        c.name.clone(),
        server_id:                   state.server_id.clone(),
        id,
        parent_id:                   Some(parent_id),
        etag:                        Some(id_hash(collection_id)),
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
        display_preferences_id:      Some(make_jf_display_preferences_id(collection_id)),
        play_access:                 Some("Full".to_string()),
        enable_media_source_display: Some(true),
        primary_image_aspect_ratio:  Some(1.7777777777777777),
        child_count:                 Some(c.items.len() as i32),
        genre_items:                 make_jf_genre_items(&collection_genres),
        genres:                      collection_genres,
        sort_name:                   Some(coll_type.to_string()),
        collection_type:             Some(coll_type.to_string()),
        ..Default::default()
    };
    Ok(item)
}

/// make_jfitem_collection_favorites creates a collection item for the favorites folder.
pub async fn make_jfitem_collection_favorites(
    state: &JellyfinState,
    user_id: &str,
) -> Result<JFItem> {
    let item_count = state
        .repo
        .get_favorites(user_id)
        .await
        .map(|f| f.len() as i32)
        .ok();

    let id = make_jf_collection_favorites_id(FAVORITES_COLLECTION_ID);

    #[rustfmt::skip]
    let item = JFItem {
        name:                        "Favorites".to_string(),
        server_id:                   state.server_id.clone(),
        id,
        parent_id:                   Some(make_jf_root_id(COLLECTION_ROOT_ID)),
        etag:                        Some(id_hash(FAVORITES_COLLECTION_ID)),
        date_created:                Some(Utc::now()),
        premiere_date:               Some(Utc::now()),
        collection_type:             Some(COLLECTION_TYPE_PLAYLISTS.to_string()),
        sort_name:                   Some(COLLECTION_TYPE_PLAYLISTS.to_string()),
        item_type:                   ITEM_TYPE_USER_VIEW.to_string(),
        is_folder:                   true,
        enable_media_source_display: Some(true),
        child_count:                 item_count,
        display_preferences_id:      Some(make_jf_display_preferences_id(FAVORITES_COLLECTION_ID)),
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
    Ok(item)
}

/// make_jfitem_favorites_overview creates a list of favorite items.
async fn make_jfitem_favorites_overview(
    state: &JellyfinState,
    user_id: &str,
) -> Result<Vec<JFItem>> {
    let favorite_ids = state.repo.get_favorites(user_id).await?;
    let mut items = Vec::new();
    for item_id in &favorite_ids {
        if let Some((c, item)) = state.collections.get_item_by_id(item_id) {
            // We only add movies and shows in favorites
            match &item {
                Item::Movie(_) | Item::Show(_) => {
                    match make_jfitem(state, user_id, &item, &c.id).await {
                        Ok(jfitem) => items.push(jfitem),
                        Err(e) => warn!("make_jfitem_favorites_overview: {}", e),
                    }
                }
                _ => {}
            }
        }
    }
    Ok(items)
}

/// make_jfitem_collection_playlist creates a top level collection item
/// representing all playlists of the user.
pub async fn make_jfitem_collection_playlist(
    state: &JellyfinState,
    user_id: &str,
) -> Result<JFItem> {
    let mut item_count = 0i32;
    if let Ok(playlist_ids) = state.repo.get_playlists(user_id).await {
        for id in &playlist_ids {
            if let Ok(playlist) = state.repo.get_playlist(user_id, id).await {
                item_count += playlist.item_ids.len() as i32;
            }
        }
    }

    let id = make_jf_collection_playlist_id(PLAYLIST_COLLECTION_ID);

    #[rustfmt::skip]
    let item = JFItem {
        name:                        "Playlists".to_string(),
        server_id:                   state.server_id.clone(),
        id,
        parent_id:                   Some(make_jf_root_id(COLLECTION_ROOT_ID)),
        etag:                        Some(id_hash(PLAYLIST_COLLECTION_ID)),
        date_created:                Some(Utc::now()),
        premiere_date:               Some(Utc::now()),
        collection_type:             Some(COLLECTION_TYPE_PLAYLISTS.to_string()),
        sort_name:                   Some(COLLECTION_TYPE_PLAYLISTS.to_string()),
        item_type:                   ITEM_TYPE_USER_VIEW.to_string(),
        is_folder:                   true,
        enable_media_source_display: Some(true),
        child_count:                 Some(item_count),
        display_preferences_id:      Some(make_jf_display_preferences_id(PLAYLIST_COLLECTION_ID)),
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
    Ok(item)
}

/// make_jfitem_playlist creates a playlist item.
async fn make_jfitem_playlist(
    state: &JellyfinState,
    user_id: &str,
    playlist_id: &str,
) -> Result<JFItem> {
    let playlist = state.repo.get_playlist(user_id, playlist_id).await?;

    #[rustfmt::skip]
    let item = JFItem {
        item_type:                   ITEM_TYPE_PLAYLIST.to_string(),
        id:                          make_jf_playlist_id(&playlist.id),
        parent_id:                   Some(make_jf_collection_playlist_id(PLAYLIST_COLLECTION_ID)),
        server_id:                   state.server_id.clone(),
        name:                        playlist.name.clone(),
        sort_name:                   Some(playlist.name.clone()),
        is_folder:                   true,
        path:                        Some("/playlist".to_string()),
        etag:                        Some(id_hash(&playlist.id)),
        date_created:                Some(Utc::now()),
        can_delete:                  Some(true),
        can_download:                Some(true),
        play_access:                 Some("Full".to_string()),
        recursive_item_count:        Some(playlist.item_ids.len() as i64),
        child_count:                 Some(playlist.item_ids.len() as i32),
        location_type:               Some("FileSystem".to_string()),
        media_type:                  Some("Video".to_string()),
        display_preferences_id:      Some(make_jf_display_preferences_id(PLAYLIST_COLLECTION_ID)),
        enable_media_source_display: Some(true),
        ..Default::default()
    };
    Ok(item)
}

/// make_jfitem_playlist_overview creates a list of playlists of the user.
pub async fn make_jfitem_playlist_overview(
    state: &JellyfinState,
    user_id: &str,
) -> Result<Vec<JFItem>> {
    let playlist_ids = state.repo.get_playlists(user_id).await?;
    let mut items = Vec::new();
    for id in &playlist_ids {
        if let Ok(playlist_item) = make_jfitem_playlist(state, user_id, id).await {
            items.push(playlist_item);
        }
    }
    Ok(items)
}

/// make_jfitem_playlist_itemlist creates an item list of one playlist of the user.
async fn make_jfitem_playlist_itemlist(
    state: &JellyfinState,
    user_id: &str,
    playlist_id: &str,
) -> Result<Vec<JFItem>> {
    let playlist = state.repo.get_playlist(user_id, playlist_id).await?;
    let mut items = Vec::new();
    for item_id in &playlist.item_ids {
        if let Some((c, item)) = state.collections.get_item_by_id(item_id) {
            match make_jfitem(state, user_id, &item, &c.id).await {
                Ok(jfitem) => items.push(jfitem),
                Err(e) => warn!("make_jfitem_playlist_itemlist: {}", e),
            }
        }
    }
    Ok(items)
}

// ---------------------------------------------------------------------------
// Item conversion functions
// ---------------------------------------------------------------------------

/// make_jfitem_movie creates a movie item.
async fn make_jfitem_movie(
    state: &JellyfinState,
    user_id: &str,
    movie: &Movie,
    parent_id: &str,
    lightweight: bool,
) -> Result<JFItem> {
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

    let (media_sources, media_streams) = if lightweight {
        (Vec::new(), Vec::new())
    } else {
        let ms = make_media_source(
            &movie.id,
            &movie.file_name,
            movie.file_size,
            &movie.metadata,
        );
        let streams = ms
            .first()
            .map(|s| s.media_streams.clone())
            .unwrap_or_default();
        (ms, streams)
    };

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

    let user_data = if lightweight {
        UserItemDataDto::default()
    } else {
        get_user_data(state, user_id, &movie.id).await
    };

    #[rustfmt::skip]
    let item = JFItem {
        name,
        id:                          movie.id.clone(),
        server_id:                   state.server_id.clone(),
        item_type:                   ITEM_TYPE_MOVIE.to_string(),
        parent_id:                   Some(make_jf_collection_id(parent_id)),
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
        user_data:                   Some(user_data),
        ..Default::default()
    };
    Ok(item)
}

/// make_jfitem_show creates a show item.
async fn make_jfitem_show(
    state: &JellyfinState,
    user_id: &str,
    show: &Show,
    parent_id: &str,
    lightweight: bool,
) -> Result<JFItem> {
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

    // User data: in lightweight mode, skip the expensive per-episode DB iteration
    let user_data = if lightweight {
        UserItemDataDto::default()
    } else {
        let mut ud = get_user_data(state, user_id, &show.id).await;

        // Calculate the number of played episodes in the show
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
                ud.played_percentage =
                    Some(100.0 * played_episodes as f64 / total_episodes as f64);
                ud.last_played_date = Some(latest_played);
                ud.key = show.id.clone();
                if played_episodes == total_episodes {
                    ud.played = true;
                }
            }
        }
        ud
    };

    #[rustfmt::skip]
    let item = JFItem {
        name,
        id:                          show.id.clone(),
        server_id:                   state.server_id.clone(),
        item_type:                   ITEM_TYPE_SHOW.to_string(),
        parent_id:                   Some(make_jf_collection_id(parent_id)),
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
        production_year:             show.metadata.year,
        primary_image_aspect_ratio:  Some(0.6666666666666666),
        can_delete:                  Some(false),
        can_download:                Some(true),
        play_access:                 Some("Full".to_string()),
        image_tags,
        backdrop_image_tags:         vec![show.id.clone()],
        overview:                    show.metadata.plot.clone(),
        official_rating:             show.metadata.official_rating.clone(),
        community_rating:            show.metadata.rating,
        taglines:                    show.metadata.taglines.clone(),
        child_count:                 Some(child_count),
        recursive_item_count:        Some(recursive_item_count),
        user_data:                   Some(user_data),
        ..Default::default()
    };
    Ok(item)
}

/// make_jfitem_seasons_overview generates all season items for a show.
pub async fn make_jfitem_seasons_overview(
    state: &JellyfinState,
    user_id: &str,
    show: &Show,
) -> Result<Vec<JFItem>> {
    let mut seasons = Vec::with_capacity(show.seasons.len());
    for s in &show.seasons {
        match make_jfitem_season(state, user_id, s).await {
            Ok(jfitem) => seasons.push(jfitem),
            Err(e) => warn!("make_jfitem_seasons_overview: {}", e),
        }
    }

    // Sort seasons by index number. Specials (season 0) get index 99, ending up last.
    seasons.sort_by_key(|s| s.index_number.unwrap_or(0));

    Ok(seasons)
}

/// make_jfitem_season creates a season item.
async fn make_jfitem_season(
    state: &JellyfinState,
    user_id: &str,
    season: &Season,
) -> Result<JFItem> {
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
    let premiere_date = season
        .episodes
        .first()
        .and_then(|e| e.metadata.premiered);

    // Image tags
    let mut image_tags = HashMap::new();
    if !season.poster().is_empty() {
        image_tags.insert(
            "Primary".to_string(),
            make_jf_season_id(&season.id),
        );
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
        user_data.played_percentage =
            Some(100.0 * played_episodes as f64 / child_count as f64);
    }
    user_data.last_played_date = Some(latest_played);
    if played_episodes == child_count && child_count > 0 {
        user_data.played = true;
    }

    #[rustfmt::skip]
    let item = JFItem {
        name,
        id:                     make_jf_season_id(&season.id),
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

/// make_jfitem_episodes_overview generates all episode items for one season.
pub async fn make_jfitem_episodes_overview(
    state: &JellyfinState,
    user_id: &str,
    season: &Season,
) -> Result<Vec<JFItem>> {
    let mut episodes = Vec::with_capacity(season.episodes.len());
    for e in &season.episodes {
        match make_jfitem_episode(state, user_id, e).await {
            Ok(jfitem) => episodes.push(jfitem),
            Err(e) => warn!("make_jfitem_episodes_overview: {}", e),
        }
    }
    Ok(episodes)
}

/// make_jfitem_episode creates an episode item.
pub async fn make_jfitem_episode(
    state: &JellyfinState,
    user_id: &str,
    episode: &Episode,
) -> Result<JFItem> {
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
    let item = JFItem {
        name,
        id:                     make_jf_episode_id(&episode.id),
        server_id:              state.server_id.clone(),
        item_type:              ITEM_TYPE_EPISODE.to_string(),
        season_id:              Some(make_jf_season_id(&season.id)),
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

/// make_jfitem_genre creates a genre item.
pub fn make_jfitem_genre(state: &JellyfinState, genre: &str) -> JFItem {
    let genre_id = make_jf_genre_id(genre);

    // Try to get actual genre item count from collections
    let mut child_count = 1;
    for c in state.collections.get_collections() {
        let counts = c.genre_count();
        if let Some(&count) = counts.get(genre) {
            child_count = count as i32;
        }
    }

    JFItem {
        id: genre_id.clone(),
        server_id: state.server_id.clone(),
        item_type: ITEM_TYPE_GENRE.to_string(),
        name: genre.to_string(),
        sort_name: Some(genre.to_string()),
        etag: Some(genre_id),
        date_created: Some(Utc::now()),
        premiere_date: Some(Utc::now()),
        location_type: Some("FileSystem".to_string()),
        media_type: Some("Unknown".to_string()),
        child_count: Some(child_count),
        ..Default::default()
    }
}

/// make_jfitem_studio creates a studio item.
pub fn make_jfitem_studio(state: &JellyfinState, studio: &str) -> JFItem {
    let studio_id = make_jf_studio_id(studio);

    JFItem {
        id: studio_id.clone(),
        server_id: state.server_id.clone(),
        item_type: ITEM_TYPE_STUDIO.to_string(),
        name: studio.to_string(),
        sort_name: Some(studio.to_string()),
        etag: Some(studio_id),
        date_created: Some(Utc::now()),
        premiere_date: Some(Utc::now()),
        location_type: Some("FileSystem".to_string()),
        media_type: Some("Unknown".to_string()),
        user_data: Some(UserItemDataDto::default()),
        ..Default::default()
    }
}

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------

/// get_user_data fetches user data from the database, returning a default if not found.
async fn get_user_data(
    state: &JellyfinState,
    user_id: &str,
    item_id: &str,
) -> UserItemDataDto {
    let db_data = state.repo.get_user_data(user_id, item_id).await.ok();
    make_jf_userdata(user_id, item_id, db_data.as_ref())
}

/// make_jf_genre_items converts a list of genre names into NameGuidPairs.
fn make_jf_genre_items(genres: &[String]) -> Vec<NameGuidPair> {
    genres
        .iter()
        .map(|g| NameGuidPair {
            name: g.clone(),
            id: make_jf_genre_id(g),
        })
        .collect()
}

/// make_jf_studio_pairs converts a list of studio names into NameGuidPairs.
fn make_jf_studio_pairs(studios: &[String]) -> Vec<NameGuidPair> {
    studios
        .iter()
        .map(|s| NameGuidPair {
            name: s.clone(),
            id: make_jf_studio_id(s),
        })
        .collect()
}

/// make_jf_userdata creates a UserItemDataDto, populating from DbUserData if provided.
pub fn make_jf_userdata(
    user_id: &str,
    item_id: &str,
    data: Option<&DbUserData>,
) -> UserItemDataDto {
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
pub(super) fn make_media_source(
    item_id: &str,
    file_name: &str,
    file_size: i64,
    metadata: &crate::collection::Metadata,
) -> Vec<MediaSourceInfo> {
    let container = file_name
        .rsplit('.')
        .next()
        .unwrap_or("mp4")
        .to_string();

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

// ---------------------------------------------------------------------------
// ID helper functions
// ---------------------------------------------------------------------------

const ITEM_PREFIX_SEPARATOR: &str = "_";
const ITEM_PREFIX_ROOT: &str = "root_";
const ITEM_PREFIX_COLLECTION: &str = "collection_";
const ITEM_PREFIX_COLLECTION_FAVORITES: &str = "collectionfavorites_";
const ITEM_PREFIX_COLLECTION_PLAYLIST: &str = "collectionplaylist_";
#[allow(dead_code)]
const ITEM_PREFIX_SHOW: &str = "show_";
const ITEM_PREFIX_SEASON: &str = "season_";
const ITEM_PREFIX_EPISODE: &str = "episode_";
const ITEM_PREFIX_PLAYLIST: &str = "playlist_";
const ITEM_PREFIX_GENRE: &str = "genre_";
const ITEM_PREFIX_STUDIO: &str = "studio_";
#[allow(dead_code)]
const ITEM_PREFIX_PERSON: &str = "person_";
const ITEM_PREFIX_DISPLAY_PREFERENCES: &str = "dp_";

fn make_jf_root_id(root_id: &str) -> String {
    format!("{}{}", ITEM_PREFIX_ROOT, root_id)
}

fn make_jf_collection_id(collection_id: &str) -> String {
    format!("{}{}", ITEM_PREFIX_COLLECTION, collection_id)
}

fn make_jf_collection_favorites_id(favorites_id: &str) -> String {
    format!("{}{}", ITEM_PREFIX_COLLECTION_FAVORITES, favorites_id)
}

fn make_jf_collection_playlist_id(playlist_collection_id: &str) -> String {
    format!("{}{}", ITEM_PREFIX_COLLECTION_PLAYLIST, playlist_collection_id)
}

fn make_jf_playlist_id(playlist_id: &str) -> String {
    format!("{}{}", ITEM_PREFIX_PLAYLIST, playlist_id)
}

fn make_jf_season_id(season_id: &str) -> String {
    format!("{}{}", ITEM_PREFIX_SEASON, season_id)
}

fn make_jf_episode_id(episode_id: &str) -> String {
    format!("{}{}", ITEM_PREFIX_EPISODE, episode_id)
}

fn make_jf_display_preferences_id(dp_id: &str) -> String {
    format!("{}{}", ITEM_PREFIX_DISPLAY_PREFERENCES, dp_id)
}

pub fn make_jf_genre_id(genre: &str) -> String {
    format!("{}{}", ITEM_PREFIX_GENRE, id_hash(genre))
}

fn make_jf_studio_id(studio: &str) -> String {
    format!("{}{}", ITEM_PREFIX_STUDIO, id_hash(studio))
}

#[allow(dead_code)]
fn make_jf_person_id(name: &str) -> String {
    format!("{}{}", ITEM_PREFIX_PERSON, id_hash(name))
}

/// trim_prefix removes the type prefix from an item id.
pub fn trim_prefix(s: &str) -> &str {
    if let Some(pos) = s.find(ITEM_PREFIX_SEPARATOR) {
        &s[pos + 1..]
    } else {
        s
    }
}

pub fn is_jf_root_id(id: &str) -> bool {
    id.starts_with(ITEM_PREFIX_ROOT)
}

pub fn is_jf_collection_id(id: &str) -> bool {
    id.starts_with(ITEM_PREFIX_COLLECTION)
}

pub fn is_jf_collection_favorites_id(id: &str) -> bool {
    id.starts_with(ITEM_PREFIX_COLLECTION_FAVORITES)
}

pub fn is_jf_collection_playlist_id(id: &str) -> bool {
    id == make_jf_collection_playlist_id(PLAYLIST_COLLECTION_ID)
}

fn is_jf_playlist_id(id: &str) -> bool {
    id.starts_with(ITEM_PREFIX_PLAYLIST)
}

#[allow(dead_code)]
fn is_jf_show_id(id: &str) -> bool {
    id.starts_with(ITEM_PREFIX_SHOW)
}

#[allow(dead_code)]
fn is_jf_season_id(id: &str) -> bool {
    id.starts_with(ITEM_PREFIX_SEASON)
}

#[allow(dead_code)]
fn is_jf_episode_id(id: &str) -> bool {
    id.starts_with(ITEM_PREFIX_EPISODE)
}

fn is_jf_genre_id(id: &str) -> bool {
    id.starts_with(ITEM_PREFIX_GENRE)
}

fn is_jf_studio_id(id: &str) -> bool {
    id.starts_with(ITEM_PREFIX_STUDIO)
}

#[allow(dead_code)]
fn is_jf_person_id(id: &str) -> bool {
    id.starts_with(ITEM_PREFIX_PERSON)
}
