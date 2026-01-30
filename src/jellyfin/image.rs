use axum::{
    body::Body,
    extract::{Path as AxumPath, Query, State},
    http::{self, StatusCode},
    response::{IntoResponse, Redirect, Response},
};
use serde::Deserialize;
use std::path::PathBuf;
use tower::ServiceExt;
use tower_http::services::ServeFile;

use super::jellyfin::JellyfinState;
use super::jfitem::trim_prefix;
use crate::collection::item::Item;
use crate::collection::CollectionRepo;

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

    let internal_id = trim_prefix(&item_id);
    let image_path = find_image_path(&state.collections, internal_id, &image_type)
        .ok_or(StatusCode::NOT_FOUND)?;

    // Determine quality: strictly following user suggestion but falling back to path type if param unavailable
    let type_to_check = params
        .image_type
        .as_deref()
        .unwrap_or(image_type.as_str());

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
    let width = params
        .width
        .or(params.max_width)
        .or(params.fill_width);
    let height = params
        .height
        .or(params.max_height)
        .or(params.fill_height);

    let serve_path = state
        .image_resizer
        .resize_image(&image_path, width, height, quality) // map_err not needed locally if returns pathbuf, but user code had map_err. Our resizer returns PathBuf currently, user code expected result.
        // Waiting, my resize_image returns PathBuf (original if fail).
        // User snippet: .resize_image(...).map_err(...)
        // I should check `imageresize/mod.rs`. It returns PathBuf.
        // So I can't map_err. I just use the result.
        ;
    
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
    let (collection, item) = collections.get_item_by_id(item_id)?;

    let image_filename = match image_type {
        "Primary" | "Poster" => match &item {
            Item::Movie(m) => &m.poster,
            Item::Show(s) => &s.poster,
            Item::Season(s) => s.poster(),
            Item::Episode(e) => &e.thumb,
        },
        "Backdrop" | "Fanart" => match &item {
            Item::Movie(m) => &m.fanart,
            Item::Show(s) => &s.fanart,
            Item::Season(s) => &s.fanart,
            Item::Episode(_) => "",
        },
        "Banner" => match &item {
            Item::Movie(m) => &m.banner,
            Item::Show(s) => &s.banner,
            Item::Season(s) => &s.banner,
            Item::Episode(_) => "",
        },
        "Thumb" => match &item {
            Item::Episode(e) => &e.thumb,
            _ => "",
        },
        "Logo" => match &item {
            Item::Show(s) => &s.logo,
            _ => "",
        },
        _ => return None,
    };

    if image_filename.is_empty() {
        return None;
    }

    let item_path = match &item {
        Item::Movie(m) => &m.path,
        Item::Show(s) => &s.path,
        Item::Season(s) => &s.path,
        Item::Episode(e) => &e.path,
    };

    let mut path = PathBuf::from(&collection.directory);
    if !item_path.is_empty() {
        path.push(item_path);
    }
    path.push(image_filename);

    if path.exists() {
        Some(path)
    } else {
        None
    }
}
