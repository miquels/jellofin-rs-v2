use chrono::{DateTime, Utc};
use regex::Regex;
use std::sync::OnceLock;
use std::time::Duration;

use super::collection::CollectionType;
use super::metadata::Metadata;
use crate::database::UserData as DbUserData;

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
    /// collection_id is the ID of the collection this movie belongs to.
    pub collection_id: String,
    /// user_data is per-request user play state (populated on cloned copies, not stored).
    pub user_data: Option<Box<DbUserData>>,
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

    pub fn duration(&self) -> Option<Duration> {
        self.metadata.duration()
    }
}

/// Show represents a TV show with multiple seasons and episodes.
#[derive(Debug, Clone)]
pub struct Show {
    /// id is the unique identifier of the show. Typically Idhash() of name.
    pub id: String,
    pub collection_id: String,
    pub user_data: Option<Box<DbUserData>>,
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
    pub collection_id: String,
    pub user_data: Option<Box<DbUserData>>,
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
    pub collection_id: String,
    pub user_data: Option<Box<DbUserData>>,
    /// show_id is the ID of the parent show.
    pub show_id: String,
    /// season_id is the ID of the parent season.
    pub season_id: String,
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
        self.metadata.duration().unwrap_or_default()
    }
}

/// CollectionFolder represents a media library collection at the root overview level.
#[derive(Debug, Clone)]
pub struct CollectionFolder {
    pub id: String,
    pub name: String,
    pub collection_type: CollectionType,
    pub child_count: i32,
    pub genres: Vec<String>,
}

/// UserView represents a virtual collection view (e.g. Favorites, Playlists).
#[derive(Debug, Clone)]
pub struct UserView {
    pub id: String,
    pub name: String,
    pub collection_type: String,
    pub child_count: Option<i32>,
}

/// PlaylistItem represents a user playlist as a native item.
#[derive(Debug, Clone)]
pub struct PlaylistItem {
    pub id: String,
    pub name: String,
    pub child_count: i32,
}

/// Item enum - replaces Go interface
/// In Go, Movie, Show, Season, and Episode all implement the Item interface
#[derive(Debug, Clone)]
pub enum Item {
    Movie(Movie),
    Show(Show),
    Season(Season),
    Episode(Episode),
    CollectionFolder(CollectionFolder),
    UserView(UserView),
    Playlist(PlaylistItem),
}

impl Item {
    pub fn id(&self) -> String {
        match self {
            Item::Movie(m) => m.id.clone(),
            Item::Show(s) => s.id.clone(),
            Item::Season(s) => s.id.clone(),
            Item::Episode(e) => e.id.clone(),
            Item::CollectionFolder(c) => c.id.clone(),
            Item::UserView(u) => u.id.clone(),
            Item::Playlist(p) => p.id.clone(),
        }
    }

    pub fn collection_id(&self) -> &str {
        match self {
            Item::Movie(m) => &m.collection_id,
            Item::Show(s) => &s.collection_id,
            Item::Season(s) => &s.collection_id,
            Item::Episode(e) => &e.collection_id,
            _ => "",
        }
    }

    pub fn set_collection_id(&mut self, id: String) {
        match self {
            Item::Movie(m) => m.collection_id = id,
            Item::Show(s) => {
                s.collection_id = id.clone();
                for season in &mut s.seasons {
                    season.collection_id = id.clone();
                    for episode in &mut season.episodes {
                        episode.collection_id = id.clone();
                    }
                }
            }
            Item::Season(s) => s.collection_id = id,
            Item::Episode(e) => e.collection_id = id,
            _ => {}
        }
    }

    /// Populate show_id and season_id on all nested episodes within a Show.
    pub fn populate_hierarchy_ids(&mut self) {
        if let Item::Show(s) = self {
            let show_id = s.id.clone();
            for season in &mut s.seasons {
                let season_id = season.id.clone();
                for episode in &mut season.episodes {
                    episode.show_id = show_id.clone();
                    episode.season_id = season_id.clone();
                }
            }
        }
    }

    pub fn get_user_data(&self) -> Option<&DbUserData> {
        match self {
            Item::Movie(m) => m.user_data.as_deref(),
            Item::Show(s) => s.user_data.as_deref(),
            Item::Season(s) => s.user_data.as_deref(),
            Item::Episode(e) => e.user_data.as_deref(),
            _ => None,
        }
    }

    pub fn set_user_data(&mut self, ud: DbUserData) {
        match self {
            Item::Movie(m) => m.user_data = Some(Box::new(ud)),
            Item::Show(s) => s.user_data = Some(Box::new(ud)),
            Item::Season(s) => s.user_data = Some(Box::new(ud)),
            Item::Episode(e) => e.user_data = Some(Box::new(ud)),
            _ => {}
        }
    }

    pub fn name(&self) -> String {
        match self {
            Item::Movie(m) => m.name.clone(),
            Item::Show(s) => s.name.clone(),
            Item::Season(s) => s.name.clone(),
            Item::Episode(e) => e.name.clone(),
            Item::CollectionFolder(c) => c.name.clone(),
            Item::UserView(u) => u.name.clone(),
            Item::Playlist(p) => p.name.clone(),
        }
    }

    pub fn duration(&self) -> Option<Duration> {
        match self {
            Item::Movie(m) => m.duration(),
            Item::Show(s) => Some(s.duration()),
            Item::Season(s) => Some(s.duration()),
            Item::Episode(e) => Some(e.duration()),
            _ => None,
        }
    }

    /// Returns the Jellyfin item type string for this item.
    pub fn jf_type(&self) -> &'static str {
        match self {
            Item::Movie(_) => "Movie",
            Item::Show(_) => "Series",
            Item::Season(_) => "Season",
            Item::Episode(_) => "Episode",
            Item::CollectionFolder(_) => "CollectionFolder",
            Item::UserView(_) => "UserView",
            Item::Playlist(_) => "Playlist",
        }
    }

    /// Returns the sort name for this item.
    pub fn sort_name(&self) -> &str {
        match self {
            Item::Movie(m) => &m.sort_name,
            Item::Show(s) => &s.sort_name,
            Item::Season(s) => &s.name,
            Item::Episode(e) => &e.sort_name,
            Item::CollectionFolder(c) => c.collection_type.as_str(),
            Item::UserView(u) => &u.collection_type,
            Item::Playlist(p) => &p.name,
        }
    }

    /// Returns the created/date for this item (used for DateCreated / DateLastContentAdded sorting).
    pub fn created(&self) -> DateTime<Utc> {
        match self {
            Item::Movie(m) => m.created,
            Item::Show(s) => s.first_video,
            Item::Episode(e) => e.created,
            _ => DateTime::<Utc>::default(),
        }
    }

    /// Returns the premiere date if available.
    pub fn premiere_date(&self) -> Option<DateTime<Utc>> {
        match self {
            Item::Movie(m) => m.metadata.premiered.or(Some(m.created)),
            Item::Show(s) => s.metadata.premiered.or(Some(s.first_video)),
            _ => None,
        }
    }

    /// Returns community rating if available.
    pub fn community_rating(&self) -> Option<f32> {
        match self {
            Item::Movie(m) => m.metadata.rating,
            Item::Show(s) => s.metadata.rating,
            _ => None,
        }
    }

    /// Returns production year if available.
    pub fn production_year(&self) -> Option<i32> {
        match self {
            Item::Movie(m) => m.metadata.year,
            Item::Show(s) => s.metadata.year,
            _ => None,
        }
    }

    /// Returns genre names for this item.
    pub fn genres(&self) -> &[String] {
        match self {
            Item::Movie(m) => &m.metadata.genres,
            Item::Show(s) => &s.metadata.genres,
            Item::CollectionFolder(c) => &c.genres,
            _ => &[],
        }
    }

    /// Returns a reference to the item's metadata.
    pub fn metadata(&self) -> &Metadata {
        match self {
            Item::Movie(m) => &m.metadata,
            Item::Show(s) => &s.metadata,
            Item::Episode(e) => &e.metadata,
            _ => {
                static EMPTY: std::sync::OnceLock<Metadata> = std::sync::OnceLock::new();
                EMPTY.get_or_init(Metadata::default)
            }
        }
    }

    /// Returns studio names for this item.
    pub fn studios(&self) -> &[String] {
        match self {
            Item::Movie(m) => &m.metadata.studios,
            Item::Show(s) => &s.metadata.studios,
            Item::Episode(e) => &e.metadata.studios,
            _ => &[],
        }
    }

    /// Returns official rating (e.g. "PG-13") if available.
    pub fn official_rating(&self) -> Option<&str> {
        match self {
            Item::Movie(m) => m.metadata.official_rating.as_deref(),
            Item::Show(s) => s.metadata.official_rating.as_deref(),
            _ => None,
        }
    }

    /// Returns whether the item has subtitles.
    pub fn has_subtitles(&self) -> bool {
        match self {
            Item::Movie(m) => !m.srt_subs.is_empty() || !m.vtt_subs.is_empty(),
            Item::Episode(e) => !e.srt_subs.is_empty() || !e.vtt_subs.is_empty(),
            _ => false,
        }
    }

    /// Returns whether the item is HD (720p or higher).
    pub fn is_hd(&self) -> bool {
        self.metadata().video_height.map(|h| h >= 720).unwrap_or(false)
    }

    /// Returns whether the item is 4K (2160p or higher).
    pub fn is_4k(&self) -> bool {
        self.metadata().video_height.map(|h| h >= 1500).unwrap_or(false)
    }

    /// Returns runtime in Jellyfin ticks (100ns units), if available.
    pub fn run_time_ticks(&self) -> Option<i64> {
        self.metadata().runtime_ticks()
    }

    /// Returns the index number (episode number, or season number for seasons).
    pub fn index_number(&self) -> Option<i32> {
        match self {
            Item::Episode(e) => Some(e.episode_no),
            Item::Season(s) => Some(if s.season_no != 0 { s.season_no } else { 99 }),
            _ => None,
        }
    }

    /// Returns the parent index number (season number for episodes).
    pub fn parent_index_number(&self) -> Option<i32> {
        match self {
            Item::Episode(e) => Some(e.season_no),
            _ => None,
        }
    }

    /// Returns whether this item is a folder/container.
    pub fn is_folder(&self) -> bool {
        matches!(
            self,
            Item::Show(_) | Item::Season(_) | Item::CollectionFolder(_) | Item::UserView(_) | Item::Playlist(_)
        )
    }

    /// Returns the series (show) ID for this item, if applicable.
    pub fn series_id(&self) -> Option<&str> {
        match self {
            Item::Episode(e) => Some(&e.show_id),
            Item::Season(s) => {
                // Seasons don't store show_id directly; return None
                // (seasons are typically filtered by parent_id, not series_id)
                let _ = s;
                None
            }
            _ => None,
        }
    }

    /// Returns the season ID for this item, if applicable.
    pub fn season_id(&self) -> Option<&str> {
        match self {
            Item::Episode(e) => Some(&e.season_id),
            _ => None,
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
    CollectionFolder(&'a CollectionFolder),
    UserView(&'a UserView),
    Playlist(&'a PlaylistItem),
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
