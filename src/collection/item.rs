use chrono::{DateTime, Utc};
use regex::Regex;
use std::sync::OnceLock;
use std::time::Duration;

use super::metadata::Metadata;

/// Subtitle file with language and path
#[derive(Debug, Clone)]
pub struct Subs {
    pub lang: String,
    pub path: String,
}

pub type Subtitles = Vec<Subs>;

/// Movie represents a movie in a collection.
#[derive(Debug, Clone)]
pub struct Movie {
    /// id is the unique identifier for the movie. Typically Idhash() of name.
    pub id: String,
    /// name is the name of the movie, e.g. "Casablanca (1949)"
    pub name: String,
    /// sort_name is used to sort on.
    pub sort_name: String,
    /// path is the directory to the movie, relative to collection root.
    pub path: String,
    /// base_url is the base URL for accessing the movie.
    pub base_url: String,
    /// created is the create timestamp of the movie.
    pub created: DateTime<Utc>,
    /// banner is the movie's banner image, often "banner.jpg", TV shows only.
    pub banner: String,
    /// fanart is this movie's fanart image, often "fanart.jpg"
    pub fanart: String,
    /// folder is this movie's folder image, often "folder.jpg"
    pub folder: String,
    /// poster is this movie's poster image, often "poster.jpg"
    pub poster: String,
    /// file_name, e.g. "casablanca.mp4"
    pub file_name: String,
    /// file_size is the size of the video file in bytes.
    pub file_size: i64,
    /// Metadata holds the metadata for the movie, e.g. from NFO file.
    pub metadata: Metadata,
    pub srt_subs: Subtitles,
    pub vtt_subs: Subtitles,
}

impl Movie {
    pub fn file_path(&self) -> String {
        format!("{}/{}", self.path, self.file_name)
    }

    pub fn duration(&self) -> Duration {
        self.metadata.duration()
    }
}

/// Show represents a TV show with multiple seasons and episodes.
#[derive(Debug, Clone)]
pub struct Show {
    /// id is the unique identifier of the show. Typically Idhash() of name.
    pub id: String,
    /// name is the display name of the show, e.g. "Casablanca"
    pub name: String,
    /// sort_name is used to sort on.
    pub sort_name: String,
    /// path is the directory to the show, relative to collection root. E.g. "Casablanca (1949)"
    pub path: String,
    /// base_url is the base URL for accessing the show.
    pub base_url: String,
    /// first_video is the timestamp of the first video in the show.
    pub first_video: DateTime<Utc>,
    /// last_video is the timestamp of the last video in the show.
    pub last_video: DateTime<Utc>,
    /// banner is the show's banner image, often "banner.jpg".
    pub banner: String,
    /// fanart is this show's fanart image, often "fanart.jpg"
    pub fanart: String,
    /// folder is this show's folder image, often "folder.jpg"
    pub folder: String,
    /// poster is this show's poster image, often "poster.jpg"
    pub poster: String,
    /// logo is this show's transparent logo, often "clearlogo.png", TV shows only.
    pub logo: String,
    /// season_all_banner is the banner to be used in case we do not have a season-specific banner.
    pub season_all_banner: String,
    /// season_all_poster to be used in case we do not have a season-specific poster.
    pub season_all_poster: String,
    /// file_name of the video file, e.g. "casablanca.mp4"
    pub file_name: String,
    /// file_size is the size of the video file in bytes.
    pub file_size: i64,
    /// Metadata holds the metadata for the show, e.g. from NFO file.
    pub metadata: Metadata,
    pub srt_subs: Subtitles,
    pub vtt_subs: Subtitles,
    /// Seasons contains the seasons in this TV show.
    pub seasons: Vec<Season>,
}

impl Show {
    pub fn duration(&self) -> Duration {
        self.seasons.iter().map(|s| s.duration()).sum()
    }
}

/// Season represents a season of a TV show, containing multiple episodes.
#[derive(Debug, Clone)]
pub struct Season {
    /// id is the unique identifier of the season.
    pub id: String,
    /// name is the human-readable name of the season.
    pub name: String,
    /// path is the directory to the show(!), relative to collection root. (e.g. Casablanca)
    pub path: String,
    /// season_no is the season number, e.g., 1, 2, etc. 0 is used for specials.
    pub season_no: i32,
    /// banner is the path to the season banner image.
    pub banner: String,
    /// fanart is the path to the season fanart image.
    pub fanart: String,
    /// poster is the path to the season poster image, e.g. "season01-poster.jpg"
    pub poster: String,
    /// season_all_banner is the banner to be used in case we do not have a season-specific banner.
    pub season_all_banner: String,
    /// season_all_poster to be used in case we do not have a season-specific poster.
    pub season_all_poster: String,
    /// Episodes contains the episodes in this season.
    pub episodes: Vec<Episode>,
}

impl Season {
    pub fn poster(&self) -> &str {
        if !self.poster.is_empty() {
            return &self.poster;
        }
        if !self.season_all_poster.is_empty() {
            return &self.season_all_poster;
        }
        ""
    }

    pub fn duration(&self) -> Duration {
        self.episodes.iter().map(|e| e.duration()).sum()
    }
}

/// Episode represents a single episode of a TV show.
#[derive(Debug, Clone)]
pub struct Episode {
    /// id is the unique identifier of the episode. Typically Idhash() of name.
    pub id: String,
    /// name is the human-readable name of the episode.
    pub name: String,
    /// path is the directory of the show, relative to collection root. (e.g. Casablanca)
    pub path: String,
    /// sort_name is the name of the episode when sorting is applied.
    pub sort_name: String,
    /// season_no is the season number, e.g., 1, 2, etc. 0 is used for specials.
    pub season_no: i32,
    /// episode_no is the episode number within the season, e.g., 1, 2, etc.
    pub episode_no: i32,
    /// double indicates if this is a double episode, e.g., 1-2.
    pub double: bool,
    /// base_name is the base name of the episode, e.g., "casablanca.s01e01"
    pub base_name: String,
    /// created is the timestamp of the episode.
    pub created: DateTime<Utc>,
    /// file_name is the filename relative to show directory, e.g. "S01/casablanca.s01e01.mp4"
    pub file_name: String,
    /// file_size is the size of the video file in bytes.
    pub file_size: i64,
    /// thumb is the thumbnail image relative to show directory, e.g. "S01/casablanca.s01e01-thumb.jpg"
    pub thumb: String,
    /// Metadata holds the metadata for the episode, e.g. from NFO file.
    pub metadata: Metadata,
    pub srt_subs: Subtitles,
    pub vtt_subs: Subtitles,
}

impl Episode {
    pub fn duration(&self) -> Duration {
        self.metadata.duration()
    }
}

/// Item enum - replaces Go interface
/// In Go, Movie, Show, Season, and Episode all implement the Item interface
#[derive(Debug, Clone)]
pub enum Item {
    Movie(Movie),
    Show(Show),
    Season(Season),
    Episode(Episode),
}

impl Item {
    pub fn id(&self) -> String {
        match self {
            Item::Movie(m) => m.id.clone(),
            Item::Show(s) => s.id.clone(),
            Item::Season(s) => s.id.clone(),
            Item::Episode(e) => e.id.clone(),
        }
    }

    pub fn name(&self) -> String {
        match self {
            Item::Movie(m) => m.name.clone(),
            Item::Show(s) => s.name.clone(),
            Item::Season(s) => s.name.clone(),
            Item::Episode(e) => e.name.clone(),
        }
    }

    pub fn duration(&self) -> Duration {
        match self {
            Item::Movie(m) => m.duration(),
            Item::Show(s) => s.duration(),
            Item::Season(s) => s.duration(),
            Item::Episode(e) => e.duration(),
        }
    }
}

/// ItemRef enum - for borrowing without ownership
#[derive(Debug, Clone, Copy)]
pub enum ItemRef<'a> {
    Movie(&'a Movie),
    Show(&'a Show),
    Season(&'a Season),
    Episode(&'a Episode),
}

/// makeSortName returns a name suitable for sorting.
pub fn make_sort_name(name: &str) -> String {
    // Start with lowercasing and trimming whitespace.
    let mut title = name.to_lowercase().trim().to_string();

    // Remove leading articles.
    for prefix in &["the ", "a ", "an "] {
        if title.starts_with(prefix) {
            title = title[prefix.len()..].trim().to_string();
            break;
        }
    }

    // Remove whitespace and punctuation.
    title = title
        .trim_start_matches(|c: char| c.is_whitespace() || c.is_ascii_punctuation())
        .to_string();

    // Remove year suffix if present.
    remove_year_suffix(&title)
}

/// removeYearSuffix removes year suffix from item name.
fn remove_year_suffix(name: &str) -> String {
    static IS_YEAR: OnceLock<Regex> = OnceLock::new();
    let regex = IS_YEAR.get_or_init(|| Regex::new(r"\s*\(\d{4}\)\s*$").unwrap());

    if let Some(mat) = regex.find(name) {
        name[..mat.start()].trim().to_string()
    } else {
        name.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_make_sort_name() {
        assert_eq!(make_sort_name("The Matrix (1999)"), "matrix");
        assert_eq!(make_sort_name("A Beautiful Mind"), "beautiful mind");
        assert_eq!(make_sort_name("An American Tail"), "american tail");
        assert_eq!(make_sort_name("Casablanca (1942)"), "casablanca");
    }

    #[test]
    fn test_remove_year_suffix() {
        assert_eq!(remove_year_suffix("Movie (2020)"), "Movie");
        assert_eq!(remove_year_suffix("Movie"), "Movie");
        assert_eq!(remove_year_suffix("Movie (2020) Extra"), "Movie (2020) Extra");
    }
}
