use super::error::apierror;
use super::item::make_jf_item;
use super::jellyfin::JellyfinState;
use super::types::*;
use crate::database::model::{AccessToken, Playlist};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Extension, Json,
};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct CreatePlaylistQuery {
    pub name: Option<String>,
    #[serde(rename = "userId")]
    pub user_id: Option<String>,
    pub ids: Option<String>,
}

/// POST /Playlists
pub async fn create_playlist(
    Extension(_token): Extension<AccessToken>,
    Query(query): Query<CreatePlaylistQuery>,
    State(state): State<JellyfinState>,
    Json(body): Json<Option<CreatePlaylistRequest>>,
) -> impl IntoResponse {
    let mut name = query.name.unwrap_or_default();
    let mut user_id = query.user_id.unwrap_or_default();
    let mut ids = Vec::new();

    if let Some(b) = body {
        if !b.name.is_empty() {
            name = b.name;
        }
        if !b.user_id.is_empty() {
            user_id = b.user_id;
        }
        if let Some(i) = b.ids {
            ids = i;
        }
    }

    if name.is_empty() || user_id.is_empty() {
        return apierror(StatusCode::BAD_REQUEST, "Name and UserId are required").into_response();
    }

    if ids.is_empty() {
        if let Some(query_ids) = query.ids {
            for id in query_ids.split(',') {
                ids.push(trim_prefix(id).to_string());
            }
        }
    }

    let new_playlist = Playlist {
        id: String::new(), // Repo will generate
        user_id: user_id.clone(),
        name: name.clone(),
        item_ids: ids,
    };

    match state.repo.create_playlist(&new_playlist).await {
        Ok(id) => (
            StatusCode::CREATED,
            Json(CreatePlaylistResponse {
                id: format!("playlist_{}", id),
            }),
        )
            .into_response(),
        Err(_) => apierror(StatusCode::INTERNAL_SERVER_ERROR, "Failed to create playlist").into_response(),
    }
}

/// GET /Playlists/:playlist_id
pub async fn get_playlist(
    Extension(token): Extension<AccessToken>,
    Path(playlist_id): Path<String>,
    State(state): State<JellyfinState>,
) -> impl IntoResponse {
    let id = trim_prefix(&playlist_id);
    match state.repo.get_playlist(&token.user_id, id).await {
        Ok(p) => Json(GetPlaylistResponse {
            open_access: false,
            shares: Vec::new(),
            item_ids: Some(p.item_ids),
        })
        .into_response(),
        Err(_) => apierror(StatusCode::NOT_FOUND, "Playlist not found").into_response(),
    }
}

/// POST /Playlists/:playlist_id
pub async fn update_playlist() -> impl IntoResponse {
    StatusCode::NO_CONTENT
}

/// GET /Playlists/:playlist_id/Items
pub async fn get_playlist_items(
    Extension(token): Extension<AccessToken>,
    Path(playlist_id): Path<String>,
    State(state): State<JellyfinState>,
) -> impl IntoResponse {
    let id = trim_prefix(&playlist_id);
    let playlist = match state.repo.get_playlist(&token.user_id, id).await {
        Ok(p) => p,
        Err(_) => return apierror(StatusCode::NOT_FOUND, "Playlist not found").into_response(),
    };

    let mut items = Vec::new();

    for item_id in playlist.item_ids {
        if let Some((_collection, item)) = state.collections.get_item_by_id(&item_id) {
            if let Ok(jfitem) = make_jf_item(&state, &token.user_id, &item).await {
                items.push(jfitem);
            }
        }
    }

    Json(UserItemsResponse {
        items: items.clone(),
        total_record_count: items.len() as i32,
        start_index: 0,
    })
    .into_response()
}

/// POST /Playlists/:playlist_id/Items
/// POST /Playlists/:playlist_id/Items/
pub async fn add_playlist_items(
    Extension(token): Extension<AccessToken>,
    Path(playlist_id): Path<String>,
    Query(query): Query<PlaylistIdQuery>,
    State(state): State<JellyfinState>,
) -> impl IntoResponse {
    let id = trim_prefix(&playlist_id);
    let ids_str = match query.ids {
        Some(s) => s,
        None => return apierror(StatusCode::BAD_REQUEST, "Ids parameter required").into_response(),
    };

    let mut item_ids: Vec<String> = Vec::new();
    for i in ids_str.split(',') {
        item_ids.push(trim_prefix(i).to_string());
    }

    if let Err(_) = state.repo.add_items_to_playlist(&token.user_id, id, &item_ids).await {
        return apierror(StatusCode::INTERNAL_SERVER_ERROR, "Failed to add items").into_response();
    }

    StatusCode::NO_CONTENT.into_response()
}

/// GET /Playlists/:playlist_id/Items/:item_id/Move/:new_index
pub async fn move_playlist_item() -> impl IntoResponse {
    StatusCode::NO_CONTENT
}

/// DELETE /Playlists/:playlist_id/Items
pub async fn delete_playlist_items() -> impl IntoResponse {
    StatusCode::NO_CONTENT
}

/// GET /Playlists/:playlist_id/Users
pub async fn get_playlist_all_users(Extension(token): Extension<AccessToken>) -> Json<Vec<PlaylistAccess>> {
    Json(vec![PlaylistAccess {
        users: vec![token.user_id.clone()],
        can_edit: true,
    }])
}

/// GET /Playlists/:playlist_id/Users/:user
pub async fn get_playlist_users(Extension(token): Extension<AccessToken>) -> Json<PlaylistAccess> {
    Json(PlaylistAccess {
        users: vec![token.user_id.clone()],
        can_edit: true,
    })
}

fn trim_prefix(id: &str) -> &str {
    if id.starts_with("playlist_") {
        &id["playlist_".len()..]
    } else {
        id
    }
}

pub struct PlaylistIdQuery {
    pub ids: Option<String>,
}

impl<'de> Deserialize<'de> for PlaylistIdQuery {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct RawIdQuery {
            #[serde(rename = "Ids")]
            ids: Option<String>,
        }
        let raw = RawIdQuery::deserialize(deserializer)?;
        Ok(PlaylistIdQuery { ids: raw.ids })
    }
}
