use arc_swap::ArcSwap;
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
            for item in &collection.items {
                match item {
                    Item::Movie(movie) if movie.id == item_id => {
                        return Some((collection.clone(), item.clone()));
                    }
                    Item::Show(show) if show.id == item_id => {
                        return Some((collection.clone(), item.clone()));
                    }
                    Item::Show(show) => {
                        for season in &show.seasons {
                            if season.id == item_id {
                                return Some((collection.clone(), Item::Season(season.clone())));
                            }
                            for episode in &season.episodes {
                                if episode.id == item_id {
                                    return Some((collection.clone(), Item::Episode(episode.clone())));
                                }
                            }
                        }
                    }
                    Item::Season(season) if season.id == item_id => {
                        return Some((collection.clone(), item.clone()));
                    }
                    Item::Episode(episode) if episode.id == item_id => {
                        return Some((collection.clone(), item.clone()));
                    }
                    _ => {}
                }
            }
        }
        
        None
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
