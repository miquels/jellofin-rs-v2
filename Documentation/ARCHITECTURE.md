# Jellofin-rs Architecture

## Overview

Jellofin-rs is a Rust port of a Go-based Jellyfin-compatible media server. It implements two APIs:
- **Jellyfin API** - Compatible with official Jellyfin clients (web, mobile, TV apps)
- **Notflix API** - A custom legacy API under `/api/`

The server scans media collections (movies and TV shows), indexes them for search, serves media files with optional image resizing, and manages user authentication and playback state.

## High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         HTTP Server (axum)                      │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐  │
│  │  Jellyfin API   │  │   Notflix API   │  │  Static Files   │  │
│  │   /Users/*      │  │     /api/*      │  │   /data/*       │  │
│  │   /Items/*      │  │  /collection/*  │  │                 │  │
│  │   /System/*     │  │                 │  │                 │  │
│  │   /... more ...
│  └────────┬────────┘  └────────┬────────┘  └────────┬────────┘  │
└───────────┼─────────────────────┼─────────────────────┼──────────┘
            │                     │                     │
            ▼                     ▼                     ▼
┌─────────────────────────────────────────────────────────────────┐
│                        Core Services                            │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐  │
│  │   Collection    │  │    Database     │  │  ImageResizer   │  │
│  │     Repo        │  │   Repository    │  │                 │  │
│  └────────┬────────┘  └────────┬────────┘  └────────┬────────┘  │
│           │                    │                    │           │
│           ▼                    ▼                    ▼           │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐  │
│  │ Search Index    │  │     SQLite      │  │  Image Cache    │  │
│  │   (tantivy)     │  │                 │  │                 │  │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
            │
            ▼
┌─────────────────────────────────────────────────────────────────┐
│                        File System                              │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐  │
│  │ Media Files     │  │  NFO Metadata   │  │  Images         │  │
│  │ (.mp4)          │  │  (.nfo XML)     │  │  (.jpg, .png,   │  │
│  │                 │  │                 │  │    .gif, .tbn)  │  │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

## Module Structure

### Source Directory Layout

```
src/
├── lib.rs                 # Library entry point
├── bin/
│   └── main.rs            # Binary entry point, CLI parsing
├── collection/
│   ├── mod.rs             # Module exports
│   ├── collection.rs      # Collection struct and methods
│   ├── collectionrepo.rs  # CollectionRepo - manages all collections
│   ├── item.rs            # Item enum, Movie, Show, Season, Episode structs
│   ├── kodifs.rs          # Kodi-style filesystem scanner
│   ├── parsefilename.rs   # Episode filename parsing (S01E02, etc.)
│   ├── metadata/
│   │   └── mod.rs         # NFO file parsing, metadata structs
│   └── search/
│       └── mod.rs         # Tantivy search index
├── database/
│   ├── mod.rs             # Module exports, Repository trait
│   ├── model.rs           # User, AccessToken, UserData, Playlist structs
│   └── sqlite.rs          # SQLite implementation
├── idhash/
│   └── mod.rs             # Hash string to 20-char identifier
├── imageresize/
│   └── mod.rs             # Image resizing with caching
├── jellyfin/
│   ├── mod.rs             # Jellyfin API handler registration
│   ├── auth.rs            # Authentication handlers
│   ├── branding.rs        # Branding configuration
│   ├── device.rs          # Device management
│   ├── error.rs           # Error handling
│   ├── genre.rs           # Genre handlers
│   ├── item.rs            # Item handlers (/Items/*)
│   ├── jellyfin.rs        # Axum Router for Jellyfin API handlers
│   ├── jfitem.rs          # Conversion from internal types to Jellyfin API types.
│   ├── library.rs         # Library handlers.
│   ├── localization.rs    # Localization handlers
│   ├── movie.rs           # Movie-specific handlers
│   ├── person.rs          # Person/actor handlers
│   ├── playlist.rs        # Playlist handlers
│   ├── session.rs         # Session management
│   ├── show.rs            # Show-specific handlers
│   ├── studio.rs          # Studio handlers
│   ├── system.rs          # System info handlers
│   ├── types.rs           # Jellyfin API types (BaseItemDto, etc.)
│   ├── user.rs            # User handlers (/Users/*)
│   └── userdata.rs        # Playback state handlers
├── notflix/
│   ├── mod.rs             # Notflix API handler registration
│   ├── types.rs           # Notflix API types
│   ├── handlers.rs        # API handlers
│   ├── proxy.rs           # HLS proxy
│   └── subtitles.rs       # Subtitle handling
└── server.rs              # HTTP server setup, middleware
```

## Core Components

### 1. Collection Module

The collection module is responsible for scanning media directories and building an in-memory representation of all media items.

#### Key Types

```rust
// Item enum - replaces Go interfaces
enum Item {
    Movie(Movie),
    Show(Show),
}

// For borrowing without ownership
enum ItemRef<'a> {
    Movie(&'a Movie),
    Show(&'a Show),
    Season(&'a Season),
    Episode(&'a Episode),
}

struct Movie {
    id: String,
    name: String,
    sort_name: String,
    path: String,           // Relative to collection root
    poster: Option<String>,
    fanart: Option<String>,
    metadata: Metadata,
    srt_subs: Vec<Subtitle>,
    vtt_subs: Vec<Subtitle>,
}

struct Show {
    id: String,
    name: String,
    sort_name: String,
    path: String,
    seasons: Vec<Season>,
    metadata: Metadata,
    // ... images, subtitles
}

struct Season {
    id: String,
    name: String,
    season_no: i32,
    episodes: Vec<Episode>,
    poster: Option<String>,
}

struct Episode {
    id: String,
    name: String,
    season_no: i32,
    episode_no: i32,
    file_name: String,
    metadata: Metadata,
}
```

#### CollectionRepo

Manages all collections and provides lookup methods:

```rust
struct CollectionRepo {
    collections: Arc<ArcSwap<Vec<Collection>>>,
    search_index: Arc<RwLock<SearchIndex>>,
}

impl CollectionRepo {
    fn get_collections(&self) -> Vec<Collection>;
    fn get_collection(&self, id: &str) -> Option<Collection>;
    fn get_item(&self, collection_id: &str, item_id: &str) -> Option<ItemRef>;
    fn get_item_by_id(&self, item_id: &str) -> Option<(Collection, ItemRef)>;
    fn search(&self, term: &str) -> Vec<String>;
    fn similar(&self, item: ItemRef) -> Vec<String>;
}
```

#### Filesystem Scanning

The scanner (kodifs.rs) walks media directories in Kodi-style format:

**Movies:**
```
/movies/
  Movie Name (2024)/
    movie.nfo          # Metadata
    Movie Name.mkv     # Video file
    poster.jpg         # Artwork
    fanart.jpg
```

**TV Shows:**
```
/shows/
  Show Name/
    tvshow.nfo         # Show metadata
    poster.jpg
    Season 01/
      S01E01 - Episode Name.mkv
      S01E01 - Episode Name.nfo
      S01E01 - Episode Name-thumb.jpg
```

### 2. Database Module

Handles persistent storage using SQLite via sqlx.

#### Repository Trait

```rust
trait Repository: UserRepo + AccessTokenRepo + ItemRepo + UserDataRepo + PlaylistRepo {
    fn start_background_jobs(&self);
}

trait UserRepo {
    async fn get_user(&self, username: &str) -> Result<User>;
    async fn get_user_by_id(&self, user_id: &str) -> Result<User>;
    async fn upsert_user(&self, user: &User) -> Result<()>;
}

trait AccessTokenRepo {
    async fn get_access_token(&self, token: &str) -> Result<AccessToken>;
    async fn get_access_tokens(&self, user_id: &str) -> Result<Vec<AccessToken>>;
    async fn upsert_access_token(&self, token: AccessToken) -> Result<()>;
    async fn delete_access_token(&self, token: &str) -> Result<()>;
}

trait UserDataRepo {
    async fn get_user_data(&self, user_id: &str, item_id: &str) -> Result<UserData>;
    async fn get_favorites(&self, user_id: &str) -> Result<Vec<String>>;
    async fn get_recently_watched(&self, user_id: &str) -> Result<Vec<String>>;
    async fn update_user_data(&self, user_id: &str, item_id: &str, data: &UserData) -> Result<()>;
}

trait PlaylistRepo {
    async fn create_playlist(&self, playlist: Playlist) -> Result<String>;
    async fn get_playlists(&self, user_id: &str) -> Result<Vec<String>>;
    async fn get_playlist(&self, user_id: &str, playlist_id: &str) -> Result<Playlist>;
    async fn add_items_to_playlist(&self, playlist_id: &str, items: &[String]) -> Result<()>;
    async fn delete_items_from_playlist(&self, playlist_id: &str, items: &[String]) -> Result<()>;
}
```

#### Data Models

```rust
struct User {
    id: String,
    username: String,
    password: String,  // Hashed
    created: DateTime<Utc>,
    last_login: DateTime<Utc>,
}

struct AccessToken {
    user_id: String,
    token: String,
    device_id: String,
    device_name: String,
    application_name: String,
    application_version: String,
    created: DateTime<Utc>,
    last_used: DateTime<Utc>,
}

struct UserData {
    position: i64,           // Playback position in seconds
    played_percentage: i32,
    play_count: i32,
    played: bool,
    favorite: bool,
    timestamp: DateTime<Utc>,
}

struct Playlist {
    id: String,
    user_id: String,
    name: String,
    item_ids: Vec<String>,
}
```

### 3. Image Resizer Module

On-demand image resizing with disk caching.

```rust
struct ImageResizer {
    cache_dir: PathBuf,
}

impl ImageResizer {
    fn resize(&self, path: &Path, width: Option<u32>, height: Option<u32>, quality: Option<u32>) -> Result<PathBuf>;
}
```

- Cache key based on file inode/device + requested dimensions
- Supports JPEG, PNG
- Returns path to cached file (or original if no resize needed)

### 4. Search Module

Full-text search using Tantivy.

```rust
struct SearchIndex {
    index: tantivy::Index,
}

struct SearchDocument {
    id: String,
    parent_id: String,
    name: String,
    name_exact: String,
    sort_name: String,
    overview: String,
    genres: Vec<String>,
    year: i32,
}

impl SearchIndex {
    fn index_batch(&mut self, docs: Vec<SearchDocument>) -> Result<()>;
    fn search(&self, term: &str, limit: usize) -> Result<Vec<String>>;
    fn similar(&self, doc: SearchDocument, limit: usize) -> Result<Vec<String>>;
}
```

### 5. ID Hash Module

Generates deterministic 20-character IDs from strings:

```rust
fn id_hash(input: &str) -> String {
    // SHA256 hash, take first 20 hex characters
}
```

## API Endpoints

### Jellyfin API (selected endpoints)

| Method | Path | Description |
|--------|------|-------------|
| POST | `/Users/AuthenticateByName` | User login |
| GET | `/Users/Me` | Current user info |
| GET | `/Users/{user}/Views` | Library views |
| GET | `/Items` | Query items |
| GET | `/Items/{id}` | Get item details |
| GET | `/Items/{id}/Images/{type}` | Get item image |
| POST | `/Items/{id}/PlaybackInfo` | Get playback info |
| GET | `/Videos/{id}/stream` | Stream video |
| GET | `/Search/Hints` | Search items |
| GET | `/Shows/NextUp` | Next episodes to watch |
| POST | `/Users/{user}/PlayedItems/{id}` | Mark as played |
| POST | `/Playlists` | Create playlist |
| GET | `/System/Info` | Server info |

### Notflix API

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/collections` | List collections |
| GET | `/api/collection/{id}` | Get collection |
| GET | `/api/collection/{id}/items` | List items |
| GET | `/api/collection/{id}/item/{id}` | Get item details |
| GET | `/api/collection/{id}/genres` | Genre counts |
| GET | `/data/{source}/{path}` | Serve media files |

## Concurrency Model

### Shared State

```rust
struct AppState {
    config: Arc<Config>,
    collections: Arc<CollectionRepo>,
    db: Arc<dyn Repository>,
    image_resizer: Arc<ImageResizer>,
}
```

### Update Strategy

For collections that need periodic updates:

1. Clone current collection data
2. Update the clone with new filesystem state
3. Swap the clone in atomically using `ArcSwap`

```rust
// In CollectionRepo
fn update_collections(&self) {
    let current = self.collections.load();
    let mut updated = (*current).clone();
    
    for collection in &mut updated {
        self.scan_collection(collection);
    }
    
    self.collections.store(Arc::new(updated));
}
```

### Locking Guidelines

- **Arc**: Read-only shared data
- **ArcSwap**: Data that's read frequently, updated infrequently (collections)
- **Mutex**: Short critical sections with no contention
- **RwLock**: Data with mixed read/write access patterns

## Configuration

YAML configuration file (compatible with Go version):

```yaml
listen:
  address: "0.0.0.0"
  port: "8096"
  tlscert: ""
  tlskey: ""

appdir: "/path/to/web/app"
cachedir: "/path/to/cache"

database:
  sqlite:
    filename: "/path/to/database.db"

collections:
  - id: "movies"
    name: "Movies"
    type: "movies"
    directory: "/media/movies"
    hlsserver: ""
  
  - id: "shows"
    name: "TV Shows"
    type: "shows"
    directory: "/media/shows"
    hlsserver: ""

jellyfin:
  serverid: ""
  servername: "Jellofin"
  autoregister: true
  imagequalityposter: 90
```

## Request Flow

1. **HTTP Request** arrives at axum server
2. **Middleware** normalizes path (removes `/emby` prefix, double slashes)
3. **Router** matches endpoint
4. **Auth middleware** (for protected endpoints) validates token
5. **Handler** processes request using AppState
6. **Response** sent back to client

## Error Handling

Use `thiserror` for error types:

```rust
#[derive(Debug, thiserror::Error)]
enum AppError {
    #[error("Not found")]
    NotFound,
    #[error("Unauthorized")]
    Unauthorized,
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = match &self {
            AppError::NotFound => StatusCode::NOT_FOUND,
            AppError::Unauthorized => StatusCode::UNAUTHORIZED,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, self.to_string()).into_response()
    }
}
```

## Testing Strategy

1. **Unit tests**: Individual functions, parsing logic
2. **Integration tests**: Database operations, API endpoints
3. **Manual testing**: With actual Jellyfin clients

## Dependencies

| Purpose | Crate |
|---------|-------|
| HTTP server | axum, axum-server |
| Async runtime | tokio |
| Database | sqlx (sqlite) |
| Serialization | serde, serde_json, serde_yaml |
| Image processing | image |
| Search | tantivy |
| CLI | clap |
| Hashing | sha2 |
| Time | chrono |
| Logging | tracing, tracing-subscriber |
| Error handling | thiserror |
