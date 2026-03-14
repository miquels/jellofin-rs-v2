use super::error::apierror;
use super::jellyfin::JellyfinState;
use super::jfitem::make_jfitem;
use super::types::*;
use crate::database::model::{AccessToken, Playlist};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Extension, Json,
};
use serde::Deserialize;
use tracing::error;

#[derive(Deserialize, Debug)]
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
    println!("XXX create_playlist: query: {:?}", query);
    println!("XXX create_playlist: body: {:?}", body);

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
                ids.push(id.to_string());
            }
        }
    }

    // Check if a playlist with this name already exists for the user
    if let Ok(existing) = state.repo.get_playlist_by_name(&user_id, &name).await {
        return (
            StatusCode::OK,
            Json(CreatePlaylistResponse {
                id: existing.id.clone(),
            }),
        )
            .into_response();
    }

    let now = chrono::Utc::now();
    let new_playlist = Playlist {
        id: String::new(), // Repo will generate
        user_id: user_id.clone(),
        name: name.clone(),
        item_ids: ids,
        created: now,
        last_updated: now,
    };

    match state.repo.create_playlist(&new_playlist).await {
        Ok(id) => (StatusCode::CREATED, Json(CreatePlaylistResponse { id })).into_response(),
        Err(e) => {
            error!("Failed to create playlist '{}' for user '{}': {}", name, user_id, e);
            (StatusCode::OK, Json(CreatePlaylistResponse { id: String::new() })).into_response()
        }
    }
}

/// GET /Playlists/:playlist_id
pub async fn get_playlist(
    Extension(token): Extension<AccessToken>,
    Path(playlist_id): Path<String>,
    State(state): State<JellyfinState>,
) -> impl IntoResponse {
    match state.repo.get_playlist(&token.user_id, &playlist_id).await {
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
    let playlist = match state.repo.get_playlist(&token.user_id, &playlist_id).await {
        Ok(p) => p,
        Err(_) => return apierror(StatusCode::NOT_FOUND, "Playlist not found").into_response(),
    };

    let mut items = Vec::new();

    for item_id in playlist.item_ids {
        if let Some((collection, item)) = state.collections.get_item_by_id(&item_id) {
            if let Ok(jfitem) = make_jfitem(&state, &token.user_id, &item, &collection.id).await {
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
    let ids_str = match query.ids {
        Some(s) => s,
        None => return apierror(StatusCode::BAD_REQUEST, "Ids parameter required").into_response(),
    };

    let item_ids: Vec<_> = ids_str.split(',').map(|s| s.to_string()).collect();

    if let Err(_) = state
        .repo
        .add_items_to_playlist(&token.user_id, &playlist_id, &item_ids)
        .await
    {
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
