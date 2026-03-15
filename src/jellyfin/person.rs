use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    Extension,
};
use urlencoding::decode;

use std::collections::HashMap;

use super::jellyfin::JellyfinState;
use super::types::*;
use crate::database::model::AccessToken;
use crate::idhash::*;

/// GET /Persons
pub async fn persons_all(
    Extension(_token): Extension<AccessToken>,
    State(_state): State<JellyfinState>,
) -> Json<UserItemsResponse> {
    // Go implementation returns empty list
    Json(UserItemsResponse {
        items: Vec::new(),
        total_record_count: 0,
        start_index: 0,
    })
}

/// GET /Persons/{name}
pub async fn person_details(
    Extension(token): Extension<AccessToken>,
    State(state): State<JellyfinState>,
    Path(name): Path<String>,
) -> Result<Json<BaseItemDto>, StatusCode> {
    let decoded_name = decode(&name).map_err(|_| StatusCode::BAD_REQUEST)?;

    let db_person = state
        .repo
        .get_person(&decoded_name, &token.user_id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    Ok(Json(make_jf_item_person(&db_person, &state.server_id)))
}

fn make_jf_item_person(person: &crate::database::model::Person, server_id: &str) -> BaseItemDto {
    let person_id = id_hash_prefix(ITEM_PREFIX_PERSON, &person.name);
    let mut dto = BaseItemDto {
        id: person_id.clone(),
        name: person.name.clone(),
        server_id: server_id.to_string(),
        item_type: "Person".to_string(),
        etag: Some(person_id.clone()),
        overview: Some(person.bio.clone()),
        date_created: Some(person.date_of_birth),
        premiere_date: Some(person.date_of_birth),
        location_type: Some("FileSystem".to_string()),
        media_type: Some("Unknown".to_string()),
        play_access: Some("Full".to_string()),
        ..BaseItemDto::default()
    };

    if !person.place_of_birth.is_empty() {
        dto.production_locations = vec![person.place_of_birth.clone()];
    }

    if !person.poster_url.is_empty() {
        let mut image_tags = HashMap::new();
        image_tags.insert("Primary".to_string(), person.poster_url.clone());
        dto.image_tags = image_tags;
    }

    dto.user_data = Some(UserItemDataDto {
        key: format!("Person-{}", person.name),
        item_id: person_id,
        ..UserItemDataDto::default()
    });

    dto
}
