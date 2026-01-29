use axum::{
    extract::{Path as AxumPath, State},
    http::StatusCode,
    response::Json,
    Extension,
};
use std::sync::Arc;

use crate::collection::CollectionRepo;
use crate::database::{model, Repository};

use super::types::*;

#[derive(Clone)]
pub struct JellyfinState {
    pub repo: Arc<dyn Repository>,
    pub collections: Arc<CollectionRepo>,
    pub server_id: String,
    pub server_name: String,
}

/// GET /Users - Get all users (returns current user only)
pub async fn users_all(
    Extension(token): Extension<model::AccessToken>,
    State(state): State<JellyfinState>,
) -> Result<Json<Vec<User>>, StatusCode> {
    let user = state.repo.get_user_by_id(&token.user_id).await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    
    Ok(Json(vec![make_user(&user, &state.server_id)]))
}

/// GET /Users/Me - Get current user
pub async fn users_me(
    Extension(token): Extension<model::AccessToken>,
    State(state): State<JellyfinState>,
) -> Result<Json<User>, StatusCode> {
    let user = state.repo.get_user_by_id(&token.user_id).await
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
    
    let user = state.repo.get_user_by_id(&token.user_id).await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    
    Ok(Json(make_user(&user, &state.server_id)))
}

/// GET /Users/Public - Get public users (returns empty list)
pub async fn users_public() -> Json<Vec<User>> {
    Json(Vec::new())
}

/// GET /System/Ping - Ping endpoint
pub async fn system_ping() -> &'static str {
    "\"Jellyfin Server\""
}

/// GET /health - Health check
pub async fn health() -> &'static str {
    "Healthy"
}

/// GET /Plugins - Get plugins (returns empty list)
pub async fn plugins() -> Json<Vec<serde_json::Value>> {
    Json(Vec::new())
}

/// GET /Branding/Configuration - Get branding configuration
pub async fn branding_configuration() -> Json<BrandingConfiguration> {
    Json(BrandingConfiguration {
        login_disclaimer: String::new(),
        custom_css: String::new(),
        splashscreen_enabled: false,
    })
}

/// Helper: Make User from database model
fn make_user(user: &model::User, server_id: &str) -> User {
    User {
        name: user.username.clone(),
        server_id: server_id.to_string(),
        id: user.id.clone(),
        has_password: true,
        has_configured_password: true,
        has_configured_easy_password: false,
        enable_auto_login: false,
        last_login_date: user.last_login,
        last_activity_date: user.last_used,
        configuration: UserConfiguration::default(),
        policy: UserPolicy::default(),
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BrandingConfiguration {
    #[serde(rename = "LoginDisclaimer")]
    pub login_disclaimer: String,
    #[serde(rename = "CustomCss")]
    pub custom_css: String,
    #[serde(rename = "SplashscreenEnabled")]
    pub splashscreen_enabled: bool,
}
