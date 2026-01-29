use arc_swap::ArcSwap;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;
use tracing::info;

use super::collection::{Collection, CollectionType};
use super::item::Item;
use crate::idhash::id_hash;

/// CollectionRepo is a repository holding content collections.
pub struct CollectionRepo {
    collections: Arc<ArcSwap<Vec<Collection>>>,
}

impl CollectionRepo {
    /// Create a new CollectionRepo
    pub fn new() -> Self {
        Self {
            collections: Arc::new(ArcSwap::from_pointee(Vec::new())),
        }
    }

    /// Add a new content collection to the repository
    pub fn add_collection(
        &self,
        name: String,
        id: Option<String>,
        collection_type: &str,
        directory: String,
        hls_server: String,
    ) -> Result<(), String> {
        let ct = CollectionType::from_str(collection_type)
            .ok_or_else(|| format!("Unknown collection type: {}", collection_type))?;

        let collection_id = id.unwrap_or_else(|| id_hash(&name));

        info!(
            "Adding collection {}, id: {}, type: {}, directory: {}",
            name,
            collection_id,
            collection_type,
            directory
        );

        let collection = Collection::new(collection_id, name, ct, directory, hls_server);

        // Add to collections
        let mut collections = (**self.collections.load()).clone();
        collections.push(collection);
        self.collections.store(Arc::new(collections));

        Ok(())
    }

    /// Initialize collections by scanning directories
    pub fn init(&self) {
        info!("Initializing collections...");
        self.update_collections(Duration::from_millis(0));
    }

    /// Background task that continuously scans for content changes
    pub fn background(&self) {
        let _collections = Arc::clone(&self.collections);
        
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(300)).await; // 5 minutes
                info!("Background scan starting...");
                
                // TODO: Implement update_collections
                // For now, just log
                info!("Background scan complete");
            }
        });
    }

    /// Update collections with latest content from filesystem
    fn update_collections(&self, scan_interval: Duration) {
        let mut updated_collections = (**self.collections.load()).clone();
        
        for collection in &mut updated_collections {
            match collection.collection_type {
                CollectionType::Movies => {
                    super::kodifs::build_movies(collection, scan_interval);
                }
                CollectionType::Shows => {
                    super::kodifs::build_shows(collection, scan_interval);
                }
            }
        }
        
        self.collections.store(Arc::new(updated_collections));
    }

    /// Get all collections
    pub fn get_collections(&self) -> Vec<Collection> {
        (**self.collections.load()).clone()
    }

    /// Get a collection by ID
    pub fn get_collection(&self, collection_id: &str) -> Option<Collection> {
        self.collections
            .load()
            .iter()
            .find(|c| c.id == collection_id)
            .cloned()
    }

    /// Get an item by collection ID and item ID
    pub fn get_item(&self, collection_id: &str, item_id: &str) -> Option<Item> {
        let collection = self.get_collection(collection_id)?;
        
        for item in &collection.items {
            match item {
                Item::Movie(movie) if movie.id == item_id => {
                    return Some(item.clone());
                }
                Item::Show(show) if show.id == item_id => {
                    return Some(item.clone());
                }
                Item::Show(show) => {
                    // Search in seasons
                    for season in &show.seasons {
                        if season.id == item_id {
                            return Some(Item::Season(season.clone()));
                        }
                        // Search in episodes
                        for episode in &season.episodes {
                            if episode.id == item_id {
                                return Some(Item::Episode(episode.clone()));
                            }
                        }
                    }
                }
                Item::Season(season) if season.id == item_id => {
                    return Some(item.clone());
                }
                Item::Episode(episode) if episode.id == item_id => {
                    return Some(item.clone());
                }
                _ => {}
            }
        }
        
        None
    }

    /// Get an item by ID across all collections
    pub fn get_item_by_id(&self, item_id: &str) -> Option<(Collection, Item)> {
        let collections = self.collections.load();
        for collection in collections.iter() {
            if let Some(item) = self.get_item(&collection.id, item_id) {
                return Some((collection.clone(), item));
            }
        }
        None
    }

    /// Get a season by ID across all collections
    pub fn get_season_by_id(&self, season_id: &str) -> Option<(Collection, crate::collection::item::Show, crate::collection::item::Season)> {
        let collections = self.collections.load();
        for collection in collections.iter() {
            for item in &collection.items {
                if let Item::Show(show) = item {
                    for season in &show.seasons {
                        if season.id == season_id {
                            return Some((collection.clone(), show.clone(), season.clone()));
                        }
                    }
                }
            }
        }
        None
    }

    /// Get an episode by ID across all collections
    pub fn get_episode_by_id(&self, episode_id: &str) -> Option<(Collection, crate::collection::item::Show, crate::collection::item::Season, crate::collection::item::Episode)> {
        let collections = self.collections.load();
        for collection in collections.iter() {
            for item in &collection.items {
                if let Item::Show(show) = item {
                    for season in &show.seasons {
                        for episode in &season.episodes {
                            if episode.id == episode_id {
                                return Some((collection.clone(), show.clone(), season.clone(), episode.clone()));
                            }
                        }
                    }
                }
            }
        }
        None
    }

    /// NextUp returns the nextup episodes in the collection based upon list of watched episodes
    pub fn next_up(&self, watched_episode_ids: &[String]) -> Vec<String> {
        struct ShowEntry {
            show_id: String,
            season_no: i32,
            episode_no: i32,
            season_idx: usize,
            ep_idx: usize,
        }
        let mut show_map: HashMap<String, ShowEntry> = HashMap::new();

        for episode_id in watched_episode_ids {
            if let Some((collection, show, season, episode)) = self.get_episode_by_id(episode_id) {
                if collection.collection_type != super::collection::CollectionType::Shows {
                    continue;
                }

                // Find indices
                let mut season_idx = None;
                let mut ep_idx = None;
                for (si, s) in show.seasons.iter().enumerate() {
                    if s.id == season.id {
                        season_idx = Some(si);
                        for (ei, e) in s.episodes.iter().enumerate() {
                            if e.id == episode.id {
                                ep_idx = Some(ei);
                                break;
                            }
                        }
                        break;
                    }
                }

                if let (Some(si), Some(ei)) = (season_idx, ep_idx) {
                    let entry = show_map.entry(show.id.clone()).or_insert(ShowEntry {
                        show_id: show.id.clone(),
                        season_no: season.season_no,
                        episode_no: episode.episode_no,
                        season_idx: si,
                        ep_idx: ei,
                    });

                    if season.season_no > entry.season_no || (season.season_no == entry.season_no && episode.episode_no > entry.episode_no) {
                        entry.season_no = season.season_no;
                        entry.episode_no = episode.episode_no;
                        entry.season_idx = si;
                        entry.ep_idx = ei;
                    }
                }
            }
        }

        let mut next_up_ids = Vec::new();
        let _collections = self.collections.load();
        
        for entry in show_map.values() {
            // Need to find the show again to get current data
            if let Some((_, Item::Show(show))) = self.get_item_by_id(&entry.show_id) {
                if entry.season_idx < show.seasons.len() {
                    let season = &show.seasons[entry.season_idx];
                    if entry.ep_idx + 1 < season.episodes.len() {
                        next_up_ids.push(season.episodes[entry.ep_idx + 1].id.clone());
                    } else if entry.season_idx + 1 < show.seasons.len() && !show.seasons[entry.season_idx + 1].episodes.is_empty() {
                        next_up_ids.push(show.seasons[entry.season_idx + 1].episodes[0].id.clone());
                    }
                }
            }
        }

        next_up_ids
    }

    pub async fn similar(&self, collection_id: &str, item_id: &str) -> Vec<String> {
        if let Some(_collection) = self.get_collection(collection_id) {
            if let Some(_item) = self.get_item(collection_id, item_id) {
                // TODO: Implement similar logic using search index
                // For now, return empty
                return Vec::new();
            }
        }
        Vec::new()
    }

    /// Search performs a item search in collection repository and returns matching items.
    pub fn search(&self, term: &str) -> Vec<String> {
        let mut results = Vec::new();
        let term_lower = term.to_lowercase();
        let collections = self.collections.load();
        for collection in collections.iter() {
            for item in &collection.items {
                match item {
                    Item::Movie(m) => {
                        if m.name.to_lowercase().contains(&term_lower) {
                            results.push(m.id.clone());
                        }
                    }
                    Item::Show(s) => {
                        if s.name.to_lowercase().contains(&term_lower) {
                            results.push(s.id.clone());
                        }
                        for season in &s.seasons {
                            for episode in &season.episodes {
                                if episode.name.to_lowercase().contains(&term_lower) {
                                    results.push(episode.id.clone());
                                }
                            }
                        }
                    }
                    Item::Season(s) => {
                        if s.name.to_lowercase().contains(&term_lower) {
                            results.push(s.id.clone());
                        }
                    }
                    Item::Episode(e) => {
                        if e.name.to_lowercase().contains(&term_lower) {
                            results.push(e.id.clone());
                        }
                    }
                }
            }
        }
        results
    }

    /// Details returns repository details
    pub fn details(&self) -> super::collection::CollectionDetails {
        let collections = self.collections.load();
        let mut movie_count = 0;
        let mut show_count = 0;
        let mut episode_count = 0;
        let mut genres = HashSet::new();
        let mut studios = HashSet::new();
        let mut official_ratings = HashSet::new();
        let mut years = HashSet::new();

        for c in collections.iter() {
            let details = c.details();
            movie_count += details.movie_count;
            show_count += details.show_count;
            episode_count += details.episode_count;
            for g in details.genres { genres.insert(g); }
            for s in details.studios { studios.insert(s); }
            for r in details.official_ratings { official_ratings.insert(r); }
            for y in details.years { years.insert(y); }
        }

        super::collection::CollectionDetails {
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
}

impl Default for CollectionRepo {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_repo() {
        let repo = CollectionRepo::new();
        assert_eq!(repo.get_collections().len(), 0);
    }

    #[test]
    fn test_add_collection() {
        let repo = CollectionRepo::new();
        
        let result = repo.add_collection(
            "Test Movies".to_string(),
            None,
            "movies",
            "/test/movies".to_string(),
            "".to_string(),
        );
        
        assert!(result.is_ok());
        assert_eq!(repo.get_collections().len(), 1);
    }

    #[test]
    fn test_add_invalid_collection_type() {
        let repo = CollectionRepo::new();
        
        let result = repo.add_collection(
            "Test".to_string(),
            None,
            "invalid",
            "/test".to_string(),
            "".to_string(),
        );
        
        assert!(result.is_err());
    }

    #[test]
    fn test_get_collection() {
        let repo = CollectionRepo::new();
        
        repo.add_collection(
            "Test Movies".to_string(),
            Some("test-id".to_string()),
            "movies",
            "/test/movies".to_string(),
            "".to_string(),
        ).unwrap();
        
        let collection = repo.get_collection("test-id");
        assert!(collection.is_some());
        assert_eq!(collection.unwrap().name, "Test Movies");
    }
}
