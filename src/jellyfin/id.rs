use std::fmt::Display;

use crate::idhash::id_hash;

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
pub const ITEM_PREFIX_DISPLAY_PREFERENCES: &str = "displaypreferences_";

pub struct Id {
    pub value: String,
}

impl Display for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

impl Id {
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
        }
    }

    pub fn as_str(&self) -> &str {
        &self.value
    }

    pub fn into_string(self) -> String {
        self.value
    }

    pub fn hash(value: impl AsRef<str>) -> Self {
        Self::new(id_hash(value.as_ref()))
    }

    pub fn as_jf_root(&self) -> String {
        format!("{}{}", ITEM_PREFIX_ROOT, self.value)
    }

    pub fn as_jf_collection(&self) -> String {
        format!("{}{}", ITEM_PREFIX_COLLECTION, self.value)
    }

    pub fn as_jf_collection_favorites(&self) -> String {
        format!("{}{}", ITEM_PREFIX_COLLECTION_FAVORITES, self.value)
    }

    pub fn as_jf_collection_playlist(&self) -> String {
        format!("{}{}", ITEM_PREFIX_COLLECTION_PLAYLIST, self.value)
    }

    pub fn as_jf_show(&self) -> String {
        format!("{}{}", ITEM_PREFIX_SHOW, self.value)
    }

    pub fn as_jf_season(&self) -> String {
        format!("{}{}", ITEM_PREFIX_SEASON, self.value)
    }

    pub fn as_jf_episode(&self) -> String {
        format!("{}{}", ITEM_PREFIX_EPISODE, self.value)
    }

    pub fn as_jf_playlist(&self) -> String {
        format!("{}{}", ITEM_PREFIX_PLAYLIST, self.value)
    }

    pub fn as_jf_genre(&self) -> String {
        format!("{}{}", ITEM_PREFIX_GENRE, self.value)
    }

    pub fn as_jf_studio(&self) -> String {
        format!("{}{}", ITEM_PREFIX_STUDIO, self.value)
    }

    pub fn as_jf_person(&self) -> String {
        format!("{}{}", ITEM_PREFIX_PERSON, self.value)
    }

    pub fn as_jf_display_preferences(&self) -> String {
        format!("{}{}", ITEM_PREFIX_DISPLAY_PREFERENCES, self.value)
    }

    pub fn trim_prefix(&self) -> &'_ str {
        self.value
            .splitn(2, ITEM_PREFIX_SEPARATOR)
            .nth(1)
            .unwrap_or(&self.value)
    }

    pub fn prefix(&self) -> &'_ str {
        self.value
            .splitn(2, ITEM_PREFIX_SEPARATOR)
            .nth(0)
            .unwrap_or("")
    }

    pub fn is_jf_root(&self) -> bool {
        self.prefix() == ITEM_PREFIX_ROOT
    }

    pub fn is_jf_collection(&self) -> bool {
        self.prefix() == ITEM_PREFIX_COLLECTION
    }

    pub fn is_jf_collection_favorites(&self) -> bool {
        self.prefix() == ITEM_PREFIX_COLLECTION_FAVORITES
    }

    pub fn is_jf_collection_playlist(&self) -> bool {
        self.prefix() == ITEM_PREFIX_COLLECTION_PLAYLIST
    }

    pub fn is_jf_show(&self) -> bool {
        self.prefix() == ITEM_PREFIX_SHOW
    }

    pub fn is_jf_season(&self) -> bool {
        self.prefix() == ITEM_PREFIX_SEASON
    }

    pub fn is_jf_episode(&self) -> bool {
        self.prefix() == ITEM_PREFIX_EPISODE
    }

    pub fn is_jf_playlist(&self) -> bool {
        self.prefix() == ITEM_PREFIX_PLAYLIST
    }

    pub fn is_jf_genre(&self) -> bool {
        self.prefix() == ITEM_PREFIX_GENRE
    }

    pub fn is_jf_studio(&self) -> bool {
        self.prefix() == ITEM_PREFIX_STUDIO
    }

    pub fn is_jf_person(&self) -> bool {
        self.prefix() == ITEM_PREFIX_PERSON
    }
}
