use async_trait::async_trait;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Mutex;

use super::model::{AccessToken, DatabaseError, Item, Person, Playlist, Result, User, UserData};
use super::{AccessTokenRepo, ItemRepo, PersonRepo, PlaylistRepo, Repository, UserDataRepo, UserRepo};

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

        Ok(())
    }

    /// Load user data from database into cache
    async fn load_user_data_from_db(&self) -> Result<()> {
        let rows = sqlx::query_as::<_, (String, String, i64, i32, i32, bool, bool, i64)>(
            "SELECT user_id, item_id, position, played_percentage, play_count, played, favorite, timestamp FROM user_data"
        )
        .fetch_all(&self.pool)
        .await?;

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
        let access_token_cache = Arc::clone(&self.access_token_cache);
        let user_data_cache = Arc::clone(&self.user_data_cache);
        let pool = self.pool.clone();

        // Spawn background task for syncing caches
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(10));
            loop {
                interval.tick().await;

                // Sync access tokens
                let tokens = access_token_cache.lock().await;
                for token in tokens.values() {
                    let _ = sqlx::query("INSERT OR REPLACE INTO access_tokens VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)")
                        .bind(&token.token)
                        .bind(&token.user_id)
                        .bind(&token.device_id)
                        .bind(&token.device_name)
                        .bind(&token.application_name)
                        .bind(&token.application_version)
                        .bind(&token.remote_address)
                        .bind(token.created.timestamp())
                        .bind(token.last_used.timestamp())
                        .execute(&pool)
                        .await;
                }
                drop(tokens);

                // Sync user data
                let user_data = user_data_cache.lock().await;
                for ((user_id, item_id), data) in user_data.iter() {
                    let _ = sqlx::query("INSERT OR REPLACE INTO user_data VALUES (?, ?, ?, ?, ?, ?, ?, ?)")
                        .bind(user_id)
                        .bind(item_id)
                        .bind(data.position)
                        .bind(data.played_percentage)
                        .bind(data.play_count)
                        .bind(data.played)
                        .bind(data.favorite)
                        .bind(data.timestamp.timestamp())
                        .execute(&pool)
                        .await;
                }
            }
        });
    }
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

        Ok(User {
            id: row.0,
            username: row.1,
            password: row.2,
            created: chrono::DateTime::from_timestamp(row.3, 0).unwrap_or_default(),
            last_login: chrono::DateTime::from_timestamp(row.4, 0).unwrap_or_default(),
            last_used: chrono::DateTime::from_timestamp(row.5, 0).unwrap_or_default(),
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

        Ok(User {
            id: row.0,
            username: row.1,
            password: row.2,
            created: chrono::DateTime::from_timestamp(row.3, 0).unwrap_or_default(),
            last_login: chrono::DateTime::from_timestamp(row.4, 0).unwrap_or_default(),
            last_used: chrono::DateTime::from_timestamp(row.5, 0).unwrap_or_default(),
        })
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
        let cache = self.user_data_cache.lock().await;
        let key = (user_id.to_string(), item_id.to_string());

        if let Some(data) = cache.get(&key) {
            return Ok(data.clone());
        }

        // Return default if not found
        Ok(UserData {
            position: 0,
            played_percentage: 0,
            play_count: 0,
            played: false,
            favorite: false,
            timestamp: chrono::Utc::now(),
        })
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

    async fn get_recently_watched(&self, user_id: &str, include_fully_watched: bool) -> Result<Vec<String>> {
        let cache = self.user_data_cache.lock().await;
        let mut items: Vec<_> = cache
            .iter()
            .filter(|((uid, _), data)| uid == user_id && (include_fully_watched || !data.played))
            .collect();

        items.sort_by(|a, b| b.1.timestamp.cmp(&a.1.timestamp));

        Ok(items.iter().take(10).map(|((_, item_id), _)| item_id.clone()).collect())
    }

    async fn update_user_data(&self, user_id: &str, item_id: &str, details: &UserData) -> Result<()> {
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

        sqlx::query("INSERT INTO playlists VALUES (?, ?, ?, ?)")
            .bind(&playlist.id)
            .bind(&playlist.user_id)
            .bind(&playlist.name)
            .bind(&item_ids_json)
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
        let row = sqlx::query_as::<_, (String, String, String, String)>(
            "SELECT id, user_id, name, item_ids FROM playlists WHERE id = ? AND user_id = ?",
        )
        .bind(playlist_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(DatabaseError::NotFound)?;

        let item_ids: Vec<String> = serde_json::from_str(&row.3)?;

        Ok(Playlist {
            id: row.0,
            user_id: row.1,
            name: row.2,
            item_ids,
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
