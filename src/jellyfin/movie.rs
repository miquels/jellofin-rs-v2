use super::types::*;
use axum::response::Json;

/// GET /Movies/Recommendations - Get movie recommendations (returns empty list)
pub async fn movies_recommendations() -> Json<Vec<BaseItemDto>> {
    Json(Vec::new())
}
