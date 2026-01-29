use std::collections::{HashMap, HashSet};

use super::item::Item;

/// Collection type enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollectionType {
    Movies,
    Shows,
}

impl CollectionType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "movies" => Some(CollectionType::Movies),
            "shows" => Some(CollectionType::Shows),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            CollectionType::Movies => "movies",
            CollectionType::Shows => "shows",
        }
    }
}

/// Collection represents a media collection (movies or TV shows)
#[derive(Debug, Clone)]
pub struct Collection {
    /// Unique identifier for the collection. Hash of the collection name, or taken from configfile.
    pub id: String,
    /// Name of the collection, e.g., "My Favorite Movies"
    pub name: String,
    /// Type of the collection, e.g., "movies", "shows"
    pub collection_type: CollectionType,
    /// Items in the collection, could be type movies or shows
    pub items: Vec<Item>,
    /// Directory where the collection is stored
    pub directory: String,
    /// HLS server URL for streaming content
    pub hls_server: String,
}

impl Collection {
    pub fn new(
        id: String,
        name: String,
        collection_type: CollectionType,
        directory: String,
        hls_server: String,
    ) -> Self {
        Self {
            id,
            name,
            collection_type,
            items: Vec::new(),
            directory,
            hls_server,
        }
    }

    /// Details returns collection details such as genres, tags, ratings, etc.
    pub fn details(&self) -> CollectionDetails {
        let mut movie_count = 0;
        let mut show_count = 0;
        let mut episode_count = 0;
        let mut genres = HashSet::new();
        let mut studios = HashSet::new();
        let mut official_ratings = HashSet::new();
        let mut years = HashSet::new();

        for item in &self.items {
            match item {
                Item::Movie(movie) => {
                    movie_count += 1;
                    for genre in movie.metadata.genres() {
                        if !genre.is_empty() {
                            genres.insert(genre.clone());
                        }
                    }
                    for studio in movie.metadata.studios() {
                        if !studio.is_empty() {
                            studios.insert(studio.clone());
                        }
                    }
                    let rating = movie.metadata.official_rating();
                    if !rating.is_empty() {
                        official_ratings.insert(rating.to_string());
                    }
                    let year = movie.metadata.year();
                    if year != 0 {
                        years.insert(year);
                    }
                }
                Item::Show(show) => {
                    show_count += 1;
                    for season in &show.seasons {
                        episode_count += season.episodes.len();
                    }
                    for genre in show.metadata.genres() {
                        if !genre.is_empty() {
                            genres.insert(genre.clone());
                        }
                    }
                    for studio in show.metadata.studios() {
                        if !studio.is_empty() {
                            studios.insert(studio.clone());
                        }
                    }
                    let rating = show.metadata.official_rating();
                    if !rating.is_empty() {
                        official_ratings.insert(rating.to_string());
                    }
                    let year = show.metadata.year();
                    if year != 0 {
                        years.insert(year);
                    }
                }
                _ => {}
            }
        }

        CollectionDetails {
            movie_count,
            show_count,
            episode_count,
            genres: genres.into_iter().collect(),
            studios: studios.into_iter().collect(),
            tags: Vec::new(),
            official_ratings: official_ratings.into_iter().collect(),
            years: years.into_iter().collect(),
        }
    }

    /// GenreCount returns number of items per genre/// Count genres across all items
    pub fn genre_count(&self) -> HashMap<String, usize> {
        let mut genre_count = HashMap::new();

        for item in &self.items {
            let genres = match item {
                Item::Movie(movie) => movie.metadata.genres(),
                Item::Show(show) => show.metadata.genres(),
                _ => &[],
            };

            for genre in genres {
                if !genre.is_empty() {
                    *genre_count.entry(genre.clone()).or_insert(0) += 1;
                }
            }
        }

        genre_count
    }
}

/// CollectionDetails contains aggregate details about a collection.
#[derive(Debug, Clone)]
pub struct CollectionDetails {
    /// Number of movies.
    pub movie_count: usize,
    /// Number of shows.
    pub show_count: usize,
    /// Number of episodes.
    pub episode_count: usize,
    /// List of genres.
    pub genres: Vec<String>,
    /// List of studios.
    pub studios: Vec<String>,
    /// List of tags.
    pub tags: Vec<String>,
    /// List of official ratings.
    pub official_ratings: Vec<String>,
    /// List of years.
    pub years: Vec<i32>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collection::item::Movie;
    use crate::collection::metadata::Metadata;
    use chrono::Utc;

    #[test]
    fn test_collection_type_from_str() {
        assert_eq!(CollectionType::from_str("movies"), Some(CollectionType::Movies));
        assert_eq!(CollectionType::from_str("shows"), Some(CollectionType::Shows));
        assert_eq!(CollectionType::from_str("invalid"), None);
    }

    #[test]
    fn test_collection_type_as_str() {
        assert_eq!(CollectionType::Movies.as_str(), "movies");
        assert_eq!(CollectionType::Shows.as_str(), "shows");
    }

    #[test]
    fn test_collection_new() {
        let collection = Collection::new(
            "test-id".to_string(),
            "Test Collection".to_string(),
            CollectionType::Movies,
            "/media/movies".to_string(),
            "".to_string(),
        );

        assert_eq!(collection.id, "test-id");
        assert_eq!(collection.name, "Test Collection");
        assert_eq!(collection.collection_type, CollectionType::Movies);
        assert_eq!(collection.items.len(), 0);
    }

    #[test]
    fn test_genre_count() {
        let mut collection = Collection::new(
            "test".to_string(),
            "Test".to_string(),
            CollectionType::Movies,
            "/test".to_string(),
            "".to_string(),
        );

        let movie = Movie {
            id: "m1".to_string(),
            name: "Movie 1".to_string(),
            sort_name: "movie 1".to_string(),
            path: "movie1".to_string(),
            base_url: "".to_string(),
            created: Utc::now(),
            banner: "".to_string(),
            fanart: "".to_string(),
            folder: "".to_string(),
            poster: "".to_string(),
            file_name: "movie1.mkv".to_string(),
            file_size: 0,
            metadata: Metadata::default(),
            srt_subs: Vec::new(),
            vtt_subs: Vec::new(),
        };

        collection.items.push(Item::Movie(movie));

        let genre_count = collection.genre_count();
        assert!(genre_count.is_empty()); // Default metadata has no genres
    }
}
