use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::*;
use tantivy::{doc, Index};
use std::path::Path;

use super::collection::Collection;
use super::item::Item;

/// Search document structure
#[derive(Debug, Clone)]
pub struct SearchDocument {
    pub id: String,
    pub collection_id: String,
    pub name: String,
    pub overview: String,
    pub genres: Vec<String>,
    pub item_type: String,
}

/// Search engine using Tantivy
pub struct Search {
    index: Index,
    schema: Schema,
}

impl Search {
    /// Create a new search index in memory
    pub fn new_in_memory() -> Result<Self, String> {
        let mut schema_builder = Schema::builder();
        
        schema_builder.add_text_field("id", STRING | STORED);
        schema_builder.add_text_field("collection_id", STRING | STORED);
        schema_builder.add_text_field("name", TEXT | STORED);
        schema_builder.add_text_field("overview", TEXT);
        schema_builder.add_text_field("genres", TEXT);
        schema_builder.add_text_field("item_type", STRING | STORED);
        
        let schema = schema_builder.build();
        let index = Index::create_in_ram(schema.clone());
        
        Ok(Self { index, schema })
    }

    /// Create a new search index on disk
    pub fn new_on_disk(path: &Path) -> Result<Self, String> {
        let mut schema_builder = Schema::builder();
        
        schema_builder.add_text_field("id", STRING | STORED);
        schema_builder.add_text_field("collection_id", STRING | STORED);
        schema_builder.add_text_field("name", TEXT | STORED);
        schema_builder.add_text_field("overview", TEXT);
        schema_builder.add_text_field("genres", TEXT);
        schema_builder.add_text_field("item_type", STRING | STORED);
        
        let schema = schema_builder.build();
        
        std::fs::create_dir_all(path).map_err(|e| e.to_string())?;
        let index = Index::create_in_dir(path, schema.clone())
            .map_err(|e| e.to_string())?;
        
        Ok(Self { index, schema })
    }

    /// Index a collection
    pub fn index_collection(&self, collection: &Collection) -> Result<(), String> {
        let mut index_writer = self.index.writer(50_000_000)
            .map_err(|e| e.to_string())?;
        
        let id_field = self.schema.get_field("id").unwrap();
        let collection_id_field = self.schema.get_field("collection_id").unwrap();
        let name_field = self.schema.get_field("name").unwrap();
        let overview_field = self.schema.get_field("overview").unwrap();
        let genres_field = self.schema.get_field("genres").unwrap();
        let item_type_field = self.schema.get_field("item_type").unwrap();
        
        for item in &collection.items {
            match item {
                Item::Movie(movie) => {
                    let genres_text = movie.metadata.genres().join(" ");
                    
                    index_writer.add_document(doc!(
                        id_field => movie.id.clone(),
                        collection_id_field => collection.id.clone(),
                        name_field => movie.name.clone(),
                        overview_field => movie.metadata.plot().to_string(),
                        genres_field => genres_text,
                        item_type_field => "movie".to_string(),
                    )).map_err(|e| e.to_string())?;
                }
                Item::Show(show) => {
                    let genres_text = show.metadata.genres().join(" ");
                    
                    index_writer.add_document(doc!(
                        id_field => show.id.clone(),
                        collection_id_field => collection.id.clone(),
                        name_field => show.name.clone(),
                        overview_field => show.metadata.plot().to_string(),
                        genres_field => genres_text,
                        item_type_field => "show".to_string(),
                    )).map_err(|e| e.to_string())?;
                    
                    // Index episodes
                    for season in &show.seasons {
                        for episode in &season.episodes {
                            index_writer.add_document(doc!(
                                id_field => episode.id.clone(),
                                collection_id_field => collection.id.clone(),
                                name_field => episode.name.clone(),
                                overview_field => episode.metadata.plot().to_string(),
                                genres_field => String::new(),
                                item_type_field => "episode".to_string(),
                            )).map_err(|e| e.to_string())?;
                        }
                    }
                }
                _ => {}
            }
        }
        
        index_writer.commit().map_err(|e| e.to_string())?;
        
        Ok(())
    }

    /// Search for items
    pub fn search(&self, query_str: &str, limit: usize) -> Result<Vec<SearchDocument>, String> {
        let reader = self.index
            .reader()
            .map_err(|e: tantivy::TantivyError| e.to_string())?;
        
        let searcher = reader.searcher();
        
        let name_field = self.schema.get_field("name").unwrap();
        let overview_field = self.schema.get_field("overview").unwrap();
        let genres_field = self.schema.get_field("genres").unwrap();
        
        let query_parser = QueryParser::for_index(
            &self.index,
            vec![name_field, overview_field, genres_field],
        );
        
        let query = query_parser.parse_query(query_str)
            .map_err(|e| e.to_string())?;
        
        let top_docs = searcher.search(&query, &TopDocs::with_limit(limit))
            .map_err(|e| e.to_string())?;
        
        let mut results = Vec::new();
        
        let id_field = self.schema.get_field("id").unwrap();
        let collection_id_field = self.schema.get_field("collection_id").unwrap();
        let item_type_field = self.schema.get_field("item_type").unwrap();
        
        for (_score, doc_address) in top_docs {
            let retrieved_doc: tantivy::TantivyDocument = searcher.doc(doc_address)
                .map_err(|e| e.to_string())?;
            
            let id = retrieved_doc.get_first(id_field)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            
            let collection_id = retrieved_doc.get_first(collection_id_field)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            
            let name = retrieved_doc.get_first(name_field)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            
            let overview = retrieved_doc.get_first(overview_field)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            
            let genres_text = retrieved_doc.get_first(genres_field)
                .and_then(|v| v.as_str())
                .unwrap_or("");
            
            let genres = if genres_text.is_empty() {
                Vec::new()
            } else {
                genres_text.split_whitespace().map(|s| s.to_string()).collect()
            };
            
            let item_type = retrieved_doc.get_first(item_type_field)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            
            results.push(SearchDocument {
                id,
                collection_id,
                name,
                overview,
                genres,
                item_type,
            });
        }
        
        Ok(results)
    }

    /// Find similar items (stub for now)
    pub fn similar(&self, _item_id: &str, _limit: usize) -> Result<Vec<SearchDocument>, String> {
        // TODO: Implement more-like-this query
        Ok(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_in_memory_search() {
        let search = Search::new_in_memory();
        assert!(search.is_ok());
    }

    #[test]
    fn test_search_empty_index() {
        let search = Search::new_in_memory().unwrap();
        let results = search.search("test", 10);
        assert!(results.is_ok());
        assert_eq!(results.unwrap().len(), 0);
    }
}
