use super::types::*;
use crate::collection::item::{Episode, Movie, Season, Show};
use crate::database::model::UserData;
use std::collections::HashMap;

pub const COLLECTION_ROOT_ID: &str = "e9d5075a555c1cbc394eec4cef295274";
pub const PLAYLIST_COLLECTION_ID: &str = "2f0340563593c4d98b97c9bfa21ce23c";
pub const FAVORITES_COLLECTION_ID: &str = "f4a0b1c2d3e5c4b8a9e6f7d8e9a0b1c2";

pub const ITEM_TYPE_USER_ROOT_FOLDER: &str = "UserRootFolder";
pub const ITEM_TYPE_COLLECTION_FOLDER: &str = "CollectionFolder";
pub const ITEM_TYPE_USER_VIEW: &str = "UserView";
pub const ITEM_TYPE_MOVIE: &str = "Movie";
pub const ITEM_TYPE_SHOW: &str = "Series";
pub const ITEM_TYPE_SEASON: &str = "Season";
pub const ITEM_TYPE_EPISODE: &str = "Episode";
pub const ITEM_TYPE_PLAYLIST: &str = "Playlist";

pub const ITEM_PREFIX_SEPARATOR: &str = "_";
pub const ITEM_PREFIX_ROOT: &str = "root_";
pub const ITEM_PREFIX_COLLECTION: &str = "collection_";
pub const ITEM_PREFIX_COLLECTION_FAVORITES: &str = "collectionfavorites_";
pub const ITEM_PREFIX_COLLECTION_PLAYLIST: &str = "collectionplaylist_";
pub const ITEM_PREFIX_SHOW: &str = "show_";
pub const ITEM_PREFIX_SEASON: &str = "season_";
pub const ITEM_PREFIX_EPISODE: &str = "episode_";
pub const ITEM_PREFIX_PLAYLIST: &str = "playlist_";
pub const ITEM_PREFIX_GENRE: &str = "genre_";
pub const ITEM_PREFIX_STUDIO: &str = "studio_";
pub const ITEM_PREFIX_PERSON: &str = "person_";

pub const TAG_PREFIX_REDIRECT: &str = "redirect_";

pub fn make_jf_root_id(root_id: &str) -> String {
    format!("{}{}", ITEM_PREFIX_ROOT, root_id)
}

pub fn make_jf_collection_id(collection_id: &str) -> String {
    format!("{}{}", ITEM_PREFIX_COLLECTION, collection_id)
}

pub fn make_jf_collection_favorites_id(favorites_id: &str) -> String {
    format!("{}{}", ITEM_PREFIX_COLLECTION_FAVORITES, favorites_id)
}

pub fn make_jf_collection_playlist_id(playlist_collection_id: &str) -> String {
    format!("{}{}", ITEM_PREFIX_COLLECTION_PLAYLIST, playlist_collection_id)
}

pub fn make_jf_playlist_id(playlist_id: &str) -> String {
    format!("{}{}", ITEM_PREFIX_PLAYLIST, playlist_id)
}

pub fn make_jf_season_id(season_id: &str) -> String {
    format!("{}{}", ITEM_PREFIX_SEASON, season_id)
}

pub fn make_jf_episode_id(episode_id: &str) -> String {
    format!("{}{}", ITEM_PREFIX_EPISODE, episode_id)
}

pub fn make_jf_genre_id(genre: &str) -> String {
    format!("{}{}", ITEM_PREFIX_GENRE, crate::idhash::id_hash(genre))
}

pub fn make_jf_studio_id(studio: &str) -> String {
    format!("{}{}", ITEM_PREFIX_STUDIO, crate::idhash::id_hash(studio))
}

pub fn make_jf_person_id(name: &str) -> String {
    format!("{}{}", ITEM_PREFIX_PERSON, crate::idhash::id_hash(name))
}

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

pub fn is_jf_playlist_id(id: &str) -> bool {
    id.starts_with(ITEM_PREFIX_PLAYLIST)
}

pub fn is_jf_show_id(id: &str) -> bool {
    id.starts_with(ITEM_PREFIX_SHOW)
}

pub fn is_jf_season_id(id: &str) -> bool {
    id.starts_with(ITEM_PREFIX_SEASON)
}

pub fn is_jf_episode_id(id: &str) -> bool {
    id.starts_with(ITEM_PREFIX_EPISODE)
}

pub fn is_jf_genre_id(id: &str) -> bool {
    id.starts_with(ITEM_PREFIX_GENRE)
}

pub fn is_jf_studio_id(id: &str) -> bool {
    id.starts_with(ITEM_PREFIX_STUDIO)
}

pub fn is_jf_person_id(id: &str) -> bool {
    id.starts_with(ITEM_PREFIX_PERSON)
}

pub fn item_is_hd(height: i32) -> bool {
    height >= 720
}

pub fn item_is_4k(height: i32) -> bool {
    height >= 1500
}

pub fn convert_movie_to_dto(movie: &Movie, server_id: &str, user_data: Option<&UserData>) -> BaseItemDto {
    let mut dto = BaseItemDto::default();
    dto.name = movie.name.clone();
    dto.id = movie.id.clone();
    dto.server_id = server_id.to_string();
    dto.item_type = "Movie".to_string();
    dto.path = Some(movie.file_path());
    dto.run_time_ticks = Some(movie.metadata.runtime_ticks());
    dto.production_year = movie.metadata.year;
    dto.overview = Some(movie.metadata.plot.clone());
    dto.sort_name = Some(movie.sort_name.clone());
    dto.premiere_date = movie.metadata.premiered;
    dto.media_type = Some("Video".to_string());
    dto.width = Some(movie.metadata.video_width);
    dto.height = Some(movie.metadata.video_height);
    dto.is_hd = Some(item_is_hd(movie.metadata.video_height));
    dto.is_4k = Some(item_is_4k(movie.metadata.video_height));

    // Genres
    if !movie.metadata.genres.is_empty() {
        dto.genres = Some(movie.metadata.genres.clone());
        dto.genre_items = Some(
            movie
                .metadata
                .genres
                .iter()
                .map(|g| GenreItem {
                    name: g.clone(),
                    id: crate::idhash::id_hash(g),
                })
                .collect(),
        );
    }

    // Studios
    if !movie.metadata.studios.is_empty() {
        dto.studios = Some(
            movie
                .metadata
                .studios
                .iter()
                .map(|s| StudioDto {
                    name: s.clone(),
                    id: crate::idhash::id_hash(s),
                })
                .collect(),
        );
    }

    // Images
    let mut image_tags = HashMap::new();
    if !movie.poster.is_empty() {
        image_tags.insert("Primary".to_string(), movie.id.clone());
    }
    if !movie.fanart.is_empty() {
        dto.backdrop_image_tags = Some(vec![movie.id.clone()]);
    }
    if !image_tags.is_empty() {
        dto.image_tags = Some(image_tags);
    }

    // User Data
    if let Some(ud) = user_data {
        dto.user_data = Some(make_jf_user_data(ud, &movie.id));
    }

    // Media Sources
    dto.media_sources = Some(vec![MediaSourceInfo {
        protocol: "File".to_string(),
        id: movie.id.clone(),
        path: movie.file_path(),
        encoder_path: None,
        encoder_protocol: None,
        r#type: "Default".to_string(),
        container: movie.file_name.split('.').last().unwrap_or("mp4").to_string(),
        size: movie.file_size,
        name: movie.name.clone(),
        is_remote: false,
        run_time_ticks: Some(movie.metadata.runtime_ticks()),
        supports_transcoding: true,
        supports_direct_stream: true,
        supports_direct_play: true,
        is_infinite_stream: false,
        requires_opening: false,
        open_token: None,
        requires_closing: false,
        live_stream_id: None,
        buffer_ms: None,
        requires_looping: false,
        supports_external_stream: true,
        media_streams: Vec::new(), // TODO: Populate if needed
        formats: Vec::new(),
        bitrate: None,
        timestamp: None,
        required_http_headers: None,
        transcoding_url: None,
        transcoding_sub_protocol: None,
        transcoding_container: None,
        analyze_duration_ms: None,
        default_audio_stream_index: Some(0),
        default_subtitle_stream_index: None,
    }]);

    dto
}

pub fn convert_show_to_dto(show: &Show, server_id: &str, user_data: Option<&UserData>) -> BaseItemDto {
    let mut dto = BaseItemDto::default();
    dto.name = show.name.clone();
    dto.id = show.id.clone();
    dto.server_id = server_id.to_string();
    dto.item_type = "Series".to_string();
    dto.path = Some(show.path.clone());
    dto.run_time_ticks = Some(show.duration().as_micros() as i64 * 10);
    dto.production_year = show.metadata.year;
    dto.overview = Some(show.metadata.plot.clone());
    dto.sort_name = Some(show.sort_name.clone());
    dto.premiere_date = show.metadata.premiered;
    dto.is_folder = Some(true);
    dto.child_count = Some(show.seasons.len() as i32);

    // Genres
    if !show.metadata.genres.is_empty() {
        dto.genres = Some(show.metadata.genres.clone());
        dto.genre_items = Some(
            show.metadata
                .genres
                .iter()
                .map(|g| GenreItem {
                    name: g.clone(),
                    id: crate::idhash::id_hash(g),
                })
                .collect(),
        );
    }

    // Images
    let mut image_tags = HashMap::new();
    if !show.poster.is_empty() {
        image_tags.insert("Primary".to_string(), show.id.clone());
    }
    if !show.fanart.is_empty() {
        dto.backdrop_image_tags = Some(vec![show.id.clone()]);
    }
    if !image_tags.is_empty() {
        dto.image_tags = Some(image_tags);
    }

    // User Data
    if let Some(ud) = user_data {
        dto.user_data = Some(make_jf_user_data(ud, &show.id));
    }

    dto
}

pub fn convert_season_to_dto(
    season: &Season,
    show: &Show,
    server_id: &str,
    user_data: Option<&UserData>,
) -> BaseItemDto {
    let mut dto = BaseItemDto::default();
    dto.name = season.name.clone();
    dto.id = season.id.clone();
    dto.server_id = server_id.to_string();
    dto.item_type = "Season".to_string();
    dto.path = Some(season.path.clone());
    dto.index_number = Some(season.season_no);
    dto.series_id = Some(show.id.clone());
    dto.series_name = Some(show.name.clone());
    dto.is_folder = Some(true);
    dto.child_count = Some(season.episodes.len() as i32);

    // Images
    let mut image_tags = HashMap::new();
    if !season.poster().is_empty() {
        image_tags.insert("Primary".to_string(), season.id.clone());
    }
    if !image_tags.is_empty() {
        dto.image_tags = Some(image_tags);
    }

    // User Data
    if let Some(ud) = user_data {
        dto.user_data = Some(make_jf_user_data(ud, &season.id));
    }

    dto
}

pub fn convert_episode_to_dto(
    episode: &Episode,
    show: &Show,
    server_id: &str,
    user_data: Option<&UserData>,
) -> BaseItemDto {
    let mut dto = BaseItemDto::default();
    dto.name = episode.name.clone();
    dto.id = episode.id.clone();
    dto.server_id = server_id.to_string();
    dto.item_type = "Episode".to_string();
    dto.path = Some(format!("{}/{}", episode.path, episode.file_name));
    dto.run_time_ticks = Some(episode.metadata.runtime_ticks());
    dto.index_number = Some(episode.episode_no);
    dto.parent_index_number = Some(episode.season_no);
    dto.series_id = Some(show.id.clone());
    dto.series_name = Some(show.name.clone());
    dto.overview = Some(episode.metadata.plot.clone());
    dto.media_type = Some("Video".to_string());
    dto.width = Some(episode.metadata.video_width);
    dto.height = Some(episode.metadata.video_height);
    dto.is_hd = Some(item_is_hd(episode.metadata.video_height));
    dto.is_4k = Some(item_is_4k(episode.metadata.video_height));

    // Find season ID
    if let Some(season) = show.seasons.iter().find(|s| s.season_no == episode.season_no) {
        dto.season_id = Some(season.id.clone());
        dto.season_name = Some(season.name.clone());
    }

    // Images
    let mut image_tags = HashMap::new();
    if !episode.thumb.is_empty() {
        image_tags.insert("Primary".to_string(), episode.id.clone());
    }
    if !image_tags.is_empty() {
        dto.image_tags = Some(image_tags);
    }

    // User Data
    if let Some(ud) = user_data {
        dto.user_data = Some(make_jf_user_data(ud, &episode.id));
    }

    // Media Sources
    dto.media_sources = Some(vec![MediaSourceInfo {
        protocol: "File".to_string(),
        id: episode.id.clone(),
        path: format!("{}/{}", episode.path, episode.file_name),
        encoder_path: None,
        encoder_protocol: None,
        r#type: "Default".to_string(),
        container: episode.file_name.split('.').last().unwrap_or("mp4").to_string(),
        size: episode.file_size,
        name: episode.name.clone(),
        is_remote: false,
        run_time_ticks: Some(episode.metadata.runtime_ticks()),
        supports_transcoding: true,
        supports_direct_stream: true,
        supports_direct_play: true,
        is_infinite_stream: false,
        requires_opening: false,
        open_token: None,
        requires_closing: false,
        live_stream_id: None,
        buffer_ms: None,
        requires_looping: false,
        supports_external_stream: true,
        media_streams: Vec::new(),
        formats: Vec::new(),
        bitrate: None,
        timestamp: None,
        required_http_headers: None,
        transcoding_url: None,
        transcoding_sub_protocol: None,
        transcoding_container: None,
        analyze_duration_ms: None,
        default_audio_stream_index: Some(0),
        default_subtitle_stream_index: None,
    }]);

    dto
}

pub fn make_jf_user_data(ud: &UserData, item_id: &str) -> UserItemDataDto {
    UserItemDataDto {
        rating: None,
        played_percentage: Some(ud.played_percentage as f64),
        unplayed_item_count: None,
        playback_position_ticks: ud.position * 10_000_000,
        play_count: ud.play_count,
        is_favorite: ud.favorite,
        likes: None,
        last_played_date: Some(ud.timestamp),
        played: ud.played,
        key: item_id.to_string(),
        item_id: "00000000000000000000000000000000".to_string(),
    }
}

pub fn make_jf_item_genre(genre: &str, server_id: &str) -> BaseItemDto {
    let id = make_jf_genre_id(genre);
    BaseItemDto {
        id: id.clone(),
        name: genre.to_string(),
        server_id: server_id.to_string(),
        item_type: "Genre".to_string(),
        etag: Some(id),
        ..BaseItemDto::default()
    }
}

pub fn make_jf_item_studio(studio: &str, server_id: &str) -> BaseItemDto {
    let id = make_jf_studio_id(studio);
    BaseItemDto {
        id: id.clone(),
        name: studio.to_string(),
        server_id: server_id.to_string(),
        item_type: "Studio".to_string(),
        etag: Some(id),
        ..BaseItemDto::default()
    }
}

pub fn make_jf_item_person(person: &crate::database::model::Person, server_id: &str) -> BaseItemDto {
    let id = make_jf_person_id(&person.name);
    let mut dto = BaseItemDto {
        id: id.clone(),
        name: person.name.clone(),
        server_id: server_id.to_string(),
        item_type: "Person".to_string(),
        etag: Some(id.clone()),
        overview: Some(person.bio.clone()),
        date_created: Some(person.date_of_birth),
        premiere_date: Some(person.date_of_birth),
        location_type: Some("FileSystem".to_string()),
        media_type: Some("Unknown".to_string()),
        play_access: Some("Full".to_string()),
        ..BaseItemDto::default()
    };

    if !person.place_of_birth.is_empty() {
        dto.production_locations = Some(vec![person.place_of_birth.clone()]);
    }

    if !person.poster_url.is_empty() {
        let mut image_tags = HashMap::new();
        image_tags.insert(
            "Primary".to_string(),
            format!("{}{}", TAG_PREFIX_REDIRECT, person.poster_url),
        );
        dto.image_tags = Some(image_tags);
    }

    dto.user_data = Some(UserItemDataDto {
        key: format!("Person-{}", person.name),
        item_id: id,
        ..UserItemDataDto::default()
    });

    dto
}
