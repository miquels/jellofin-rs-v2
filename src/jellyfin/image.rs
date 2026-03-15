use axum::{
    body::Body,
    extract::{Path as AxumPath, Query, State},
    http::{self, HeaderMap, StatusCode},
    response::{IntoResponse, Redirect, Response},
    Extension,
};
use serde::Deserialize;
use std::path::PathBuf;
use tower::ServiceExt;
use tower_http::services::ServeFile;

use tracing::warn;

use super::jellyfin::JellyfinState;
use crate::collection::item::Item;
use crate::collection::CollectionRepo;
use crate::database::model::AccessToken;
use crate::database::ImageMetadata;
use crate::idhash::*;

#[derive(Debug, Deserialize)]
pub struct ImageParams {
    pub width: Option<u32>,
    pub height: Option<u32>,
    #[serde(rename = "maxWidth")]
    pub max_width: Option<u32>,
    #[serde(rename = "maxHeight")]
    pub max_height: Option<u32>,
    #[serde(rename = "fillWidth")]
    pub fill_width: Option<u32>,
    #[serde(rename = "fillHeight")]
    pub fill_height: Option<u32>,
    pub quality: Option<u32>,
    pub tag: Option<String>,
    #[serde(rename = "type")]
    pub image_type: Option<String>,
}

/// GET /Items/{item}/Images/{type}
pub async fn get_item_image(
    State(state): State<JellyfinState>,
    AxumPath((item_id, image_type)): AxumPath<(String, String)>,
    Query(params): Query<ImageParams>,
    req: http::Request<Body>,
) -> Result<Response, StatusCode> {
    get_image_common(state, item_id, image_type, 0, params, req).await
}

/// GET /Items/{item}/Images/{type}/{index}
/// Note: Index is currently ignored/validated as 0 for single-image types, but supports the route structure.
pub async fn get_item_image_indexed(
    State(state): State<JellyfinState>,
    AxumPath((item_id, image_type, index)): AxumPath<(String, String, usize)>,
    Query(params): Query<ImageParams>,
    req: http::Request<Body>,
) -> Result<Response, StatusCode> {
    get_image_common(state, item_id, image_type, index, params, req).await
}

async fn get_image_common(
    state: JellyfinState,
    item_id: String,
    image_type: String,
    index: usize,
    params: ImageParams,
    req: http::Request<Body>,
) -> Result<Response, StatusCode> {
    // Handle tags (redirect / local file)
    if let Some(tag) = &params.tag {
        // Jellyfin redirect tag.
        if let Some(url) = tag.strip_prefix("redirect_") {
            return Ok(Redirect::to(url).into_response());
        }

        // Jellyfin 'open local file' tag.
        if let Some(file) = tag.strip_prefix("file_") {
            // Let's only allow images, shall we.
            const IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "webp"];
            if let Some((_, ext)) = file.rsplit_once('.') {
                if IMAGE_EXTENSIONS.contains(&ext.to_lowercase().as_str()) {
                    let service = ServeFile::new(file);
                    let response = service
                        .oneshot(req)
                        .await
                        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                    return Ok(response.map(Body::new));
                }
            }
            return Err(StatusCode::NOT_FOUND);
        }
    }

    if index > 0 {
        return Err(StatusCode::NOT_FOUND);
    }

    // Check DB first
    if let Ok(Some(meta)) = state.repo.has_image(&item_id, &image_type).await {
        if let Ok((_, data)) = state.repo.get_image(&item_id, &image_type).await {
            return Ok((
                [
                    (axum::http::header::CONTENT_TYPE, meta.mime_type.clone()),
                    (axum::http::header::ETAG, format!("\"{}\"", meta.etag)),
                ],
                data,
            )
                .into_response());
        }
    }

    let image_path = find_image_path(&state.collections, &item_id, &image_type).ok_or_else(|| {
        warn!("Image not found: item_id={}, image_type={}", item_id, image_type);
        StatusCode::NOT_FOUND
    })?;

    // Determine quality: strictly following user suggestion but falling back to path type if param unavailable
    let type_to_check = params.image_type.as_deref().unwrap_or(image_type.as_str());

    // Config defaults
    let quality = if params.quality.is_some() {
        params.quality
    } else {
        match type_to_check.to_lowercase().as_str() {
            "primary" | "logo" => Some(state.config.jellyfin.image_quality_poster),
            _ => None,
        }
    };

    // Resolve width/height from various params
    let width = params.width.or(params.max_width).or(params.fill_width);
    let height = params.height.or(params.max_height).or(params.fill_height);

    let serve_path = state
        .image_resizer
        .resize_image(&image_path, width, height, quality);

    // Ensure the file exists
    if !serve_path.exists() {
        return Err(StatusCode::NOT_FOUND);
    }

    // Use ServeFile for proper ETag and Range header support
    let service = ServeFile::new(serve_path);
    let response = service
        .oneshot(req)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(response.map(Body::new))
}

fn find_image_path(collections: &CollectionRepo, item_id: &str, image_type: &str) -> Option<PathBuf> {
    let (collection, item) = collections.get_item_by_id(item_id).or_else(|| {
        warn!("find_image_path: item not found for id={}", item_id);
        None
    })?;

    let image_filename = match image_type.to_lowercase().as_str() {
        "primary" | "poster" => match &item {
            Item::Movie(m) => m.poster.as_str(),
            Item::Show(s) => s.poster.as_str(),
            Item::Season(s) => s.poster(),
            Item::Episode(e) => e.thumb.as_str(),
            _ => "",
        },
        "backdrop" | "fanart" => match &item {
            Item::Movie(m) => m.fanart.as_str(),
            Item::Show(s) => s.fanart.as_str(),
            Item::Season(s) => s.fanart.as_str(),
            _ => "",
        },
        "banner" => match &item {
            Item::Movie(m) => m.banner.as_str(),
            Item::Show(s) => s.banner.as_str(),
            Item::Season(s) => s.banner.as_str(),
            _ => "",
        },
        "thumb" => match &item {
            Item::Episode(e) => e.thumb.as_str(),
            _ => "",
        },
        "logo" => match &item {
            Item::Show(s) => s.logo.as_str(),
            _ => "",
        },
        _ => return None,
    };

    if image_filename.is_empty() {
        warn!(
            "find_image_path: empty image filename for id={}, type={}",
            item_id, image_type
        );
        return None;
    }

    let item_path = match &item {
        Item::Movie(m) => m.path.as_str(),
        Item::Show(s) => s.path.as_str(),
        Item::Season(s) => s.path.as_str(),
        Item::Episode(e) => e.path.as_str(),
        _ => "",
    };

    let mut path = PathBuf::from(&collection.directory);
    if !item_path.is_empty() {
        path.push(item_path);
    }
    path.push(image_filename);

    if path.exists() {
        Some(path)
    } else {
        warn!("find_image_path: file does not exist: {}", path.display());
        None
    }
}

/// POST /Items/{item}/Images/{type} — upload image to DB
pub async fn post_item_image(
    Extension(_token): Extension<AccessToken>,
    State(state): State<JellyfinState>,
    AxumPath((item_id, image_type)): AxumPath<(String, String)>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> StatusCode {
    let mime_type = headers
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("image/jpeg")
        .to_string();

    let etag = hash_bytes(&body);
    let meta = ImageMetadata {
        mime_type,
        file_size: body.len() as i64,
        etag,
        updated: chrono::Utc::now(),
    };

    match state.repo.store_image(&item_id, &image_type, &meta, &body).await {
        Ok(_) => StatusCode::NO_CONTENT,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

/// DELETE /Items/{item}/Images/{type} — delete image from DB
pub async fn delete_item_image(
    Extension(_token): Extension<AccessToken>,
    State(state): State<JellyfinState>,
    AxumPath((item_id, image_type)): AxumPath<(String, String)>,
) -> StatusCode {
    match state.repo.delete_image(&item_id, &image_type).await {
        Ok(_) => StatusCode::NO_CONTENT,
        Err(_) => StatusCode::NOT_FOUND,
    }
}

/// GET /Users/{id}/Images/{type} — serve user profile image
pub async fn get_user_image(
    State(state): State<JellyfinState>,
    AxumPath((user_id, image_type)): AxumPath<(String, String)>,
) -> Result<Response, StatusCode> {
    let meta = state
        .repo
        .has_image(&user_id, &image_type)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let (_, data) = state
        .repo
        .get_image(&user_id, &image_type)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    Ok((
        [
            (axum::http::header::CONTENT_TYPE, meta.mime_type),
            (axum::http::header::ETAG, format!("\"{}\"", meta.etag)),
        ],
        data,
    )
        .into_response())
}

/// POST /UserImage — upload current user's profile image
pub async fn post_user_image(
    Extension(token): Extension<AccessToken>,
    State(state): State<JellyfinState>,
    Query(params): Query<std::collections::HashMap<String, String>>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> StatusCode {
    let user_id = params
        .get("userId")
        .cloned()
        .unwrap_or_else(|| token.user_id.clone());

    let mime_type = headers
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("image/jpeg")
        .to_string();

    let etag = hash_bytes(&body);
    let meta = ImageMetadata {
        mime_type,
        file_size: body.len() as i64,
        etag,
        updated: chrono::Utc::now(),
    };

    match state.repo.store_image(&user_id, "Primary", &meta, &body).await {
        Ok(_) => StatusCode::NO_CONTENT,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

/// DELETE /UserImage — delete current user's profile image
pub async fn delete_user_image(
    Extension(token): Extension<AccessToken>,
    State(state): State<JellyfinState>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> StatusCode {
    let user_id = params
        .get("userId")
        .cloned()
        .unwrap_or_else(|| token.user_id.clone());

    match state.repo.delete_image(&user_id, "Primary").await {
        Ok(_) => StatusCode::NO_CONTENT,
        Err(_) => StatusCode::NOT_FOUND,
    }
}

/// GET /Genres/{name}/Images/{type} — serve genre image from DB
pub async fn get_genre_image(
    State(state): State<JellyfinState>,
    AxumPath((name, image_type)): AxumPath<(String, String)>,
) -> Result<Response, StatusCode> {
    let genre_id = id_hash_prefix(ITEM_PREFIX_GENRE, &name);
    get_db_image(&state, &genre_id, &image_type).await
}

/// POST /Genres/{name}/Images/{type} — upload genre image
pub async fn post_genre_image(
    Extension(_token): Extension<AccessToken>,
    State(state): State<JellyfinState>,
    AxumPath((name, image_type)): AxumPath<(String, String)>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> StatusCode {
    let genre_id = id_hash_prefix(ITEM_PREFIX_GENRE, &name);
    store_db_image(&state, &genre_id, &image_type, &headers, &body).await
}

/// GET /Studios/{name}/Images/{type} — serve studio image from DB
pub async fn get_studio_image(
    State(state): State<JellyfinState>,
    AxumPath((name, image_type)): AxumPath<(String, String)>,
) -> Result<Response, StatusCode> {
    let studio_id = id_hash_prefix(ITEM_PREFIX_STUDIO, &name);
    get_db_image(&state, &studio_id, &image_type).await
}

/// POST /Studios/{name}/Images/{type} — upload studio image
pub async fn post_studio_image(
    Extension(_token): Extension<AccessToken>,
    State(state): State<JellyfinState>,
    AxumPath((name, image_type)): AxumPath<(String, String)>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> StatusCode {
    let studio_id = id_hash_prefix(ITEM_PREFIX_STUDIO, &name);
    store_db_image(&state, &studio_id, &image_type, &headers, &body).await
}

/// GET /Persons/{name}/Images/{type} — serve person image from DB
pub async fn get_person_image(
    State(state): State<JellyfinState>,
    AxumPath((name, image_type)): AxumPath<(String, String)>,
) -> Result<Response, StatusCode> {
    let person_id = id_hash_prefix(ITEM_PREFIX_PERSON, &name);
    get_db_image(&state, &person_id, &image_type).await
}

/// POST /Persons/{name}/Images/{type} — upload person image
pub async fn post_person_image(
    Extension(_token): Extension<AccessToken>,
    State(state): State<JellyfinState>,
    AxumPath((name, image_type)): AxumPath<(String, String)>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> StatusCode {
    let person_id = id_hash_prefix(ITEM_PREFIX_PERSON, &name);
    store_db_image(&state, &person_id, &image_type, &headers, &body).await
}

async fn get_db_image(
    state: &JellyfinState,
    item_id: &str,
    image_type: &str,
) -> Result<Response, StatusCode> {
    let meta = state
        .repo
        .has_image(item_id, image_type)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let (_, data) = state
        .repo
        .get_image(item_id, image_type)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    Ok((
        [
            (axum::http::header::CONTENT_TYPE, meta.mime_type),
            (axum::http::header::ETAG, format!("\"{}\"", meta.etag)),
        ],
        data,
    )
        .into_response())
}

async fn store_db_image(
    state: &JellyfinState,
    item_id: &str,
    image_type: &str,
    headers: &HeaderMap,
    body: &axum::body::Bytes,
) -> StatusCode {
    let mime_type = headers
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("image/jpeg")
        .to_string();

    let etag = hash_bytes(body);
    let meta = ImageMetadata {
        mime_type,
        file_size: body.len() as i64,
        etag,
        updated: chrono::Utc::now(),
    };

    match state.repo.store_image(item_id, image_type, &meta, body).await {
        Ok(_) => StatusCode::NO_CONTENT,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}
