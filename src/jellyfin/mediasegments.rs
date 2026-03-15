use axum::{response::IntoResponse, Json};

use super::types::*;

pub async fn media_segments_handler() -> impl IntoResponse {
    let response = UserItemsResponse {
        items: vec![],
        total_record_count: 0,
        start_index: 0,
    };
    Json(response)
}
