use std::path::{Path, PathBuf};
use std::time::Duration;
use tracing::info;
use walkdir::WalkDir;

use super::collection::Collection;
use super::item::{Episode, Item, Movie, Season, Show};
use super::metadata::Metadata;
use crate::idhash::id_hash;

/// Build movies collection by scanning directory
pub fn build_movies(collection: &mut Collection, _scan_interval: Duration) {
    info!("Scanning movies in: {}", collection.directory);

    let mut movies = Vec::new();

    // Walk the directory looking for movie folders
    for entry in WalkDir::new(&collection.directory)
        .max_depth(2)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if !entry.file_type().is_dir() {
            continue;
        }

        let path = entry.path();
        if let Some(movie) = scan_movie_directory(path, &collection.directory) {
            movies.push(Item::Movie(movie));
        }
    }

    info!("Found {} movies in {}", movies.len(), collection.name);
    collection.items = movies;
}

/// Build shows collection by scanning directory
pub fn build_shows(collection: &mut Collection, _scan_interval: Duration) {
    info!("Scanning shows in: {}", collection.directory);

    let mut shows = Vec::new();

    // Walk the directory looking for show folders
    for entry in WalkDir::new(&collection.directory)
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if !entry.file_type().is_dir() {
            continue;
        }

        let path = entry.path();
        if path == Path::new(&collection.directory) {
            continue; // Skip root directory
        }

        if let Some(show) = scan_show_directory(path, &collection.directory) {
            shows.push(Item::Show(show));
        }
    }

    info!("Found {} shows in {}", shows.len(), collection.name);
    collection.items = shows;
}

/// Scan a movie directory for video files and metadata
fn scan_movie_directory(path: &Path, collection_root: &str) -> Option<Movie> {
    let dir_name = path.file_name()?.to_str()?;

    // Find video file
    let video_file = find_video_file(path)?;
    let video_path = video_file.strip_prefix(collection_root).ok()?;

    // Generate ID from directory name
    let id = id_hash(dir_name);

    // Get relative path
    let relative_path = path.strip_prefix(collection_root).ok()?.to_str()?.to_string();

    let movie = Movie {
        id,
        name: dir_name.to_string(),
        sort_name: super::item::make_sort_name(dir_name),
        path: relative_path,
        base_url: String::new(),
        created: chrono::Utc::now(),
        banner: find_image(path, "banner"),
        fanart: find_image(path, "fanart"),
        folder: find_image(path, "folder"),
        poster: find_image(path, "poster"),
        file_name: video_path.file_name()?.to_str()?.to_string(),
        file_size: std::fs::metadata(&video_file).ok()?.len() as i64,
        metadata: Metadata::default(), // TODO: Parse NFO
        srt_subs: Vec::new(),          // TODO: Find subtitles
        vtt_subs: Vec::new(),
    };

    Some(movie)
}

/// Scan a show directory for seasons and episodes
fn scan_show_directory(path: &Path, collection_root: &str) -> Option<Show> {
    let dir_name = path.file_name()?.to_str()?;

    // Generate ID from directory name
    let id = id_hash(dir_name);

    // Get relative path
    let relative_path = path.strip_prefix(collection_root).ok()?.to_str()?.to_string();

    // Scan for seasons
    let mut seasons = Vec::new();

    for entry in WalkDir::new(path).max_depth(1).into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_dir() {
            continue;
        }

        let season_path = entry.path();
        if season_path == path {
            continue; // Skip show root
        }

        let season_name = season_path.file_name()?.to_str()?;

        // Try to parse season number from directory name
        if let Some(season_no) = parse_season_number(season_name) {
            if let Some(season) = scan_season_directory(season_path, &relative_path, season_no) {
                seasons.push(season);
            }
        }
    }

    // Sort seasons by number
    seasons.sort_by_key(|s| s.season_no);

    let first_video = chrono::Utc::now();
    let last_video = chrono::Utc::now();

    let show = Show {
        id,
        name: dir_name.to_string(),
        sort_name: super::item::make_sort_name(dir_name),
        path: relative_path,
        base_url: String::new(),
        first_video,
        last_video,
        banner: find_image(path, "banner"),
        fanart: find_image(path, "fanart"),
        folder: find_image(path, "folder"),
        poster: find_image(path, "poster"),
        logo: find_image(path, "logo"),
        season_all_banner: find_image(path, "season-all-banner"),
        season_all_poster: find_image(path, "season-all-poster"),
        file_name: String::new(),
        file_size: 0,
        metadata: Metadata::default(), // TODO: Parse tvshow.nfo
        srt_subs: Vec::new(),
        vtt_subs: Vec::new(),
        seasons,
    };

    Some(show)
}

/// Scan a season directory for episodes
fn scan_season_directory(path: &Path, show_path: &str, season_no: i32) -> Option<Season> {
    let season_name = format!("Season {}", season_no);
    let season_id = id_hash(&format!("{}-{}", show_path, season_no));

    let mut episodes = Vec::new();

    // Find all video files in season directory
    for entry in WalkDir::new(path).max_depth(1).into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() {
            continue;
        }

        let file_path = entry.path();
        if !is_video_file(file_path) {
            continue;
        }

        let file_name = file_path.file_name()?.to_str()?;

        // Try to parse episode info from filename
        if let Some((parsed_season, episode_no, is_double, ep_name)) =
            super::parsefilename::parse_episode_name(file_name, season_no)
        {
            if parsed_season == season_no {
                let episode_id = id_hash(&format!("{}-s{}e{}", show_path, parsed_season, episode_no));

                let episode = Episode {
                    id: episode_id,
                    name: ep_name.clone(),
                    path: show_path.to_string(),
                    sort_name: ep_name,
                    season_no: parsed_season,
                    episode_no,
                    double: is_double,
                    base_name: file_name.to_string(),
                    created: chrono::Utc::now(),
                    file_name: format!("Season {:02}/{}", season_no, file_name),
                    file_size: std::fs::metadata(file_path).ok()?.len() as i64,
                    thumb: String::new(),          // TODO: Find thumbnail
                    metadata: Metadata::default(), // TODO: Parse episode NFO
                    srt_subs: Vec::new(),
                    vtt_subs: Vec::new(),
                };

                episodes.push(episode);
            }
        }
    }

    // Sort episodes by number
    episodes.sort_by_key(|e| e.episode_no);

    let season = Season {
        id: season_id,
        name: season_name,
        path: show_path.to_string(),
        season_no,
        banner: String::new(),
        fanart: String::new(),
        poster: find_image(path, &format!("season{:02}-poster", season_no)),
        season_all_banner: String::new(),
        season_all_poster: String::new(),
        episodes,
    };

    Some(season)
}

/// Find a video file in a directory
fn find_video_file(path: &Path) -> Option<PathBuf> {
    for entry in std::fs::read_dir(path).ok()? {
        let entry = entry.ok()?;
        let path = entry.path();

        if path.is_file() && is_video_file(&path) {
            return Some(path);
        }
    }
    None
}

/// Check if a file is a video file based on extension
fn is_video_file(path: &Path) -> bool {
    if let Some(ext) = path.extension() {
        let ext = ext.to_str().unwrap_or("").to_lowercase();
        matches!(
            ext.as_str(),
            "mkv" | "mp4" | "avi" | "m4v" | "mov" | "wmv" | "flv" | "webm"
        )
    } else {
        false
    }
}

/// Find an image file with a specific name pattern
fn find_image(path: &Path, name: &str) -> String {
    let extensions = ["jpg", "jpeg", "png", "webp"];

    for ext in &extensions {
        let image_path = path.join(format!("{}.{}", name, ext));
        if image_path.exists() {
            if let Some(file_name) = image_path.file_name() {
                return file_name.to_str().unwrap_or("").to_string();
            }
        }
    }

    String::new()
}

/// Parse season number from directory name (e.g., "Season 01" -> 1)
fn parse_season_number(name: &str) -> Option<i32> {
    let name_lower = name.to_lowercase();

    // Try "Season 01" format
    if name_lower.starts_with("season") {
        let num_str = name_lower.trim_start_matches("season").trim();
        return num_str.parse().ok();
    }

    // Try "S01" format
    if name_lower.starts_with('s') {
        let num_str = name_lower.trim_start_matches('s').trim();
        return num_str.parse().ok();
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_video_file() {
        assert!(is_video_file(Path::new("movie.mkv")));
        assert!(is_video_file(Path::new("movie.mp4")));
        assert!(!is_video_file(Path::new("image.jpg")));
        assert!(!is_video_file(Path::new("subtitle.srt")));
    }

    #[test]
    fn test_parse_season_number() {
        assert_eq!(parse_season_number("Season 01"), Some(1));
        assert_eq!(parse_season_number("Season 10"), Some(10));
        assert_eq!(parse_season_number("S01"), Some(1));
        assert_eq!(parse_season_number("S10"), Some(10));
        assert_eq!(parse_season_number("Invalid"), None);
    }
}
