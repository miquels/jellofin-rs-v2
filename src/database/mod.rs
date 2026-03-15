pub mod model;
pub mod sqlite;

pub use model::{
    AccessToken, DatabaseError, ImageMetadata, Item, Playlist, QuickConnectCode, Result, User, UserData, UserProperties,
};
pub use sqlite::SqliteRepository;

use async_trait::async_trait;

/// PersonRepo defines person DB operations
#[async_trait]
pub trait PersonRepo {
    /// GetPerson retrieves a person by name.
    async fn get_person(&self, name: &str, user_id: &str) -> Result<model::Person>;
}

/// Database repo aggregates the repo interfaces.
#[async_trait]
pub trait Repository:
    UserRepo
    + AccessTokenRepo
    + ItemRepo
    + UserDataRepo
    + PlaylistRepo
    + PersonRepo
    + QuickConnectRepo
    + ImageRepo
    + Send
    + Sync
{
    /// Start background jobs for the repository.
    fn start_background_jobs(&self);
}

/// UserRepo defines the interface for user database operations
#[async_trait]
pub trait UserRepo {
    /// GetUser retrieves a user.
    async fn get_user(&self, username: &str) -> Result<User>;
    /// GetByID retrieves a user from the database by ID.
    async fn get_user_by_id(&self, user_id: &str) -> Result<User>;
    /// GetAllUsers retrieves all users.
    async fn get_all_users(&self) -> Result<Vec<User>>;
    /// UpsertUser upserts a user into the database.
    async fn upsert_user(&self, user: &User) -> Result<()>;
    /// DeleteUser deletes a user from the database.
    async fn delete_user(&self, user_id: &str) -> Result<()>;
}

/// AccessTokenRepo defines access token operations
#[async_trait]
pub trait AccessTokenRepo {
    /// Get accesstoken details by tokenid.
    async fn get_access_token(&self, token: &str) -> Result<AccessToken>;
    /// Get all access tokens for a user.
    async fn get_access_tokens(&self, user_id: &str) -> Result<Vec<AccessToken>>;
    /// Get access token by device ID.
    async fn get_access_token_by_device_id(&self, device_id: &str) -> Result<AccessToken>;
    /// UpsertAccessToken upserts an access token.
    async fn upsert_access_token(&self, token: &AccessToken) -> Result<()>;
    /// DeleteAccessToken deletes an access token.
    async fn delete_access_token(&self, token: &str) -> Result<()>;
}

/// QuickConnectRepo defines quick connect code operations
#[async_trait]
pub trait QuickConnectRepo {
    async fn get_quick_connect_by_secret(&self, secret: &str) -> Result<QuickConnectCode>;
    async fn get_quick_connect_by_code(&self, code: &str) -> Result<QuickConnectCode>;
    async fn upsert_quick_connect(&self, qc: &QuickConnectCode) -> Result<()>;
    async fn delete_expired_quick_connects(&self, before: chrono::DateTime<chrono::Utc>) -> Result<()>;
}

/// ImageRepo defines image storage operations
#[async_trait]
pub trait ImageRepo {
    async fn has_image(&self, item_id: &str, image_type: &str) -> Result<Option<ImageMetadata>>;
    async fn get_image(&self, item_id: &str, image_type: &str) -> Result<(ImageMetadata, Vec<u8>)>;
    async fn store_image(&self, item_id: &str, image_type: &str, meta: &ImageMetadata, data: &[u8]) -> Result<()>;
    async fn delete_image(&self, item_id: &str, image_type: &str) -> Result<()>;
}

/// ItemRepo defines item operations
#[async_trait]
pub trait ItemRepo {
    /// Load item from database.
    async fn db_load_item(&self, item: &mut Item) -> Result<()>;
}

/// UserDataRepo defines play-state operations
#[async_trait]
pub trait UserDataRepo {
    /// Get the play state details for an item per user.
    async fn get_user_data(&self, user_id: &str, item_id: &str) -> Result<UserData>;
    /// Get all favorite items of a user.
    async fn get_favorites(&self, user_id: &str) -> Result<Vec<String>>;
    /// GetRecentlyWatched returns up to `count` most recently watched items.
    async fn get_recently_watched(
        &self,
        user_id: &str,
        include_fully_watched: bool,
        count: usize,
    ) -> Result<Vec<String>>;
    /// Update stores the play state details for a user and item.
    async fn update_user_data(&self, user_id: &str, item_id: &str, details: &UserData) -> Result<()>;
}

/// PlaylistRepo defines playlist DB operations
#[async_trait]
pub trait PlaylistRepo {
    /// Create a new playlist.
    async fn create_playlist(&self, playlist: &Playlist) -> Result<String>;
    /// Get all playlists for a user.
    async fn get_playlists(&self, user_id: &str) -> Result<Vec<String>>;
    /// Get a specific playlist.
    async fn get_playlist(&self, user_id: &str, playlist_id: &str) -> Result<Playlist>;
    /// Get a playlist by name for a user.
    async fn get_playlist_by_name(&self, user_id: &str, name: &str) -> Result<Playlist>;
    /// Add items to a playlist.
    async fn add_items_to_playlist(&self, user_id: &str, playlist_id: &str, item_ids: &[String]) -> Result<()>;
    /// Delete items from a playlist.
    async fn delete_items_from_playlist(&self, playlist_id: &str, item_ids: &[String]) -> Result<()>;
    /// Move a playlist item to a new index.
    async fn move_playlist_item(&self, playlist_id: &str, item_id: &str, new_index: i32) -> Result<()>;
}

/// Create a new database repository based on the type and config provided.
pub async fn new_repository(db_type: &str, config: &crate::server::Config) -> Result<Box<dyn Repository>> {
    match db_type {
        "sqlite" => {
            let filename = format!("{}/tink-items.db", config.dbdir);

            let repo = SqliteRepository::new(&filename).await?;
            Ok(Box::new(repo))
        }
        _ => Err(DatabaseError::NoConfiguration),
    }
}
