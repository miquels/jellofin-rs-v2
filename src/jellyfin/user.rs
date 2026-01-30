use super::jellyfin::{make_user, JellyfinState};
use super::types::*;
use crate::database::model;
use axum::{
    extract::{Path as AxumPath, Query, State},
    http::StatusCode,
    response::Json,
    Extension,
};

#[derive(serde::Deserialize)]
pub struct UserViewsQuery {
    #[serde(rename = "userId")]
    pub user_id: Option<String>,
}

/// GET /Users - Get all users (returns current user only)
pub async fn users_all(
    Extension(token): Extension<model::AccessToken>,
    State(state): State<JellyfinState>,
) -> Result<Json<Vec<User>>, StatusCode> {
    let user: model::User = state
        .repo
        .get_user_by_id(&token.user_id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    Ok(Json(vec![make_user(&user, &state.server_id)]))
}

/// GET /Users/Me - Get current user
pub async fn users_me(
    Extension(token): Extension<model::AccessToken>,
    State(state): State<JellyfinState>,
) -> Result<Json<User>, StatusCode> {
    let user: model::User = state
        .repo
        .get_user_by_id(&token.user_id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    Ok(Json(make_user(&user, &state.server_id)))
}

/// GET /Users/{id} - Get user by ID
pub async fn users_by_id(
    Extension(token): Extension<model::AccessToken>,
    State(state): State<JellyfinState>,
    AxumPath(user_id): AxumPath<String>,
) -> Result<Json<User>, StatusCode> {
    if user_id != token.user_id {
        return Err(StatusCode::NOT_FOUND);
    }

    let user: model::User = state
        .repo
        .get_user_by_id(&token.user_id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    Ok(Json(make_user(&user, &state.server_id)))
}

/// GET /Users/Public - Get public users (returns empty list)
pub async fn users_public() -> Json<Vec<User>> {
    Json(Vec::new())
}

// Helper to build a rich BaseItemDto for a collection
fn build_collection_dto(collection: &crate::collection::Collection, server_id: &str) -> BaseItemDto {
    let mut dto = BaseItemDto::default();
    dto.name = collection.name.clone();
    dto.sort_name = Some(collection.name.to_lowercase());
    // Prefix ID with collection_ to match Go server behavior
    dto.id = format!("collection_{}", collection.id);
    // Use a derived root ID
    dto.parent_id = Some(format!("root_{}", server_id));
    dto.server_id = server_id.to_string();
    dto.item_type = "CollectionFolder".to_string();
    dto.media_type = Some("Unknown".to_string());
    
    // Add Etag and DisplayPreferencesId
    dto.etag = Some(crate::idhash::id_hash(&format!("etag_{}", collection.id)));
    dto.display_preferences_id = Some(format!("dp_{}", collection.id));
    dto.primary_image_aspect_ratio = Some(1.7777777777777777); // Standard 16:9
    dto.provider_ids = Some(std::collections::HashMap::new()); // Empty object {}
    
    dto.is_folder = Some(true);
    dto.is_hd = Some(false);
    dto.is_4k = Some(false);
    dto.lock_data = Some(false);
    
    dto.play_access = Some("Full".to_string());
    dto.location_type = Some("FileSystem".to_string());
    dto.can_delete = Some(false);
    dto.can_download = Some(true);
    dto.date_created = Some(chrono::Utc::now()); // Placeholder
    dto.premiere_date = Some(chrono::Utc::now()); // Placeholder
    dto.enable_media_source_display = Some(true);
    dto.path = Some("/collection".to_string()); // Dummy path
    
    // Map collection type
    let ctype = match collection.collection_type {
        crate::collection::CollectionType::Movies => "movies",
        crate::collection::CollectionType::Shows => "tvshows",
    };
    dto.collection_type = Some(ctype.to_string());

    // Calculate stats
    // Note: details() iterates all items, which gives accurate counts/genres
    let details = collection.details();
    let child_count = match collection.collection_type {
        crate::collection::CollectionType::Movies => details.movie_count,
        crate::collection::CollectionType::Shows => details.show_count,
    };
    dto.child_count = Some(child_count as i32);
    
    dto.genres = Some(details.genres.clone());
    
    // Construct GenreItems
    let genre_items: Vec<GenreItem> = details.genres.iter().map(|g| {
        GenreItem {
            name: g.clone(),
            id: format!("genre_{}", crate::idhash::id_hash(g)), // Match Go format "genre_HASH" if possible, but hash is fine
        }
    }).collect();
    dto.genre_items = Some(genre_items);
    
    dto
}

/// GET /Users/{id}/Views - Get user views (libraries)
pub async fn user_views(
    Extension(_token): Extension<model::AccessToken>,
    State(state): State<JellyfinState>,
    AxumPath(_user_id): AxumPath<String>,
) -> Json<QueryResult<BaseItemDto>> {
    let collections = state.collections.get_collections();
    let mut items = Vec::new();

    for collection in collections {
        items.push(build_collection_dto(&collection, &state.server_id));
    }

    Json(QueryResult {
        total_record_count: items.len() as i32,
        start_index: 0,
        items,
    })
}

/// GET /UserViews - Get user views (libraries) with query param
pub async fn user_views_query(
    Extension(_token): Extension<model::AccessToken>,
    State(state): State<JellyfinState>,
    Query(_query): Query<UserViewsQuery>,
) -> Json<QueryResult<BaseItemDto>> {
    let collections = state.collections.get_collections();
    let mut items = Vec::new();

    for collection in collections {
        items.push(build_collection_dto(&collection, &state.server_id));
    }

    Json(QueryResult {
        total_record_count: items.len() as i32,
        start_index: 0,
        items,
    })
}
