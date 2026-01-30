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
    pub image_resizer: Arc<crate::imageresize::ImageResizer>,
    pub config: Arc<crate::server::Config>,
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
        policy: UserPolicy {
            is_administrator: true,
            is_hidden: false,
            is_disabled: false,
            enable_remote_access: true,
            enable_media_playback: true,
            enable_audio_playback_transcoding: true,
            enable_video_playback_transcoding: true,
            enable_content_deletion: false,
            enable_content_downloading: true,
            enable_all_devices: true,
            enable_all_folders: true,
            blocked_tags: vec![],
            enabled_folders: vec![],
            enable_collection_management: false,
            enable_subtitle_management: false,
            enable_lyric_management: false,
            allowed_tags: vec![],
            enable_user_preference_access: false,
            access_schedules: vec![],
            block_unrated_items: vec![],
            enable_remote_control_of_other_users: false,
            enable_shared_device_control: false,
            enable_live_tv_management: false,
            enable_live_tv_access: false,
            enable_playback_remuxing: false,
            force_remote_source_transcoding: false,
            enable_content_deletion_from_folders: vec![],
            enable_sync_transcoding: false,
            enable_media_conversion: false,
            enabled_devices: vec![],
            enabled_channels: vec![],
            enable_all_channels: false,
            invalid_login_attempt_count: 0,
            login_attempts_before_lockout: 0,
            max_active_sessions: 0,
            enable_public_sharing: false,
            blocked_media_folders: vec![],
            blocked_channels: vec![],
            remote_client_bitrate_limit: 0,
            authentication_provider_id: "DefaultAuthenticationProvider".to_string(),
            password_reset_provider_id: "DefaultPasswordResetProvider".to_string(),
            sync_play_access: "CreateAndJoinGroups".to_string(),
        },

        primary_image_tag: None,
    }
}
