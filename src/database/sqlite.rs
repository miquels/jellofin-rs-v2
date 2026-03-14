use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Mutex;

use async_trait::async_trait;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};

use crate::idhash::*;
use super::model::{AccessToken, DatabaseError, ImageMetadata, Item, Person, Playlist, QuickConnectCode, Result, User, UserData, UserProperties};
use super::{AccessTokenRepo, ImageRepo, ItemRepo, PersonRepo, PlaylistRepo, QuickConnectRepo, Repository, UserDataRepo, UserRepo};

/// SQLite database repository implementation
pub struct SqliteRepository {
    /// Read pool
    pool: SqlitePool,
    /// In-memory access token cache, synced to DB periodically
    access_token_cache: Arc<Mutex<HashMap<String, AccessToken>>>,
    /// In-memory user data cache, synced to DB periodically
    user_data_cache: Arc<Mutex<HashMap<(String, String), UserData>>>,
}

impl SqliteRepository {
    /// Create a new SQLite repository
    pub async fn new(filename: &str) -> Result<Self> {
        // Create connection options
        let options = SqliteConnectOptions::from_str(filename)?.create_if_missing(true);

        // Create connection pool
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await?;

        // Initialize schema
        Self::init_schema(&pool).await?;

        let repo = Self {
            pool,
            access_token_cache: Arc::new(Mutex::new(HashMap::new())),
            user_data_cache: Arc::new(Mutex::new(HashMap::new())),
        };

        // Load user data from database into cache
        repo.load_user_data_from_db().await?;

        Ok(repo)
    }

    /// Initialize database schema
    async fn init_schema(pool: &SqlitePool) -> Result<()> {
        sqlx::query("PRAGMA journal_mode = WAL").execute(pool).await?;
        sqlx::query("PRAGMA foreign_keys = ON").execute(pool).await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS users (
                id TEXT PRIMARY KEY,
                username TEXT UNIQUE NOT NULL,
                password TEXT NOT NULL,
                created INTEGER NOT NULL,
                last_login INTEGER NOT NULL,
                last_used INTEGER NOT NULL
            )
            "#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS access_tokens (
                token TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                device_id TEXT NOT NULL,
                device_name TEXT NOT NULL,
                application_name TEXT NOT NULL,
                application_version TEXT NOT NULL,
                remote_address TEXT NOT NULL,
                created INTEGER NOT NULL,
                last_used INTEGER NOT NULL,
                FOREIGN KEY (user_id) REFERENCES users(id)
            )
            "#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS items (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                votes INTEGER NOT NULL DEFAULT 0,
                genre TEXT NOT NULL DEFAULT '',
                rating REAL NOT NULL DEFAULT 0.0,
                year INTEGER NOT NULL DEFAULT 0,
                nfo_time INTEGER NOT NULL DEFAULT 0,
                first_video INTEGER NOT NULL DEFAULT 0,
                last_video INTEGER NOT NULL DEFAULT 0
            )
            "#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS user_data (
                user_id TEXT NOT NULL,
                item_id TEXT NOT NULL,
                position INTEGER NOT NULL DEFAULT 0,
                played_percentage INTEGER NOT NULL DEFAULT 0,
                play_count INTEGER NOT NULL DEFAULT 0,
                played INTEGER NOT NULL DEFAULT 0,
                favorite INTEGER NOT NULL DEFAULT 0,
                timestamp INTEGER NOT NULL,
                PRIMARY KEY (user_id, item_id)
            )
            "#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS playlists (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                name TEXT NOT NULL,
                item_ids TEXT NOT NULL,
                created TEXT NOT NULL DEFAULT (datetime('now')),
                last_updated TEXT NOT NULL DEFAULT (datetime('now')),
                FOREIGN KEY (user_id) REFERENCES users(id)
            )
            "#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS persons (
                id TEXT PRIMARY KEY,
                name TEXT UNIQUE NOT NULL,
                date_of_birth INTEGER NOT NULL,
                place_of_birth TEXT NOT NULL DEFAULT '',
                poster_url TEXT NOT NULL DEFAULT '',
                bio TEXT NOT NULL DEFAULT '',
                created INTEGER NOT NULL,
                last_updated INTEGER NOT NULL
            )
            "#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS user_properties (
                userid TEXT NOT NULL,
                key TEXT NOT NULL,
                value TEXT NOT NULL DEFAULT '',
                PRIMARY KEY (userid, key),
                FOREIGN KEY (userid) REFERENCES users(id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS quickconnect (
                userid TEXT NOT NULL DEFAULT '',
                deviceid TEXT NOT NULL,
                secret TEXT NOT NULL,
                authorized INTEGER NOT NULL DEFAULT 0,
                code TEXT NOT NULL,
                created INTEGER NOT NULL,
                PRIMARY KEY (deviceid, secret)
            )
            "#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS images (
                itemid TEXT NOT NULL,
                type TEXT NOT NULL,
                mimetype TEXT NOT NULL,
                etag TEXT NOT NULL,
                updated INTEGER NOT NULL,
                filesize INTEGER NOT NULL,
                data BLOB NOT NULL,
                PRIMARY KEY (itemid, type)
            )
            "#,
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Load user data from database into cache
    async fn load_user_data_from_db(&self) -> Result<()> {
        let rows = sqlx::query_as::<_, (String, String, i64, i32, i32, bool, bool, i64)>(
            "SELECT user_id, item_id, position, played_percentage, play_count, played, favorite, timestamp FROM user_data"
        )
        .fetch_all(&self.pool)
        .await?;

        tracing::info!("Loaded {} user_data rows from database", rows.len());

        let mut cache = self.user_data_cache.lock().await;
        for row in rows {
            let key = (row.0.clone(), row.1.clone());
            cache.insert(
                key,
                UserData {
                    position: row.2,
                    played_percentage: row.3,
                    play_count: row.4,
                    played: row.5,
                    favorite: row.6,
                    timestamp: chrono::DateTime::from_timestamp(row.7, 0).unwrap_or_default(),
                },
            );
        }

        Ok(())
    }
}

#[async_trait]
impl Repository for SqliteRepository {
    fn start_background_jobs(&self) {
        // No-op: all writes are now write-through (DB first, then cache).
    }
}

impl SqliteRepository {
    /// Load user properties from the user_properties key-value table.
    async fn load_user_properties(&self, user_id: &str) -> Result<UserProperties> {
        let rows = sqlx::query_as::<_, (String, String)>(
            "SELECT key, value FROM user_properties WHERE userid = ?",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        let mut props = UserProperties::default();
        for (key, value) in rows {
            match key.as_str() {
                "admin" => props.admin = value == "1",
                "disabled" => props.disabled = value == "1",
                "is_hidden" => props.is_hidden = value == "1",
                "enable_downloads" => props.enable_downloads = value == "1",
                "enable_all_folders" => props.enable_all_folders = value == "1",
                "enabled_folders" => props.enabled_folders = split_comma(&value),
                "ordered_views" => props.ordered_views = split_comma(&value),
                "my_media_excludes" => props.my_media_excludes = split_comma(&value),
                "allow_tags" => props.allow_tags = split_comma(&value),
                "block_tags" => props.block_tags = split_comma(&value),
                _ => {}
            }
        }
        Ok(props)
    }

    /// Save user properties to the user_properties key-value table.
    async fn save_user_properties(&self, user_id: &str, props: &UserProperties) -> Result<()> {
        let kvs: &[(&str, String)] = &[
            ("admin", bool_to_string(props.admin)),
            ("disabled", bool_to_string(props.disabled)),
            ("is_hidden", bool_to_string(props.is_hidden)),
            ("enable_downloads", bool_to_string(props.enable_downloads)),
            ("enable_all_folders", bool_to_string(props.enable_all_folders)),
            ("enabled_folders", props.enabled_folders.join(",")),
            ("ordered_views", props.ordered_views.join(",")),
            ("my_media_excludes", props.my_media_excludes.join(",")),
            ("allow_tags", props.allow_tags.join(",")),
            ("block_tags", props.block_tags.join(",")),
        ];
        for (key, value) in kvs {
            sqlx::query("INSERT OR REPLACE INTO user_properties (userid, key, value) VALUES (?, ?, ?)")
                .bind(user_id)
                .bind(key)
                .bind(value)
                .execute(&self.pool)
                .await?;
        }
        Ok(())
    }
}

fn split_comma(s: &str) -> Vec<String> {
    if s.is_empty() {
        Vec::new()
    } else {
        s.split(',').map(|p| p.to_string()).collect()
    }
}

fn bool_to_string(b: bool) -> String {
    if b { "1" } else { "0" }.to_string()
}

#[async_trait]
impl UserRepo for SqliteRepository {
    async fn get_user(&self, username: &str) -> Result<User> {
        let row = sqlx::query_as::<_, (String, String, String, i64, i64, i64)>(
            "SELECT id, username, password, created, last_login, last_used FROM users WHERE username = ?",
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(DatabaseError::NotFound)?;

        let props = self.load_user_properties(&row.0).await?;
        Ok(User {
            id: row.0,
            username: row.1,
            password: row.2,
            created: chrono::DateTime::from_timestamp(row.3, 0).unwrap_or_default(),
            last_login: chrono::DateTime::from_timestamp(row.4, 0).unwrap_or_default(),
            last_used: chrono::DateTime::from_timestamp(row.5, 0).unwrap_or_default(),
            properties: props,
        })
    }

    async fn get_user_by_id(&self, user_id: &str) -> Result<User> {
        let row = sqlx::query_as::<_, (String, String, String, i64, i64, i64)>(
            "SELECT id, username, password, created, last_login, last_used FROM users WHERE id = ?",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(DatabaseError::NotFound)?;

        let props = self.load_user_properties(&row.0).await?;
        Ok(User {
            id: row.0,
            username: row.1,
            password: row.2,
            created: chrono::DateTime::from_timestamp(row.3, 0).unwrap_or_default(),
            last_login: chrono::DateTime::from_timestamp(row.4, 0).unwrap_or_default(),
            last_used: chrono::DateTime::from_timestamp(row.5, 0).unwrap_or_default(),
            properties: props,
        })
    }

    async fn get_all_users(&self) -> Result<Vec<User>> {
        let rows = sqlx::query_as::<_, (String, String, String, i64, i64, i64)>(
            "SELECT id, username, password, created, last_login, last_used FROM users",
        )
        .fetch_all(&self.pool)
        .await?;

        let mut users = Vec::new();
        for row in rows {
            let props = self.load_user_properties(&row.0).await?;
            users.push(User {
                id: row.0,
                username: row.1,
                password: row.2,
                created: chrono::DateTime::from_timestamp(row.3, 0).unwrap_or_default(),
                last_login: chrono::DateTime::from_timestamp(row.4, 0).unwrap_or_default(),
                last_used: chrono::DateTime::from_timestamp(row.5, 0).unwrap_or_default(),
                properties: props,
            });
        }
        Ok(users)
    }

    async fn upsert_user(&self, user: &User) -> Result<()> {
        sqlx::query("INSERT OR REPLACE INTO users VALUES (?, ?, ?, ?, ?, ?)")
            .bind(&user.id)
            .bind(&user.username)
            .bind(&user.password)
            .bind(user.created.timestamp())
            .bind(user.last_login.timestamp())
            .bind(user.last_used.timestamp())
            .execute(&self.pool)
            .await?;

        self.save_user_properties(&user.id, &user.properties).await?;
        Ok(())
    }

    async fn delete_user(&self, user_id: &str) -> Result<()> {
        // Cascade delete will remove user_properties automatically
        sqlx::query("DELETE FROM users WHERE id = ?")
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

#[async_trait]
impl AccessTokenRepo for SqliteRepository {
    async fn get_access_token(&self, token: &str) -> Result<AccessToken> {
        // Check cache first
        let cache = self.access_token_cache.lock().await;
        if let Some(token_data) = cache.get(token) {
            return Ok(token_data.clone());
        }
        drop(cache);

        // Query database
        let row = sqlx::query_as::<_, (String, String, String, String, String, String, String, i64, i64)>(
            "SELECT token, user_id, device_id, device_name, application_name, application_version, remote_address, created, last_used FROM access_tokens WHERE token = ?"
        )
        .bind(token)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(DatabaseError::NotFound)?;

        let token_data = AccessToken {
            token: row.0,
            user_id: row.1,
            device_id: row.2,
            device_name: row.3,
            application_name: row.4,
            application_version: row.5,
            remote_address: row.6,
            created: chrono::DateTime::from_timestamp(row.7, 0).unwrap_or_default(),
            last_used: chrono::DateTime::from_timestamp(row.8, 0).unwrap_or_default(),
        };

        // Update cache
        let mut cache = self.access_token_cache.lock().await;
        cache.insert(token.to_string(), token_data.clone());

        Ok(token_data)
    }

    async fn get_access_token_by_device_id(&self, device_id: &str) -> Result<AccessToken> {
        let row = sqlx::query_as::<_, (String, String, String, String, String, String, String, i64, i64)>(
            "SELECT token, user_id, device_id, device_name, application_name, application_version, remote_address, created, last_used FROM access_tokens WHERE device_id = ? LIMIT 1"
        )
        .bind(device_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(DatabaseError::NotFound)?;

        Ok(AccessToken {
            token: row.0,
            user_id: row.1,
            device_id: row.2,
            device_name: row.3,
            application_name: row.4,
            application_version: row.5,
            remote_address: row.6,
            created: chrono::DateTime::from_timestamp(row.7, 0).unwrap_or_default(),
            last_used: chrono::DateTime::from_timestamp(row.8, 0).unwrap_or_default(),
        })
    }

    async fn get_access_tokens(&self, user_id: &str) -> Result<Vec<AccessToken>> {
        let rows = sqlx::query_as::<_, (String, String, String, String, String, String, String, i64, i64)>(
            "SELECT token, user_id, device_id, device_name, application_name, application_version, remote_address, created, last_used FROM access_tokens WHERE user_id = ?"
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| AccessToken {
                token: row.0,
                user_id: row.1,
                device_id: row.2,
                device_name: row.3,
                application_name: row.4,
                application_version: row.5,
                remote_address: row.6,
                created: chrono::DateTime::from_timestamp(row.7, 0).unwrap_or_default(),
                last_used: chrono::DateTime::from_timestamp(row.8, 0).unwrap_or_default(),
            })
            .collect())
    }

    async fn upsert_access_token(&self, token: &AccessToken) -> Result<()> {
        let mut cache = self.access_token_cache.lock().await;
        cache.insert(token.token.clone(), token.clone());
        drop(cache);

        sqlx::query("INSERT OR REPLACE INTO access_tokens VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)")
            .bind(&token.token)
            .bind(&token.user_id)
            .bind(&token.device_id)
            .bind(&token.device_name)
            .bind(&token.application_name)
            .bind(&token.application_version)
            .bind(&token.remote_address)
            .bind(token.created.timestamp())
            .bind(token.last_used.timestamp())
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn delete_access_token(&self, token: &str) -> Result<()> {
        let mut cache = self.access_token_cache.lock().await;
        cache.remove(token);
        drop(cache);

        sqlx::query("DELETE FROM access_tokens WHERE token = ?")
            .bind(token)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}

#[async_trait]
impl ItemRepo for SqliteRepository {
    async fn db_load_item(&self, item: &mut Item) -> Result<()> {
        let row = sqlx::query_as::<_, (String, String, i32, String, f32, i32, i64, i64, i64)>(
            "SELECT id, name, votes, genre, rating, year, nfo_time, first_video, last_video FROM items WHERE id = ?",
        )
        .bind(&item.id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(DatabaseError::NotFound)?;

        item.name = row.1;
        item.votes = row.2;
        item.genre = row.3;
        item.rating = row.4;
        item.year = row.5;
        item.nfo_time = row.6;
        item.first_video = row.7;
        item.last_video = row.8;

        Ok(())
    }
}

#[async_trait]
impl UserDataRepo for SqliteRepository {
    async fn get_user_data(&self, user_id: &str, item_id: &str) -> Result<UserData> {
        // Check cache first
        let cache = self.user_data_cache.lock().await;
        let key = (user_id.to_string(), item_id.to_string());
        if let Some(data) = cache.get(&key) {
            return Ok(data.clone());
        }
        drop(cache);

        // Fall through to DB
        let row = sqlx::query_as::<_, (i64, i32, i32, bool, bool, i64)>(
            "SELECT position, played_percentage, play_count, played, favorite, timestamp FROM user_data WHERE user_id = ? AND item_id = ?"
        )
        .bind(user_id)
        .bind(item_id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(r) => {
                let data = UserData {
                    position: r.0,
                    played_percentage: r.1,
                    play_count: r.2,
                    played: r.3,
                    favorite: r.4,
                    timestamp: chrono::DateTime::from_timestamp(r.5, 0).unwrap_or_default(),
                };
                // Only cache entries that actually exist in the DB
                let mut cache = self.user_data_cache.lock().await;
                cache.insert((user_id.to_string(), item_id.to_string()), data.clone());
                Ok(data)
            }
            None => Err(DatabaseError::NotFound),
        }
    }

    async fn get_favorites(&self, user_id: &str) -> Result<Vec<String>> {
        let cache = self.user_data_cache.lock().await;
        let favorites: Vec<String> = cache
            .iter()
            .filter(|((uid, _), data)| uid == user_id && data.favorite)
            .map(|((_, item_id), _)| item_id.clone())
            .collect();

        Ok(favorites)
    }

    async fn get_recently_watched(&self, user_id: &str, include_fully_watched: bool, count: usize) -> Result<Vec<String>> {
        let cache = self.user_data_cache.lock().await;
        let mut items: Vec<_> = cache
            .iter()
            .filter(|((uid, _), data)| uid == user_id && (include_fully_watched || !data.played))
            .collect();

        items.sort_by(|a, b| b.1.timestamp.cmp(&a.1.timestamp));

        Ok(items.iter().take(count).map(|((_, item_id), _)| item_id.clone()).collect())
    }

    async fn update_user_data(&self, user_id: &str, item_id: &str, details: &UserData) -> Result<()> {
        // Write-through: persist to DB first, then update cache
        sqlx::query(
            "INSERT OR REPLACE INTO user_data (user_id, item_id, position, played_percentage, play_count, played, favorite, timestamp) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
        )
            .bind(user_id)
            .bind(item_id)
            .bind(details.position)
            .bind(details.played_percentage)
            .bind(details.play_count)
            .bind(details.played)
            .bind(details.favorite)
            .bind(details.timestamp.timestamp())
            .execute(&self.pool)
            .await
            .map_err(|e| {
                tracing::error!("Failed to write user_data for user={}, item={}: {}", user_id, item_id, e);
                DatabaseError::Sqlx(e)
            })?;

        let mut cache = self.user_data_cache.lock().await;
        let key = (user_id.to_string(), item_id.to_string());
        cache.insert(key, details.clone());
        Ok(())
    }
}

#[async_trait]
impl PlaylistRepo for SqliteRepository {
    async fn create_playlist(&self, playlist: &Playlist) -> Result<String> {
        let item_ids_json = serde_json::to_string(&playlist.item_ids)?;
        let now = chrono::Utc::now().to_rfc3339();

        #[allow(unused_assignments)]
        let mut new_id = String::new();
        let playlist_id = if playlist.id == "" {
            new_id = id_hash_prefix(ITEM_PREFIX_PLAYLIST, &format!("{}:{}", playlist.user_id, playlist.name));
            new_id.as_str()
        } else {
            &playlist.id
        };

        sqlx::query(
            "INSERT OR REPLACE INTO playlists (id, user_id, name, item_ids, created, last_updated) VALUES (?, ?, ?, ?, ?, ?)"
        )
            .bind(&playlist_id)
            .bind(&playlist.user_id)
            .bind(&playlist.name)
            .bind(&item_ids_json)
            .bind(&now)
            .bind(&now)
            .execute(&self.pool)
            .await?;

        Ok(playlist.id.clone())
    }

    async fn get_playlists(&self, user_id: &str) -> Result<Vec<String>> {
        let rows = sqlx::query_as::<_, (String,)>("SELECT id FROM playlists WHERE user_id = ?")
            .bind(user_id)
            .fetch_all(&self.pool)
            .await?;

        Ok(rows.into_iter().map(|r| r.0).collect())
    }

    async fn get_playlist(&self, user_id: &str, playlist_id: &str) -> Result<Playlist> {
        let row = sqlx::query_as::<_, (String, String, String, String, String, String)>(
            "SELECT id, user_id, name, item_ids, created, last_updated FROM playlists WHERE id = ? AND user_id = ?",
        )
        .bind(playlist_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(DatabaseError::NotFound)?;

        let item_ids: Vec<String> = serde_json::from_str(&row.3)?;
        let created = chrono::DateTime::parse_from_rfc3339(&row.4)
            .map(|d| d.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now());
        let last_updated = chrono::DateTime::parse_from_rfc3339(&row.5)
            .map(|d| d.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now());

        Ok(Playlist {
            id: row.0,
            user_id: row.1,
            name: row.2,
            item_ids,
            created,
            last_updated,
        })
    }

    async fn get_playlist_by_name(&self, user_id: &str, name: &str) -> Result<Playlist> {
        let row = sqlx::query_as::<_, (String, String, String, String, String, String)>(
            "SELECT id, user_id, name, item_ids, created, last_updated FROM playlists WHERE user_id = ? AND name = ?",
        )
        .bind(user_id)
        .bind(name)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(DatabaseError::NotFound)?;

        let item_ids: Vec<String> = serde_json::from_str(&row.3)?;
        let created = chrono::DateTime::parse_from_rfc3339(&row.4)
            .map(|d| d.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now());
        let last_updated = chrono::DateTime::parse_from_rfc3339(&row.5)
            .map(|d| d.with_timezone(&chrono::Utc))
            .unwrap_or_else(|_| chrono::Utc::now());

        Ok(Playlist {
            id: row.0,
            user_id: row.1,
            name: row.2,
            item_ids,
            created,
            last_updated,
        })
    }

    async fn add_items_to_playlist(&self, _user_id: &str, playlist_id: &str, item_ids: &[String]) -> Result<()> {
        let row = sqlx::query_as::<_, (String,)>("SELECT item_ids FROM playlists WHERE id = ?")
            .bind(playlist_id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or(DatabaseError::NotFound)?;

        let mut existing_ids: Vec<String> = serde_json::from_str(&row.0)?;
        existing_ids.extend_from_slice(item_ids);

        let item_ids_json = serde_json::to_string(&existing_ids)?;
        sqlx::query("UPDATE playlists SET item_ids = ? WHERE id = ?")
            .bind(&item_ids_json)
            .bind(playlist_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn delete_items_from_playlist(&self, playlist_id: &str, item_ids: &[String]) -> Result<()> {
        let row = sqlx::query_as::<_, (String,)>("SELECT item_ids FROM playlists WHERE id = ?")
            .bind(playlist_id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or(DatabaseError::NotFound)?;

        let mut existing_ids: Vec<String> = serde_json::from_str(&row.0)?;
        existing_ids.retain(|id| !item_ids.contains(id));

        let item_ids_json = serde_json::to_string(&existing_ids)?;
        sqlx::query("UPDATE playlists SET item_ids = ? WHERE id = ?")
            .bind(&item_ids_json)
            .bind(playlist_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn move_playlist_item(&self, playlist_id: &str, item_id: &str, new_index: i32) -> Result<()> {
        let row = sqlx::query_as::<_, (String,)>("SELECT item_ids FROM playlists WHERE id = ?")
            .bind(playlist_id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or(DatabaseError::NotFound)?;

        let mut existing_ids: Vec<String> = serde_json::from_str(&row.0)?;

        if let Some(old_index) = existing_ids.iter().position(|id| id == item_id) {
            existing_ids.remove(old_index);
            let new_idx = (new_index as usize).min(existing_ids.len());
            existing_ids.insert(new_idx, item_id.to_string());

            let item_ids_json = serde_json::to_string(&existing_ids)?;
            sqlx::query("UPDATE playlists SET item_ids = ? WHERE id = ?")
                .bind(&item_ids_json)
                .bind(playlist_id)
                .execute(&self.pool)
                .await?;
        }

        Ok(())
    }
}
#[async_trait]
impl PersonRepo for SqliteRepository {
    async fn get_person(&self, name: &str, _user_id: &str) -> Result<Person> {
        let row = sqlx::query_as::<_, (String, String, i64, String, String, String, i64, i64)>(
            "SELECT id, name, date_of_birth, place_of_birth, poster_url, bio, created, last_updated FROM persons WHERE name = ?"
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(DatabaseError::NotFound)?;

        Ok(Person {
            id: row.0,
            name: row.1,
            date_of_birth: chrono::DateTime::from_timestamp(row.2, 0).unwrap_or_default(),
            place_of_birth: row.3,
            poster_url: row.4,
            bio: row.5,
            created: chrono::DateTime::from_timestamp(row.6, 0).unwrap_or_default(),
            last_updated: chrono::DateTime::from_timestamp(row.7, 0).unwrap_or_default(),
        })
    }
}

#[async_trait]
impl QuickConnectRepo for SqliteRepository {
    async fn get_quick_connect_by_secret(&self, secret: &str) -> Result<QuickConnectCode> {
        let row = sqlx::query_as::<_, (String, String, String, bool, String, i64)>(
            "SELECT userid, deviceid, secret, authorized, code, created FROM quickconnect WHERE secret = ?"
        )
        .bind(secret)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(DatabaseError::NotFound)?;

        Ok(QuickConnectCode {
            user_id: row.0,
            device_id: row.1,
            secret: row.2,
            authorized: row.3,
            code: row.4,
            created: chrono::DateTime::from_timestamp(row.5, 0).unwrap_or_default(),
        })
    }

    async fn get_quick_connect_by_code(&self, code: &str) -> Result<QuickConnectCode> {
        let row = sqlx::query_as::<_, (String, String, String, bool, String, i64)>(
            "SELECT userid, deviceid, secret, authorized, code, created FROM quickconnect WHERE code = ?"
        )
        .bind(code)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(DatabaseError::NotFound)?;

        Ok(QuickConnectCode {
            user_id: row.0,
            device_id: row.1,
            secret: row.2,
            authorized: row.3,
            code: row.4,
            created: chrono::DateTime::from_timestamp(row.5, 0).unwrap_or_default(),
        })
    }

    async fn upsert_quick_connect(&self, qc: &QuickConnectCode) -> Result<()> {
        sqlx::query(
            "INSERT OR REPLACE INTO quickconnect (userid, deviceid, secret, authorized, code, created) VALUES (?, ?, ?, ?, ?, ?)"
        )
        .bind(&qc.user_id)
        .bind(&qc.device_id)
        .bind(&qc.secret)
        .bind(qc.authorized)
        .bind(&qc.code)
        .bind(qc.created.timestamp())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn delete_expired_quick_connects(&self, before: chrono::DateTime<chrono::Utc>) -> Result<()> {
        sqlx::query("DELETE FROM quickconnect WHERE created < ?")
            .bind(before.timestamp())
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

#[async_trait]
impl ImageRepo for SqliteRepository {
    async fn has_image(&self, item_id: &str, image_type: &str) -> Result<Option<ImageMetadata>> {
        let row = sqlx::query_as::<_, (String, i64, String, i64)>(
            "SELECT mimetype, filesize, etag, updated FROM images WHERE itemid = ? AND type = ?"
        )
        .bind(item_id)
        .bind(image_type)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|(mime_type, file_size, etag, updated)| ImageMetadata {
            mime_type,
            file_size,
            etag,
            updated: chrono::DateTime::from_timestamp(updated, 0).unwrap_or_default(),
        }))
    }

    async fn get_image(&self, item_id: &str, image_type: &str) -> Result<(ImageMetadata, Vec<u8>)> {
        let row = sqlx::query_as::<_, (String, i64, String, i64, Vec<u8>)>(
            "SELECT mimetype, filesize, etag, updated, data FROM images WHERE itemid = ? AND type = ?"
        )
        .bind(item_id)
        .bind(image_type)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(DatabaseError::NotFound)?;

        let meta = ImageMetadata {
            mime_type: row.0,
            file_size: row.1,
            etag: row.2,
            updated: chrono::DateTime::from_timestamp(row.3, 0).unwrap_or_default(),
        };
        Ok((meta, row.4))
    }

    async fn store_image(&self, item_id: &str, image_type: &str, meta: &ImageMetadata, data: &[u8]) -> Result<()> {
        sqlx::query(
            "INSERT OR REPLACE INTO images (itemid, type, mimetype, etag, updated, filesize, data) VALUES (?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(item_id)
        .bind(image_type)
        .bind(&meta.mime_type)
        .bind(&meta.etag)
        .bind(meta.updated.timestamp())
        .bind(meta.file_size)
        .bind(data)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn delete_image(&self, item_id: &str, image_type: &str) -> Result<()> {
        sqlx::query("DELETE FROM images WHERE itemid = ? AND type = ?")
            .bind(item_id)
            .bind(image_type)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
