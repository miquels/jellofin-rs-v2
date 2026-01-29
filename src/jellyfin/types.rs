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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SystemInfo {
    pub local_address: String,
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
    pub original_title: Option<String>,
    pub server_id: String,
    pub id: String,
    pub etag: Option<String>,
    pub date_created: Option<DateTime<Utc>>,
    pub can_delete: Option<bool>,
    pub can_download: Option<bool>,
    pub has_subtitles: Option<bool>,
    pub container: Option<String>,
    pub sort_name: Option<String>,
    pub premiere_date: Option<DateTime<Utc>>,
    pub external_urls: Option<Vec<ExternalUrl>>,
    pub media_sources: Option<Vec<MediaSourceInfo>>,
    pub critic_rating: Option<f32>,
    pub media_type: Option<String>,
    pub production_locations: Option<Vec<String>>,
    pub path: Option<String>,
    pub official_rating: Option<String>,
    pub overview: Option<String>,
    pub short_overview: Option<String>,
    pub taglines: Option<Vec<String>>,
    pub genres: Option<Vec<String>>,
    pub community_rating: Option<f32>,
    pub run_time_ticks: Option<i64>,
    pub production_year: Option<i32>,
    pub index_number: Option<i32>,
    pub parent_index_number: Option<i32>,
    pub provider_ids: Option<HashMap<String, String>>,
    pub is_folder: Option<bool>,
    pub is_hd: Option<bool>,
    pub is_4k: Option<bool>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub parent_id: Option<String>,
    #[serde(rename = "Type")]
    pub item_type: String,
    pub people: Option<Vec<BaseItemPerson>>,
    pub studios: Option<Vec<StudioDto>>,
    pub genre_items: Option<Vec<GenreItem>>,
    pub display_specials_within_seasons: Option<bool>,
    pub current_program: Option<Box<BaseItemDto>>,
    pub address: Option<String>,
    pub primary_image_aspect_ratio: Option<f64>,
    pub artists: Option<Vec<String>>,
    pub artist_items: Option<Vec<ArtistItem>>,
    pub album: Option<String>,
    pub collection_type: Option<String>,
    pub display_order: Option<String>,
    pub album_id: Option<String>,
    pub album_primary_image_tag: Option<String>,
    pub series_primary_image_tag: Option<String>,
    pub album_artist: Option<String>,
    pub album_artists: Option<Vec<ArtistItem>>,
    pub season_id: Option<String>,
    pub season_name: Option<String>,
    pub series_id: Option<String>,
    pub series_name: Option<String>,
    pub video_type: Option<String>,
    pub image_tags: Option<HashMap<String, String>>,
    pub backdrop_image_tags: Option<Vec<String>>,
    pub screenshot_image_tags: Option<Vec<String>>,
    pub image_blur_hashes: Option<HashMap<String, HashMap<String, String>>>,
    pub location_type: Option<String>,
    pub media_streams: Option<Vec<MediaStream>>,
    pub video_3d_format: Option<String>,
    pub user_data: Option<UserItemDataDto>,
    pub recursive_item_count: Option<i64>,
    pub child_count: Option<i32>,
    pub series_timer_id: Option<String>,
    pub program_id: Option<String>,
    pub channel_id: Option<String>,
    pub channel_name: Option<String>,
    pub overview_html: Option<String>,
    pub completion_percentage: Option<f64>,
    pub play_access: Option<String>,
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
    pub encoder_path: Option<String>,
    pub encoder_protocol: Option<String>,
    pub r#type: String,
    pub container: String,
    pub size: i64,
    pub name: String,
    pub is_remote: bool,
    pub run_time_ticks: Option<i64>,
    pub supports_transcoding: bool,
    pub supports_direct_stream: bool,
    pub supports_direct_play: bool,
    pub is_infinite_stream: bool,
    pub requires_opening: bool,
    pub open_token: Option<String>,
    pub requires_closing: bool,
    pub live_stream_id: Option<String>,
    pub buffer_ms: Option<i32>,
    pub requires_looping: bool,
    pub supports_external_stream: bool,
    pub media_streams: Vec<MediaStream>,
    pub formats: Vec<String>,
    pub bitrate: Option<i32>,
    pub timestamp: Option<String>,
    pub required_http_headers: Option<HashMap<String, String>>,
    pub transcoding_url: Option<String>,
    pub transcoding_sub_protocol: Option<String>,
    pub transcoding_container: Option<String>,
    pub analyze_duration_ms: Option<i32>,
    pub default_audio_stream_index: Option<i32>,
    pub default_subtitle_stream_index: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct MediaStream {
    pub codec: String,
    pub language: Option<String>,
    pub time_base: Option<String>,
    pub title: Option<String>,
    pub display_title: Option<String>,
    pub display_language: Option<String>,
    pub is_interlaced: bool,
    pub channel_layout: Option<String>,
    pub bit_rate: Option<i32>,
    pub bit_depth: Option<i32>,
    pub ref_frames: Option<i32>,
    pub packet_length: Option<i32>,
    pub channels: Option<i32>,
    pub sample_rate: Option<i32>,
    pub is_default: bool,
    pub is_forced: bool,
    pub height: Option<i32>,
    pub width: Option<i32>,
    pub average_frame_rate: Option<f32>,
    pub real_frame_rate: Option<f32>,
    pub profile: Option<String>,
    #[serde(rename = "Type")]
    pub stream_type: String,
    pub aspect_ratio: Option<String>,
    pub index: i32,
    pub score: Option<i32>,
    pub is_external: bool,
    pub delivery_method: Option<String>,
    pub delivery_url: Option<String>,
    pub is_external_url: Option<bool>,
    pub is_text_subtitle_stream: bool,
    pub supports_external_stream: bool,
    pub path: Option<String>,
    pub pixel_format: Option<String>,
    pub level: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct BaseItemPerson {
    pub name: String,
    pub id: String,
    pub role: Option<String>,
    pub r#type: String,
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
    pub rating: Option<f64>,
    pub played_percentage: Option<f64>,
    pub unplayed_item_count: Option<i32>,
    pub playback_position_ticks: i64,
    pub play_count: i32,
    pub is_favorite: bool,
    pub likes: Option<bool>,
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
    pub play_session_id: Option<String>,
    pub media_source_id: Option<String>,
    pub item_id: String,
    pub play_method: Option<String>,
    pub is_muted: bool,
    pub is_paused: bool,
    pub event_name: Option<String>,
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

impl Default for SystemInfo {
    fn default() -> Self {
        Self {
            local_address: String::new(),
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
