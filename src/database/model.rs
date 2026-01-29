use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// User represents a user in the system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    /// ID is the unique identifier for the user.
    pub id: String,
    /// Username is the username of the user.
    pub username: String,
    /// Password is the hashed password of the user.
    pub password: String,
    /// Created is the time the user was created.
    pub created: DateTime<Utc>,
    /// LastLogin is the last time the user logged in.
    pub last_login: DateTime<Utc>,
    /// LastUsed is the last time the user was active.
    pub last_used: DateTime<Utc>,
}

/// AccessToken represents an access token for a user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessToken {
    /// UserID is the ID of the user associated with the token.
    pub user_id: String,
    /// Token is the access token string.
    pub token: String,
    /// DeviceId is the unique identifier for the device.
    pub device_id: String,
    /// DeviceName is the name of the device.
    pub device_name: String,
    /// ApplicationName is the name of the application.
    pub application_name: String,
    /// ApplicationVersion is the version of the application.
    pub application_version: String,
    /// RemoteAddress is the remote address of the client.
    pub remote_address: String,
    /// Created is the time the token was created.
    pub created: DateTime<Utc>,
    /// LastUsed is the last time the token was used.
    pub last_used: DateTime<Utc>,
}

/// Item represents a media item in the database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub id: String,
    pub name: String,
    pub votes: i32,
    pub genre: String,
    pub rating: f32,
    pub year: i32,
    pub nfo_time: i64,
    pub first_video: i64,
    pub last_video: i64,
}

/// UserData is the structure for storing user play state data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserData {
    /// Offset in seconds
    pub position: i64,
    /// Played playedPercentage
    pub played_percentage: i32,
    /// Play count of the item
    pub play_count: i32,
    /// True if the item has been fully played
    pub played: bool,
    /// True if the item is favorite of user
    pub favorite: bool,
    /// Timestamp of item playing
    pub timestamp: DateTime<Utc>,
}

/// Playlist represents a user playlist with item IDs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Playlist {
    /// ID is the unique identifier for the playlist.
    pub id: String,
    /// UserID is the identifier of the user who owns the playlist.
    pub user_id: String,
    /// Name of the playlist.
    pub name: String,
    /// ItemIDs is a list of item IDs contained in the playlist.
    pub item_ids: Vec<String>,
}

/// Database errors
#[derive(Debug, thiserror::Error)]
pub enum DatabaseError {
    #[error("Database directory not set")]
    NoConfiguration,
    #[error("Database connection not available")]
    NoDbHandle,
    #[error("Not found")]
    NotFound,
    #[error("Invalid password")]
    InvalidPassword,
    #[error("Database error: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, DatabaseError>;
