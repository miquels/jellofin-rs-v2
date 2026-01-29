use axum::response::Json;
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
pub fn make_user(user: &model::User, server_id: &str) -> User {
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
