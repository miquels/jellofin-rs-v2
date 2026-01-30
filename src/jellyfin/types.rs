use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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
    #[serde(rename = "PrimaryImageTag")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary_image_tag: Option<String>,
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
    #[serde(rename = "AudioLanguagePreference")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_language_preference: Option<String>,
    #[serde(rename = "RememberAudioSelections")]
    pub remember_audio_selections: bool,
    #[serde(rename = "RememberSubtitleSelections")]
    pub remember_subtitle_selections: bool,
    #[serde(rename = "HidePlayedInLatest")]
    pub hide_played_in_latest: bool,
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
    #[serde(rename = "EnableCollectionManagement")]
    pub enable_collection_management: bool,
    #[serde(rename = "EnableSubtitleManagement")]
    pub enable_subtitle_management: bool,
    #[serde(rename = "EnableLyricManagement")]
    pub enable_lyric_management: bool,
    #[serde(rename = "AllowedTags")]
    pub allowed_tags: Vec<String>,
    #[serde(rename = "EnableUserPreferenceAccess")]
    pub enable_user_preference_access: bool,
    #[serde(rename = "AccessSchedules")]
    pub access_schedules: Vec<serde_json::Value>,
    #[serde(rename = "BlockUnratedItems")]
    pub block_unrated_items: Vec<String>,
    #[serde(rename = "EnableRemoteControlOfOtherUsers")]
    pub enable_remote_control_of_other_users: bool,
    #[serde(rename = "EnableSharedDeviceControl")]
    pub enable_shared_device_control: bool,
    #[serde(rename = "EnableLiveTvManagement")]
    pub enable_live_tv_management: bool,
    #[serde(rename = "EnableLiveTvAccess")]
    pub enable_live_tv_access: bool,
    #[serde(rename = "EnablePlaybackRemuxing")]
    pub enable_playback_remuxing: bool,
    #[serde(rename = "ForceRemoteSourceTranscoding")]
    pub force_remote_source_transcoding: bool,
    #[serde(rename = "EnableContentDeletionFromFolders")]
    pub enable_content_deletion_from_folders: Vec<String>,
    #[serde(rename = "EnableSyncTranscoding")]
    pub enable_sync_transcoding: bool,
    #[serde(rename = "EnableMediaConversion")]
    pub enable_media_conversion: bool,
    #[serde(rename = "EnabledDevices")]
    pub enabled_devices: Vec<String>,
    #[serde(rename = "EnabledChannels")]
    pub enabled_channels: Vec<String>,
    #[serde(rename = "EnableAllChannels")]
    pub enable_all_channels: bool,
    #[serde(rename = "InvalidLoginAttemptCount")]
    pub invalid_login_attempt_count: i32,
    #[serde(rename = "LoginAttemptsBeforeLockout")]
    pub login_attempts_before_lockout: i32,
    #[serde(rename = "MaxActiveSessions")]
    pub max_active_sessions: i32,
    #[serde(rename = "EnablePublicSharing")]
    pub enable_public_sharing: bool,
    #[serde(rename = "BlockedMediaFolders")]
    pub blocked_media_folders: Vec<String>,
    #[serde(rename = "BlockedChannels")]
    pub blocked_channels: Vec<String>,
    #[serde(rename = "RemoteClientBitrateLimit")]
    pub remote_client_bitrate_limit: i32,
    #[serde(rename = "AuthenticationProviderId")]
    pub authentication_provider_id: String,
    #[serde(rename = "PasswordResetProviderId")]
    pub password_reset_provider_id: String,
    #[serde(rename = "SyncPlayAccess")]
    pub sync_play_access: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    #[serde(rename = "PlayState")]
    pub play_state: PlayState,
    #[serde(rename = "AdditionalUsers")]
    pub additional_users: Vec<String>,
    #[serde(rename = "Capabilities")]
    pub capabilities: SessionResponseCapabilities,
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
    #[serde(rename = "HasCustomDeviceName")]
    pub has_custom_device_name: bool,
    #[serde(rename = "NowPlayingQueue")]
    pub now_playing_queue: Vec<serde_json::Value>,
    #[serde(rename = "NowPlayingQueueFullItems")]
    pub now_playing_queue_full_items: Vec<serde_json::Value>,
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
    #[serde(rename = "PlaybackOrder")]
    pub playback_order: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionResponseCapabilities {
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub local_address: Option<String>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SystemInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub local_address: Option<String>,
    pub server_name: String,
    pub version: String,
    pub product_name: String,
    pub operating_system: String,
    pub id: String,
    pub startup_wizard_completed: bool,
    pub has_pending_restart: bool,
    pub is_shutting_down: bool,
    pub supports_library_monitor: bool,
    pub web_socket_port_number: i32,
    pub completed_installations: Vec<serde_json::Value>,
    pub can_self_restart: bool,
    pub can_self_update: bool,
    pub can_launch_web_browser: bool,
    pub program_data_path: String,
    pub items_by_name_path: String,
    pub cache_path: String,
    pub log_path: String,
    pub internal_metadata_path: String,
    pub transcoding_temp_path: String,
    pub has_update_available: bool,
    pub encoder_location: String,
    pub system_architecture: String,
}

pub type SystemInfoResponse = SystemInfo;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct QueryResult<T> {
    pub items: Vec<T>,
    pub total_record_count: i32,
    pub start_index: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SearchHintsResponse {
    pub search_hints: Vec<BaseItemDto>,
    pub total_record_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ItemCountResponse {
    pub movie_count: i32,
    pub series_count: i32,
    pub episode_count: i32,
    pub artist_count: i32,
    pub program_count: i32,
    pub trailer_count: i32,
    pub song_count: i32,
    pub album_count: i32,
    pub music_video_count: i32,
    pub box_set_count: i32,
    pub book_count: i32,
    pub item_count: i32,
}

pub struct PlaybackInfoResponse {
    pub media_sources: Vec<MediaSourceInfo>,
    pub play_session_id: String,
}

pub type UserItemsResponse = QueryResult<BaseItemDto>;
pub type UsersItemsResumeResponse = QueryResult<BaseItemDto>;
pub type UsersItemsSimilarResponse = QueryResult<BaseItemDto>;
pub type UsersItemsSuggestionsResponse = QueryResult<BaseItemDto>;
pub type ShowsNextUpResponse = QueryResult<BaseItemDto>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ItemFilterResponse {
    pub genres: Vec<String>,
    pub tags: Vec<String>,
    pub official_ratings: Vec<String>,
    pub years: Vec<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ItemFilter2Response {
    pub genres: Vec<GenreItem>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct BaseItemDto {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_title: Option<String>,
    pub server_id: String,
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub etag: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_created: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub can_delete: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub can_download: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_subtitles: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub premiere_date: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_urls: Option<Vec<ExternalUrl>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media_sources: Option<Vec<MediaSourceInfo>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub critic_rating: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub production_locations: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub official_rating: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overview: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub short_overview: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub taglines: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub genres: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub community_rating: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_time_ticks: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub production_year: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index_number: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_index_number: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_ids: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_folder: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_hd: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_4k: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    #[serde(rename = "Type")]
    pub item_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub people: Option<Vec<BaseItemPerson>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub studios: Option<Vec<StudioDto>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub genre_items: Option<Vec<GenreItem>>,
    #[serde(rename = "EnableMediaSourceDisplay")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_media_source_display: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_specials_within_seasons: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_program: Option<Box<BaseItemDto>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary_image_aspect_ratio: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artists: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artist_items: Option<Vec<ArtistItem>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub album: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collection_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_order: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub album_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub album_primary_image_tag: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub series_primary_image_tag: Option<String>,
    #[serde(rename = "DisplayPreferencesId")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_preferences_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lock_data: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locked_fields: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub album_artist: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub album_artists: Option<Vec<ArtistItem>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub season_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub season_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub series_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub series_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_tags: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backdrop_image_tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub screenshot_image_tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_blur_hashes: Option<HashMap<String, HashMap<String, String>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media_streams: Option<Vec<MediaStream>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video_3d_format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_data: Option<UserItemDataDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recursive_item_count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub child_count: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub series_timer_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub program_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overview_html: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completion_percentage: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub play_access: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ExternalUrl {
    pub name: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct MediaSourceInfo {
    pub protocol: String,
    pub id: String,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encoder_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encoder_protocol: Option<String>,
    pub r#type: String,
    pub container: String,
    pub size: i64,
    pub name: String,
    pub is_remote: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_time_ticks: Option<i64>,
    pub supports_transcoding: bool,
    pub supports_direct_stream: bool,
    pub supports_direct_play: bool,
    pub is_infinite_stream: bool,
    pub requires_opening: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub open_token: Option<String>,
    pub requires_closing: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub live_stream_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub buffer_ms: Option<i32>,
    pub requires_looping: bool,
    pub supports_external_stream: bool,
    pub media_streams: Vec<MediaStream>,
    pub formats: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bitrate: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required_http_headers: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transcoding_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transcoding_sub_protocol: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transcoding_container: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub analyze_duration_ms: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_audio_stream_index: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_subtitle_stream_index: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct MediaStream {
    pub codec: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_base: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_language: Option<String>,
    pub is_interlaced: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel_layout: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bit_rate: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bit_depth: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ref_frames: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub packet_length: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channels: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sample_rate: Option<i32>,
    pub is_default: bool,
    pub is_forced: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub average_frame_rate: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub real_frame_rate: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
    #[serde(rename = "Type")]
    pub stream_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aspect_ratio: Option<String>,
    pub index: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<i32>,
    pub is_external: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delivery_method: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delivery_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_external_url: Option<bool>,
    pub is_text_subtitle_stream: bool,
    pub supports_external_stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pixel_format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct BaseItemPerson {
    pub name: String,
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    pub r#type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary_image_tag: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct StudioDto {
    pub name: String,
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct GenreItem {
    pub name: String,
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ArtistItem {
    pub name: String,
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct UserItemDataDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rating: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub played_percentage: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unplayed_item_count: Option<i32>,
    pub playback_position_ticks: i64,
    pub play_count: i32,
    pub is_favorite: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub likes: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_played_date: Option<DateTime<Utc>>,
    pub played: bool,
    pub key: String,
    pub item_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct UpdatePlayStateRequest {
    pub can_seek: bool,
    pub repeat_mode: String,
    pub position_ticks: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub play_session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media_source_id: Option<String>,
    pub item_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub play_method: Option<String>,
    pub is_muted: bool,
    pub is_paused: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Language {
    pub display_name: String,
    pub name: String,
    pub three_letter_iso_language_name: String,
    pub three_letter_iso_language_names: Vec<String>,
    pub two_letter_iso_language_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct LocalizationOption {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ParentalRating {
    pub name: String,
    pub value: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct PathInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct TypeOption {
    pub r#type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata_fetchers: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata_fetcher_order: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_fetchers: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_fetcher_order: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_options: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct LibraryOptions {
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_photos: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_realtime_monitor: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_lufs_scan: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_chapter_image_extraction: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extract_chapter_images_during_library_scan: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_trickplay_image_extraction: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extract_trickplay_images_during_library_scan: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path_infos: Option<Vec<PathInfo>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub save_local_metadata: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_internet_providers: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_automatic_series_grouping: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_embedded_titles: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_embedded_extras_titles: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_embedded_episode_infos: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub automatic_refresh_interval_days: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preferred_metadata_language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata_country_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub season_zero_display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata_savers: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disabled_local_metadata_readers: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub local_metadata_reader_order: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disabled_subtitle_fetchers: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtitle_fetcher_order: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skip_subtitles_if_embedded_subtitles_present: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skip_subtitles_if_audio_track_matches: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtitle_download_languages: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub require_perfect_subtitle_match: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub save_subtitles_with_media: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub save_lyrics_with_media: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub automatically_add_to_collection: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_embedded_subtitles: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_options: Option<Vec<TypeOption>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct MediaLibrary {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locations: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collection_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub library_options: Option<LibraryOptions>,
    #[serde(rename = "ItemId")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary_image_item_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct BrandingConfiguration {
    pub login_disclaimer: String,
    pub custom_css: String,
    pub splashscreen_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct DevicesOptionsResponse {
    #[serde(rename = "DeviceId")]
    pub device_id: String,
    pub custom_name: String,
    pub disable_auto_login: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CreatePlaylistRequest {
    pub name: String,
    #[serde(rename = "UserId")]
    pub user_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ids: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CreatePlaylistResponse {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct GetPlaylistResponse {
    pub open_access: bool,
    pub shares: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item_ids: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PlaylistAccess {
    pub users: Vec<String>,
    pub can_edit: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HTTPError {
    pub status: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<HashMap<String, Vec<String>>>,
    #[serde(rename = "traceId")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
}

use std::collections::HashMap;

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
            audio_language_preference: None,
            remember_audio_selections: true,
            remember_subtitle_selections: true,
            hide_played_in_latest: true,
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
            enable_collection_management: false,
            enable_subtitle_management: false,
            enable_lyric_management: false,
            allowed_tags: Vec::new(),
            enable_user_preference_access: false,
            access_schedules: Vec::new(),
            block_unrated_items: Vec::new(),
            enable_remote_control_of_other_users: false,
            enable_shared_device_control: false,
            enable_live_tv_management: false,
            enable_live_tv_access: false,
            enable_playback_remuxing: false,
            force_remote_source_transcoding: false,
            enable_content_deletion_from_folders: Vec::new(),
            enable_sync_transcoding: false,
            enable_media_conversion: false,
            enabled_devices: Vec::new(),
            enabled_channels: Vec::new(),
            enable_all_channels: false,
            invalid_login_attempt_count: 0,
            login_attempts_before_lockout: 0,
            max_active_sessions: 0,
            enable_public_sharing: false,
            blocked_media_folders: Vec::new(),
            blocked_channels: Vec::new(),
            remote_client_bitrate_limit: 0,
            authentication_provider_id: "DefaultAuthenticationProvider".to_string(),
            password_reset_provider_id: "DefaultPasswordResetProvider".to_string(),
            sync_play_access: "CreateAndJoinGroups".to_string(),
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
            playback_order: "Default".to_string(),
        }
    }
}

impl Default for SessionResponseCapabilities {
    fn default() -> Self {
        Self {
            playable_media_types: vec!["Video".to_string(), "Audio".to_string()],
            supported_commands: Vec::new(),
            supports_media_control: false,
            supports_persistent_identifier: true,
        }
    }
}

impl Default for SystemInfo {
    fn default() -> Self {
        Self {
            local_address: None,
            server_name: String::new(),
            version: String::new(),
            product_name: String::new(),
            operating_system: String::new(),
            id: String::new(),
            startup_wizard_completed: true,
            has_pending_restart: false,
            is_shutting_down: false,
            supports_library_monitor: false,
            web_socket_port_number: 0,
            completed_installations: Vec::new(),
            can_self_restart: false,
            can_self_update: false,
            can_launch_web_browser: false,
            program_data_path: String::new(),
            items_by_name_path: String::new(),
            cache_path: String::new(),
            log_path: String::new(),
            internal_metadata_path: String::new(),
            transcoding_temp_path: String::new(),
            has_update_available: false,
            encoder_location: String::new(),
            system_architecture: String::new(),
        }
    }
}

impl Default for BaseItemDto {
    fn default() -> Self {
        Self {
            name: String::new(),
            original_title: None,
            server_id: String::new(),
            id: String::new(),
            etag: None,
            date_created: None,
            can_delete: None,
            can_download: None,
            has_subtitles: None,
            container: None,
            sort_name: None,
            premiere_date: None,
            external_urls: None,
            media_sources: None,
            critic_rating: None,
            production_locations: None,
            path: None,
            official_rating: None,
            overview: None,
            short_overview: None,
            taglines: None,
            genres: None,
            community_rating: None,
            run_time_ticks: None,
            production_year: None,
            index_number: None,
            parent_index_number: None,
            provider_ids: None,
            is_folder: None,
            parent_id: None,
            item_type: "Unknown".to_string(),
            people: None,
            studios: None,
            genre_items: None,
            enable_media_source_display: None,
            display_specials_within_seasons: None,
            current_program: None,
            address: None,
            primary_image_aspect_ratio: None,
            artists: None,
            artist_items: None,
            album: None,
            collection_type: None,
            display_order: None,
            album_id: None,
            album_primary_image_tag: None,
            series_primary_image_tag: None,
            display_preferences_id: None,
            lock_data: None,
            tags: None,
            locked_fields: None,
            album_artist: None,
            album_artists: None,
            season_id: None,
            season_name: None,
            series_id: None,
            series_name: None,
            video_type: None,
            image_tags: None,
            backdrop_image_tags: None,
            screenshot_image_tags: None,
            image_blur_hashes: None,
            location_type: Some("FileSystem".to_string()),
            media_streams: None,
            video_3d_format: None,
            user_data: None,
            recursive_item_count: None,
            child_count: None,
            series_timer_id: None,
            program_id: None,
            channel_id: None,
            channel_name: None,
            overview_html: None,
            completion_percentage: None,
            status: None,
            play_access: None,
            media_type: None,
            is_hd: None,
            is_4k: None,
            width: None,
            height: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct DeviceItem {
    #[serde(rename = "Id")]
    pub id: String,
    pub last_user_id: String,
    pub last_user_name: String,
    pub name: String,
    pub app_name: String,
    pub app_version: String,
    pub capabilities: SessionResponseCapabilities,
    pub date_last_activity: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Country {
    pub name: String,
    pub two_letter_iso_region_name: String,
    pub three_letter_iso_region_name: String,
}
