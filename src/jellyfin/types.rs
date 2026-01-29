use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticateUserByNameRequest {
    #[serde(rename = "Username")]
    pub username: String,
    #[serde(rename = "Pw")]
    pub pw: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticateByNameResponse {
    #[serde(rename = "User")]
    pub user: User,
    #[serde(rename = "SessionInfo")]
    pub session_info: SessionInfo,
    #[serde(rename = "AccessToken")]
    pub access_token: String,
    #[serde(rename = "ServerId")]
    pub server_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "ServerId")]
    pub server_id: String,
    #[serde(rename = "Id")]
    pub id: String,
    #[serde(rename = "HasPassword")]
    pub has_password: bool,
    #[serde(rename = "HasConfiguredPassword")]
    pub has_configured_password: bool,
    #[serde(rename = "HasConfiguredEasyPassword")]
    pub has_configured_easy_password: bool,
    #[serde(rename = "EnableAutoLogin")]
    pub enable_auto_login: bool,
    #[serde(rename = "LastLoginDate")]
    pub last_login_date: DateTime<Utc>,
    #[serde(rename = "LastActivityDate")]
    pub last_activity_date: DateTime<Utc>,
    #[serde(rename = "Configuration")]
    pub configuration: UserConfiguration,
    #[serde(rename = "Policy")]
    pub policy: UserPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserConfiguration {
    #[serde(rename = "GroupedFolders")]
    pub grouped_folders: Vec<String>,
    #[serde(rename = "SubtitleMode")]
    pub subtitle_mode: String,
    #[serde(rename = "OrderedViews")]
    pub ordered_views: Vec<String>,
    #[serde(rename = "MyMediaExcludes")]
    pub my_media_excludes: Vec<String>,
    #[serde(rename = "LatestItemsExcludes")]
    pub latest_items_excludes: Vec<String>,
    #[serde(rename = "SubtitleLanguagePreference")]
    pub subtitle_language_preference: String,
    #[serde(rename = "PlayDefaultAudioTrack")]
    pub play_default_audio_track: bool,
    #[serde(rename = "DisplayMissingEpisodes")]
    pub display_missing_episodes: bool,
    #[serde(rename = "EnableNextEpisodeAutoPlay")]
    pub enable_next_episode_auto_play: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPolicy {
    #[serde(rename = "IsAdministrator")]
    pub is_administrator: bool,
    #[serde(rename = "IsHidden")]
    pub is_hidden: bool,
    #[serde(rename = "IsDisabled")]
    pub is_disabled: bool,
    #[serde(rename = "EnableRemoteAccess")]
    pub enable_remote_access: bool,
    #[serde(rename = "EnableMediaPlayback")]
    pub enable_media_playback: bool,
    #[serde(rename = "EnableAudioPlaybackTranscoding")]
    pub enable_audio_playback_transcoding: bool,
    #[serde(rename = "EnableVideoPlaybackTranscoding")]
    pub enable_video_playback_transcoding: bool,
    #[serde(rename = "EnableContentDeletion")]
    pub enable_content_deletion: bool,
    #[serde(rename = "EnableContentDownloading")]
    pub enable_content_downloading: bool,
    #[serde(rename = "EnableAllDevices")]
    pub enable_all_devices: bool,
    #[serde(rename = "EnableAllFolders")]
    pub enable_all_folders: bool,
    #[serde(rename = "BlockedTags")]
    pub blocked_tags: Vec<String>,
    #[serde(rename = "EnabledFolders")]
    pub enabled_folders: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    #[serde(rename = "PlayState")]
    pub play_state: PlayState,
    #[serde(rename = "AdditionalUsers")]
    pub additional_users: Vec<String>,
    #[serde(rename = "Capabilities")]
    pub capabilities: Capabilities,
    #[serde(rename = "RemoteEndPoint")]
    pub remote_end_point: String,
    #[serde(rename = "PlayableMediaTypes")]
    pub playable_media_types: Vec<String>,
    #[serde(rename = "Id")]
    pub id: String,
    #[serde(rename = "UserId")]
    pub user_id: String,
    #[serde(rename = "UserName")]
    pub user_name: String,
    #[serde(rename = "Client")]
    pub client: String,
    #[serde(rename = "LastActivityDate")]
    pub last_activity_date: DateTime<Utc>,
    #[serde(rename = "DeviceName")]
    pub device_name: String,
    #[serde(rename = "DeviceId")]
    pub device_id: String,
    #[serde(rename = "ApplicationVersion")]
    pub application_version: String,
    #[serde(rename = "IsActive")]
    pub is_active: bool,
    #[serde(rename = "SupportsMediaControl")]
    pub supports_media_control: bool,
    #[serde(rename = "SupportsRemoteControl")]
    pub supports_remote_control: bool,
    #[serde(rename = "ServerId")]
    pub server_id: String,
    #[serde(rename = "SupportedCommands")]
    pub supported_commands: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayState {
    #[serde(rename = "CanSeek")]
    pub can_seek: bool,
    #[serde(rename = "IsPaused")]
    pub is_paused: bool,
    #[serde(rename = "IsMuted")]
    pub is_muted: bool,
    #[serde(rename = "RepeatMode")]
    pub repeat_mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capabilities {
    #[serde(rename = "PlayableMediaTypes")]
    pub playable_media_types: Vec<String>,
    #[serde(rename = "SupportedCommands")]
    pub supported_commands: Vec<String>,
    #[serde(rename = "SupportsMediaControl")]
    pub supports_media_control: bool,
    #[serde(rename = "SupportsPersistentIdentifier")]
    pub supports_persistent_identifier: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfoPublicResponse {
    #[serde(rename = "LocalAddress")]
    pub local_address: String,
    #[serde(rename = "ServerName")]
    pub server_name: String,
    #[serde(rename = "Version")]
    pub version: String,
    #[serde(rename = "ProductName")]
    pub product_name: String,
    #[serde(rename = "OperatingSystem")]
    pub operating_system: String,
    #[serde(rename = "Id")]
    pub id: String,
    #[serde(rename = "StartupWizardCompleted")]
    pub startup_wizard_completed: bool,
}

impl Default for UserConfiguration {
    fn default() -> Self {
        Self {
            grouped_folders: Vec::new(),
            subtitle_mode: "Default".to_string(),
            ordered_views: Vec::new(),
            my_media_excludes: Vec::new(),
            latest_items_excludes: Vec::new(),
            subtitle_language_preference: String::new(),
            play_default_audio_track: true,
            display_missing_episodes: false,
            enable_next_episode_auto_play: true,
        }
    }
}

impl Default for UserPolicy {
    fn default() -> Self {
        Self {
            is_administrator: false,
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
            blocked_tags: Vec::new(),
            enabled_folders: Vec::new(),
        }
    }
}

impl Default for PlayState {
    fn default() -> Self {
        Self {
            can_seek: true,
            is_paused: false,
            is_muted: false,
            repeat_mode: "RepeatNone".to_string(),
        }
    }
}

impl Default for Capabilities {
    fn default() -> Self {
        Self {
            playable_media_types: vec!["Video".to_string(), "Audio".to_string()],
            supported_commands: Vec::new(),
            supports_media_control: false,
            supports_persistent_identifier: true,
        }
    }
}
