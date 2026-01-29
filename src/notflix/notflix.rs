use axum::{
    body::Body,
    extract::{Path as AxumPath, Query, State},
    http::{header, HeaderMap, Request, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use chrono::Datelike;
use std::sync::Arc;
use urlencoding::encode;

use crate::collection::{CollectionRepo, Item as CollectionItem};
use crate::imageresize::ImageResizer;
use tower::ServiceExt;
use tower_http::services::ServeFile;
use tracing::warn;

use super::etag::{check_etag, check_etag_obj};
use super::proxy::hls_handler;
use super::subtitles::open_sub;
use super::types::*;

#[derive(Clone)]
pub struct NotflixState {
    pub collections: Arc<CollectionRepo>,
    pub image_resizer: Arc<ImageResizer>,
    pub app_dir: String,
}

/// GET /api/collections - List all collections
pub async fn collections_handler(State(state): State<NotflixState>) -> Result<Json<Vec<Collection>>, StatusCode> {
    let collections = state.collections.get_collections();
    let result: Vec<Collection> = collections
        .iter()
        .map(|c| Collection {
            id: c.id.clone(),
            name: c.name.clone(),
            collection_type: c.collection_type.as_str().to_string(),
            items: None,
        })
        .collect();

    Ok(Json(result))
}

/// GET /api/collection/{id} - Get single collection
pub async fn collection_handler(
    State(state): State<NotflixState>,
    AxumPath(collection_id): AxumPath<String>,
) -> Result<Json<Collection>, StatusCode> {
    let collection = state
        .collections
        .get_collection(&collection_id)
        .ok_or(StatusCode::NOT_FOUND)?;

    let result = Collection {
        id: collection.id.clone(),
        name: collection.name.clone(),
        collection_type: collection.collection_type.as_str().to_string(),
        items: None,
    };

    Ok(Json(result))
}

/// GET /api/collection/{id}/items - Get all items in collection
pub async fn items_handler(
    State(state): State<NotflixState>,
    AxumPath(collection_id): AxumPath<String>,
    headers: HeaderMap,
) -> Result<Response, StatusCode> {
    let collection = state
        .collections
        .get_collection(&collection_id)
        .ok_or(StatusCode::NOT_FOUND)?;

    // Calculate last video time for ETag
    let mut last_video = chrono::DateTime::<chrono::Utc>::MIN_UTC;
    for item in &collection.items {
        if let CollectionItem::Show(show) = item {
            if show.last_video > last_video {
                last_video = show.last_video;
            }
        }
    }

    // Check ETag
    if last_video > chrono::DateTime::<chrono::Utc>::MIN_UTC {
        if let Some(resp) = check_etag_obj(&headers, last_video) {
            return Ok(resp);
        }
    }

    let items: Vec<Item> = collection
        .items
        .iter()
        .map(|item| copy_item(item, &collection.id))
        .collect();

    Ok(Json(items).into_response())
}

/// GET /api/collection/{coll}/item/{item} - Get single item
pub async fn item_handler(
    State(state): State<NotflixState>,
    AxumPath((collection_id, item_id)): AxumPath<(String, String)>,
    Query(params): Query<std::collections::HashMap<String, String>>,
    headers: HeaderMap,
) -> Result<Response, StatusCode> {
    let item = state
        .collections
        .get_item(&collection_id, &item_id)
        .ok_or(StatusCode::NOT_FOUND)?;

    // Check ETag based on item type
    match &item {
        CollectionItem::Movie(movie) => {
            if let Some(resp) = check_etag_obj(&headers, movie.created) {
                return Ok(resp);
            }
        }
        CollectionItem::Show(show) => {
            if let Some(resp) = check_etag_obj(&headers, show.last_video) {
                return Ok(resp);
            }
        }
        _ => {}
    }

    let do_nfo = !params.contains_key("nonfo");
    let mut result = copy_item(&item, &collection_id);

    // Add seasons for shows
    if let CollectionItem::Show(show) = &item {
        let seasons: Vec<Season> = show.seasons.iter().map(|s| copy_season(s, do_nfo)).collect();
        result.seasons = Some(seasons);
    }

    Ok(Json(result).into_response())
}

/// GET /api/collection/{id}/genres - Get genre counts
pub async fn genres_handler(
    State(state): State<NotflixState>,
    AxumPath(collection_id): AxumPath<String>,
) -> Result<Json<std::collections::HashMap<String, usize>>, StatusCode> {
    let collection = state
        .collections
        .get_collection(&collection_id)
        .ok_or(StatusCode::NOT_FOUND)?;

    let genre_count = collection.genre_count();

    Ok(Json(genre_count))
}

/// GET /data/{source}/{path} - Get media data
pub async fn data_handler(
    State(state): State<NotflixState>,
    AxumPath((source, path_string)): AxumPath<(String, String)>,
    Query(params): Query<std::collections::HashMap<String, String>>,
    headers: HeaderMap,
    req: Request<Body>,
) -> Response {
    // Try HLS handler first
    let hls_req = Request::builder()
        .method(req.method())
        .uri(req.uri())
        .version(req.version());
    let mut hls_req = hls_req.body(Body::empty()).unwrap();
    *hls_req.headers_mut() = req.headers().clone();

    let hls_resp = hls_handler(
        State(state.clone()),
        AxumPath((source.clone(), path_string.clone())),
        hls_req,
    )
    .await
    .into_response();
    if hls_resp.status() != StatusCode::NOT_FOUND {
        return hls_resp;
    }

    let collection = match state.collections.get_collection(&source) {
        Some(c) => c,
        None => return StatusCode::NOT_FOUND.into_response(),
    };

    let base_path = std::path::Path::new(&collection.directory);
    let file_path = base_path.join(path_string.trim_start_matches('/'));
    let file_path_str = file_path.to_str().unwrap_or_default();
    
    tracing::debug!("Data handler: source={} path_string={} -> resolved={}", source, path_string, file_path_str);

    let ext = file_path.extension().and_then(|s| s.to_str()).unwrap_or("");
    if ext == "srt" || ext == "vtt" {
        return open_sub(&headers, file_path_str).into_response();
    }

    // Parse resize parameters
    let width = params.get("width").or_else(|| params.get("w")).and_then(|v| v.parse().ok());
    let height = params.get("height").or_else(|| params.get("h")).and_then(|v| v.parse().ok());
    let quality = params.get("quality").or_else(|| params.get("q")).and_then(|v| v.parse().ok());

    // Try image resizer
    let resized_path = state
        .image_resizer
        .resize_image(std::path::Path::new(file_path_str), width, height, quality);
    if resized_path.exists() {
        // Look up metadata for ETag check
        if let Ok(metadata) = std::fs::metadata(&resized_path) {
            if let Some(resp) = check_etag(&headers, file_path_str, &metadata) {
                return resp;
            }
        }

        match ServeFile::new(resized_path).oneshot(req).await {
            Ok(response) => {
                let mut response = response.into_response();
                response.headers_mut().insert(
                    header::CACHE_CONTROL,
                    "max-age=86400, stale-while-revalidate=300".parse().unwrap(),
                );
                response
            }
            Err(e) => {
                warn!("Failed to serve file: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}

/// GET /v/{path} - Index handler
pub async fn index_handler(State(state): State<NotflixState>) -> impl IntoResponse {
    let index_path = std::path::PathBuf::from(&state.app_dir).join("index.html");
    match std::fs::read(index_path) {
        Ok(content) => ([(header::CONTENT_TYPE, "text/html")], content).into_response(),
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

/// Helper: Copy collection item to API type
fn copy_item(item: &CollectionItem, _collection_id: &str) -> Item {
    match item {
        CollectionItem::Movie(movie) => {
            let premiered = movie
                .metadata
                .premiered
                .filter(|dt: &chrono::DateTime<chrono::Utc>| dt.year() > 1900)
                .map(|dt: chrono::DateTime<chrono::Utc>| dt.format("%Y-%m-%d").to_string());

            let studio = if !movie.metadata.studios.is_empty() {
                Some(movie.metadata.studios[0].clone())
            } else {
                None
            };

            Item {
                id: movie.id.clone(),
                name: movie.name.clone(),
                path: escape_path(&movie.path),
                baseurl: movie.base_url.clone(),
                item_type: "movie".to_string(),
                firstvideo: Some(movie.created.timestamp_millis()),
                lastvideo: Some(movie.created.timestamp_millis()),
                sort_name: Some(movie.sort_name.clone()),
                nfo: ItemNfo {
                    id: movie.id.clone(),
                    title: Some(movie.metadata.title.clone()),
                    plot: Some(movie.metadata.plot.clone()),
                    genre: Some(movie.metadata.genres.clone()),
                    premiered: premiered.clone(),
                    mpaa: Some(movie.metadata.official_rating.clone()),
                    aired: premiered,
                    studio,
                    rating: Some(movie.metadata.rating),
                },
                banner: if movie.banner.is_empty() {
                    None
                } else {
                    Some(movie.banner.clone())
                },
                fanart: if movie.fanart.is_empty() {
                    None
                } else {
                    Some(movie.fanart.clone())
                },
                folder: if movie.folder.is_empty() {
                    None
                } else {
                    Some(movie.folder.clone())
                },
                poster: if movie.poster.is_empty() {
                    None
                } else {
                    Some(escape_path(&movie.poster))
                },
                rating: Some(movie.metadata.rating),
                votes: None,
                genre: Some(movie.metadata.genres.clone()),
                year: movie.metadata.year,
                video: Some(escape_path(&movie.file_name)),
                thumb: None,
                srtsubs: None,
                vttsubs: None,
                season_all_banner: None,
                season_all_poster: None,
                seasons: None,
            }
        }
        CollectionItem::Show(show) => {
            let premiered = show
                .metadata
                .premiered
                .filter(|dt: &chrono::DateTime<chrono::Utc>| dt.year() > 1900)
                .map(|dt: chrono::DateTime<chrono::Utc>| dt.format("%Y-%m-%d").to_string());

            let studio = if !show.metadata.studios.is_empty() {
                Some(show.metadata.studios[0].clone())
            } else {
                None
            };

            Item {
                id: show.id.clone(),
                name: show.name.clone(),
                path: escape_path(&show.path),
                baseurl: show.base_url.clone(),
                item_type: "show".to_string(),
                firstvideo: Some(show.first_video.timestamp_millis()),
                lastvideo: Some(show.last_video.timestamp_millis()),
                sort_name: Some(show.sort_name.clone()),
                nfo: ItemNfo {
                    id: show.id.clone(),
                    title: Some(show.metadata.title.clone()),
                    plot: Some(show.metadata.plot.clone()),
                    genre: Some(show.metadata.genres.clone()),
                    premiered: premiered.clone(),
                    mpaa: Some(show.metadata.official_rating.clone()),
                    aired: premiered,
                    studio,
                    rating: Some(show.metadata.rating),
                },
                banner: if show.banner.is_empty() {
                    None
                } else {
                    Some(show.banner.clone())
                },
                fanart: if show.fanart.is_empty() {
                    None
                } else {
                    Some(show.fanart.clone())
                },
                folder: if show.folder.is_empty() {
                    None
                } else {
                    Some(show.folder.clone())
                },
                poster: if show.poster.is_empty() {
                    None
                } else {
                    Some(escape_path(&show.poster))
                },
                rating: Some(show.metadata.rating),
                votes: None,
                genre: Some(show.metadata.genres.clone()),
                year: show.metadata.year,
                video: None,
                thumb: None,
                srtsubs: None,
                vttsubs: None,
                season_all_banner: if show.season_all_banner.is_empty() {
                    None
                } else {
                    Some(show.season_all_banner.clone())
                },
                season_all_poster: if show.season_all_poster.is_empty() {
                    None
                } else {
                    Some(show.season_all_poster.clone())
                },
                seasons: None,
            }
        }
        _ => Item {
            id: String::new(),
            name: String::new(),
            path: String::new(),
            baseurl: String::new(),
            item_type: "unknown".to_string(),
            firstvideo: None,
            lastvideo: None,
            sort_name: None,
            nfo: ItemNfo {
                id: String::new(),
                title: None,
                plot: None,
                genre: None,
                premiered: None,
                mpaa: None,
                aired: None,
                studio: None,
                rating: None,
            },
            banner: None,
            fanart: None,
            folder: None,
            poster: None,
            rating: None,
            votes: None,
            genre: None,
            year: None,
            video: None,
            thumb: None,
            srtsubs: None,
            vttsubs: None,
            season_all_banner: None,
            season_all_poster: None,
            seasons: None,
        },
    }
}

/// Helper: Copy season to API type
fn copy_season(season: &crate::collection::Season, do_nfo: bool) -> Season {
    let episodes: Vec<Episode> = season.episodes.iter().map(|e| copy_episode(e, do_nfo)).collect();

    Season {
        seasonno: season.season_no,
        banner: if season.banner.is_empty() {
            None
        } else {
            Some(escape_path(&season.banner))
        },
        fanart: if season.fanart.is_empty() {
            None
        } else {
            Some(escape_path(&season.fanart))
        },
        poster: if season.poster.is_empty() {
            None
        } else {
            Some(escape_path(&season.poster))
        },
        episodes: Some(episodes),
    }
}

/// Helper: Copy episode to API type
fn copy_episode(episode: &crate::collection::Episode, do_nfo: bool) -> Episode {
    let nfo = if do_nfo {
        let aired = episode
            .metadata
            .premiered
            .filter(|dt: &chrono::DateTime<chrono::Utc>| dt.year() > 1900)
            .map(|dt: chrono::DateTime<chrono::Utc>| dt.format("%Y-%m-%d").to_string());

        EpisodeNfo {
            title: Some(episode.metadata.title.clone()),
            plot: Some(episode.metadata.plot.clone()),
            season: Some(episode.season_no.to_string()),
            episode: Some(episode.episode_no.to_string()),
            aired,
        }
    } else {
        EpisodeNfo {
            title: None,
            plot: None,
            season: None,
            episode: None,
            aired: None,
        }
    };

    Episode {
        name: episode.name.clone(),
        seasonno: episode.season_no,
        episodeno: episode.episode_no,
        double: if episode.double { Some(true) } else { None },
        sort_name: Some(episode.sort_name.clone()),
        nfo,
        video: escape_path(&episode.file_name),
        thumb: if episode.thumb.is_empty() {
            None
        } else {
            Some(episode.thumb.clone())
        },
        srtsubs: None,
        vttsubs: None,
    }
}

/// Helper: URL-escape a path
fn escape_path(path: &str) -> String {
    path.split('/')
        .map(|part| encode(part))
        .collect::<Vec<_>>()
        .join("/")
}
