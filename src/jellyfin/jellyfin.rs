use super::types::*;
use crate::collection::CollectionRepo;
use crate::database::{model, Repository};
use std::sync::Arc;

#[derive(Clone)]
pub struct JellyfinState {
    pub repo: Arc<dyn Repository>,
    pub collections: Arc<CollectionRepo>,
    pub server_id: String,
    pub server_name: String,
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
