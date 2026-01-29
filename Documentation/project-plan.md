# Jellofin-rs Project Plan

## Overview

This document outlines the execution plan for porting the jellofin-go server to Rust. The port follows a line-by-line translation approach while adapting to Rust idioms where necessary.

## Phases

### Phase 1: Project Setup and Core Infrastructure

**Goal:** Establish the Rust project structure and core dependencies.

#### Tasks

- [ ] **1.1 Initialize Cargo project**
  - Create `Cargo.toml` with all dependencies
  - Set up binary and library targets
  - Configure workspace if needed

- [ ] **1.2 Create module structure**
  ```
  src/
  ├── lib.rs
  ├── bin/main.rs
  ├── collection/mod.rs
  ├── database/mod.rs
  ├── idhash/mod.rs
  ├── imageresize/mod.rs
  ├── jellyfin/mod.rs
  ├── notflix/mod.rs
  └── server.rs
  ```

- [ ] **1.3 Implement configuration loading**
  - Port `configFile` struct from `server.go`
  - Use serde_yaml for parsing
  - Implement `Config::from_file()`

- [ ] **1.4 Implement CLI argument parsing**
  - Use clap with derive macros
  - Support `--config` flag

- [ ] **1.5 Set up HTTP server skeleton**
  - Create axum router
  - Implement path normalization middleware
  - Implement request logging middleware
  - Set up TLS support with certificate reloading

**Verification:** Server starts and responds to `/health` endpoint.

---

### Phase 2: ID Hash and Utility Modules

**Goal:** Port simple utility modules.

#### Tasks

- [ ] **2.1 Port idhash module**
  - File: `idhash/idhash.go` → `src/idhash/mod.rs`
  - Implement `id_hash(input: &str) -> String`
  - Use sha2 crate for SHA256

**Verification:** `cargo check` passes, unit tests for id_hash.

---

### Phase 3: Database Module

**Goal:** Port database layer with SQLite support.

#### Tasks

- [ ] **3.1 Port data models**
  - File: `database/model/model.go` → `src/database/model.rs`
  - Structs: `User`, `AccessToken`, `Item`, `UserData`, `Playlist`
  - Define error types

- [ ] **3.2 Define repository traits**
  - File: `database/database.go` → `src/database/mod.rs`
  - Traits: `UserRepo`, `AccessTokenRepo`, `ItemRepo`, `UserDataRepo`, `PlaylistRepo`
  - Aggregate `Repository` trait

- [ ] **3.3 Implement SQLite repository**
  - Files: `database/sqlite/*.go` → `src/database/sqlite.rs`
  - Use sqlx with SQLite feature
  - Implement all repository traits
  - Schema creation/migration
  - Background job for cache flushing

**Verification:** Database tests pass, can create/read users and tokens.

---

### Phase 4: Collection Module - Core Types

**Goal:** Port collection data structures.

#### Tasks

- [ ] **4.1 Port Item types**
  - File: `collection/item.go` → `src/collection/item.rs`
  - Create `Item` enum with `Movie`, `Show` variants
  - Create `ItemRef` enum for borrowing
  - Structs: `Movie`, `Show`, `Season`, `Episode`, `Subtitle`
  - Implement common methods on the enum

- [ ] **4.2 Port Collection type**
  - File: `collection/collection.go` → `src/collection/collection.rs`
  - Struct: `Collection`, `CollectionType`, `CollectionDetails`
  - Methods: `details()`, `genre_count()`

- [ ] **4.3 Port Metadata types**
  - Files: `collection/metadata/*.go` → `src/collection/metadata/mod.rs`
  - NFO parsing (XML)
  - Metadata struct with all fields

**Verification:** Types compile, can create collection instances.

---

### Phase 5: Collection Module - Filesystem Scanning

**Goal:** Port filesystem scanning logic.

#### Tasks

- [ ] **5.1 Port filename parsing**
  - File: `collection/parsefilename.go` → `src/collection/parsefilename.rs`
  - Episode number extraction (S01E02, 1x02, etc.)
  - Regex patterns

- [ ] **5.2 Port directory scanning**
  - File: `collection/opendir.go` → `src/collection/opendir.rs`
  - Directory enumeration helpers

- [ ] **5.3 Port Kodi filesystem scanner**
  - File: `collection/kodifs.go` → `src/collection/kodifs.rs`
  - `build_movies()` - scan movie directories
  - `build_shows()` - scan TV show directories
  - Image detection (poster, fanart, banner)
  - NFO file detection and parsing

- [ ] **5.4 Port CollectionRepo**
  - File: `collection/collectionrepo.go` → `src/collection/collectionrepo.rs`
  - `CollectionRepo` struct
  - Methods: `add_collection()`, `init()`, `background()`
  - Item lookup: `get_item()`, `get_item_by_id()`, `get_episode_by_id()`
  - `next_up()` logic

**Verification:** Can scan a test media directory and list items.

---

### Phase 6: Search Module

**Goal:** Port search functionality using Tantivy.

#### Tasks

- [ ] **6.1 Port search index**
  - File: `collection/search/search.go` → `src/collection/search/mod.rs`
  - `SearchIndex` struct
  - `SearchDocument` struct
  - Methods: `new()`, `index_batch()`, `search()`, `similar()`
  - Note: Tantivy API differs from Bleve, focus on equivalent functionality

- [ ] **6.2 Integrate search with CollectionRepo**
  - `build_search_index()` method
  - Search and similar item queries

**Verification:** Can search for items by name, get similar items.

---

### Phase 7: Image Resizer Module

**Goal:** Port image resizing with caching.

#### Tasks

- [ ] **7.1 Port image resizer**
  - File: `imageresize/imageresize.go` → `src/imageresize/mod.rs`
  - `ImageResizer` struct
  - Cache key generation
  - `resize()` method with width/height/quality params
  - Disk cache management
  - Use `image` crate for resizing

**Verification:** Can resize images, cache works correctly.

---

### Phase 8: Notflix API

**Goal:** Port the legacy Notflix API.

#### Tasks

- [ ] **8.1 Port Notflix types**
  - File: `notflix/apitypes.go` → `src/notflix/types.rs`
  - `Collection`, `Item`, `Season`, `Episode` API types
  - `ItemNfo`, `EpisodeNfo` types

- [ ] **8.2 Port Notflix handlers**
  - File: `notflix/apihandler.go` → `src/notflix/handlers.rs`
  - `/api/collections` - list collections
  - `/api/collection/{id}` - get collection
  - `/api/collection/{id}/items` - list items
  - `/api/collection/{id}/item/{id}` - get item details
  - `/api/collection/{id}/genres` - genre counts
  - `/data/{source}/{path}` - serve media files

- [ ] **8.3 Port ETag handling**
  - File: `notflix/etag.go` → `src/notflix/etag.rs`
  - ETag generation and validation

- [ ] **8.4 Port HLS proxy**
  - File: `notflix/proxy.go` → `src/notflix/proxy.rs`
  - Proxy requests to HLS server

- [ ] **8.5 Port subtitle handling**
  - File: `notflix/subtitles.go` → `src/notflix/subtitles.rs`
  - SRT/VTT subtitle serving

**Verification:** Notflix API endpoints work, can browse collections.

---

### Phase 9: Jellyfin API - Authentication

**Goal:** Port Jellyfin authentication.

#### Tasks

- [ ] **9.1 Port auth handlers**
  - File: `jellyfin/auth.go` → `src/jellyfin/auth.rs`
  - `/Users/AuthenticateByName` - login
  - Auth middleware for token validation
  - Token extraction from headers/query params
  - Auto-registration support

**Verification:** Can authenticate and get valid token.

---

### Phase 10: Jellyfin API - Core Types

**Goal:** Port Jellyfin API types.

#### Tasks

- [ ] **10.1 Port Jellyfin types**
  - File: `jellyfin/type.go` → `src/jellyfin/types.rs`
  - `BaseItemDto` - main item representation
  - `MediaSourceInfo`, `MediaStream`
  - `UserDto`, `UserPolicy`
  - `SystemInfo`, `PublicSystemInfo`
  - Note: Use `#[serde(rename_all = "PascalCase")]` for JSON

- [ ] **10.2 Port item conversion**
  - File: `jellyfin/jfitem.go` → `src/jellyfin/jfitem.rs`
  - Convert `Movie` → `BaseItemDto`
  - Convert `Show` → `BaseItemDto`
  - Convert `Season` → `BaseItemDto`
  - Convert `Episode` → `BaseItemDto`

**Verification:** Types serialize correctly to Jellyfin JSON format.

---

### Phase 11: Jellyfin API - System and User Endpoints

**Goal:** Port system and user management endpoints.

#### Tasks

- [ ] **11.1 Port system handlers**
  - File: `jellyfin/system.go` → `src/jellyfin/system.rs`
  - `/System/Info` - server info
  - `/System/Info/Public` - public info
  - `/System/Ping` - health check
  - `/Plugins` - empty plugin list

- [ ] **11.2 Port user handlers**
  - File: `jellyfin/user.go` → `src/jellyfin/user.rs`
  - `/Users` - list users
  - `/Users/Me` - current user
  - `/Users/{id}` - get user
  - `/Users/{id}/Views` - library views

**Verification:** Jellyfin client can connect and show libraries.

---

### Phase 12: Jellyfin API - Item Endpoints

**Goal:** Port item browsing and details endpoints.

#### Tasks

- [ ] **12.1 Port item handlers**
  - File: `jellyfin/item.go` → `src/jellyfin/item.rs`
  - `/Items` - query items
  - `/Items/{id}` - get item
  - `/Items/Latest` - latest items
  - `/Items/Counts` - item counts
  - `/Items/Filters` - filter options
  - `/Items/{id}/Images/{type}` - item images
  - `/Items/{id}/PlaybackInfo` - playback info
  - `/Items/{id}/Similar` - similar items

- [ ] **12.2 Port show handlers**
  - File: `jellyfin/show.go` → `src/jellyfin/show.rs`
  - `/Shows/NextUp` - next episodes
  - `/Shows/{id}/Seasons` - list seasons
  - `/Shows/{id}/Episodes` - list episodes

- [ ] **12.3 Port video streaming**
  - `/Videos/{id}/stream` - video streaming

**Verification:** Can browse and play media in Jellyfin client.

---

### Phase 13: Jellyfin API - User Data Endpoints

**Goal:** Port playback state tracking.

#### Tasks

- [ ] **13.1 Port userdata handlers**
  - File: `jellyfin/userdata.go` → `src/jellyfin/userdata.rs`
  - `/Users/{id}/PlayedItems/{id}` - mark played/unplayed
  - `/Users/{id}/FavoriteItems/{id}` - favorites
  - `/Sessions/Playing` - playback started
  - `/Sessions/Playing/Progress` - playback progress
  - `/Sessions/Playing/Stopped` - playback stopped
  - `/Users/{id}/Items/Resume` - resume items

**Verification:** Playback state persists across sessions.

---

### Phase 14: Jellyfin API - Metadata Endpoints

**Goal:** Port metadata browsing endpoints.

#### Tasks

- [ ] **14.1 Port genre handlers**
  - File: `jellyfin/genre.go` → `src/jellyfin/genre.rs`
  - `/Genres` - list genres
  - `/Genres/{name}` - genre details

- [ ] **14.2 Port studio handlers**
  - File: `jellyfin/studio.go` → `src/jellyfin/studio.rs`
  - `/Studios` - list studios
  - `/Studios/{name}` - studio details

- [ ] **14.3 Port person handlers**
  - File: `jellyfin/person.go` → `src/jellyfin/person.rs`
  - `/Persons` - list persons

**Verification:** Can filter by genre, studio, person in client.

---

### Phase 15: Jellyfin API - Additional Endpoints

**Goal:** Port remaining Jellyfin endpoints.

#### Tasks

- [ ] **15.1 Port session handlers**
  - File: `jellyfin/session.go` → `src/jellyfin/session.rs`
  - `/Sessions` - list sessions
  - `/Sessions/Capabilities` - report capabilities

- [ ] **15.2 Port device handlers**
  - File: `jellyfin/device.go` → `src/jellyfin/device.rs`
  - `/Devices` - list/delete devices
  - `/Devices/Info` - device info

- [ ] **15.3 Port playlist handlers**
  - File: `jellyfin/playlist.go` → `src/jellyfin/playlist.rs`
  - `/Playlists` - create playlist
  - `/Playlists/{id}` - get/update playlist
  - `/Playlists/{id}/Items` - playlist items

- [ ] **15.4 Port branding handlers**
  - File: `jellyfin/branding.go` → `src/jellyfin/branding.rs`
  - `/Branding/Configuration`
  - `/Branding/Css`

- [ ] **15.5 Port localization handlers**
  - File: `jellyfin/localization.go` → `src/jellyfin/localization.rs`
  - `/Localization/Countries`
  - `/Localization/Cultures`

**Verification:** Full Jellyfin client compatibility.

---

### Phase 16: Final Compliance & Refactoring

**Goal:** Ensure 1:1 structural and functional compliance for the Jellyfin API.

#### Tasks

- [x] **16.1 Modularize Jellyfin handlers**
  - Rename `handlers.rs` to `jellyfin.rs`
  - Split business logic into dedicated module files (branding, device, item, etc.)
- [x] **16.2 Fix type inference and compilation**
  - Resolve all `cargo check` errors
  - Standardize error handling and response types
- [x] **16.3 Verify structural compliance**
  - Ensure 1:1 match with Go implementation directory structure

**Verification:** `cargo check` passes with zero errors for the Jellyfin module.

---

### Phase 17: Notflix API Compliance & Refactoring

**Goal:** Ensure 1:1 structural and functional compliance for the Notflix API.

#### Tasks

- [x] **17.1 Modularize Notflix handlers**
  - Move logic into `notflix.rs`, `etag.rs`, `proxy.rs`, and `subtitles.rs`
- [x] **17.2 Implement missing Notflix features**
  - Port HLS proxying logic
  - Port subtitle conversion (.srt to .vtt/JSON)
- [x] **17.3 Standardize ETag handling**
  - Implement validation for both file-based and object-based ETags

**Verification:** Notflix API is fully modularized and functional, build is clean.

---

### Phase 18: Integration and Testing

**Goal:** Final integration and testing.

#### Tasks

- [ ] **18.1 Integration testing**
  - Test with Jellyfin web client
  - Test with Jellyfin mobile apps
  - Test with third-party apps (Infuse, etc.)

- [ ] **18.2 Performance testing**
  - Large library scanning
  - Concurrent request handling
  - Memory usage profiling

- [ ] **18.3 Documentation**
  - Update README
  - Configuration examples
  - Deployment guide

**Verification:** Server is production-ready.

---

## Execution Order

The phases should be executed roughly in order, but some parallelization is possible:

1. **Phase 1-2** (Setup) - Sequential
2. **Phase 3** (Database) - Can start after Phase 1
3. **Phase 4-6** (Collection) - Sequential, depends on Phase 2-3
4. **Phase 7** (ImageResize) - Can run parallel to Phase 4-6
5. **Phase 8** (Notflix) - Depends on Phase 4-7
6. **Phase 9-15** (Jellyfin) - Sequential, depends on Phase 4-7
7. **Phase 16-17** (Refactoring) - Final compliance checks
8. **Phase 18** (Testing) - Final

## File Mapping Reference

| Go File | Rust File |
|---------|-----------|
| `server.go` | `src/lib.rs`, `src/bin/main.rs`, `src/server.rs` |
| `httplog.go` | `src/server.rs` (middleware) |
| `idhash/idhash.go` | `src/idhash/mod.rs` |
| `database/database.go` | `src/database/mod.rs` |
| `database/model/model.go` | `src/database/model.rs` |
| `database/sqlite/*.go` | `src/database/sqlite.rs` |
| `collection/collection.go` | `src/collection/collection.rs` |
| `collection/collectionrepo.go` | `src/collection/collectionrepo.rs` |
| `collection/item.go` | `src/collection/item.rs` |
| `collection/kodifs.go` | `src/collection/kodifs.rs` |
| `collection/opendir.go` | `src/collection/opendir.rs` |
| `collection/parsefilename.go` | `src/collection/parsefilename.rs` |
| `collection/metadata/*.go` | `src/collection/metadata/mod.rs` |
| `collection/search/search.go` | `src/collection/search/mod.rs` |
| `imageresize/imageresize.go` | `src/imageresize/mod.rs` |
| `notflix/apihandler.go` | `src/notflix/handlers.rs` |
| `notflix/apitypes.go` | `src/notflix/types.rs` |
| `notflix/etag.go` | `src/notflix/etag.rs` |
| `notflix/proxy.go` | `src/notflix/proxy.rs` |
| `notflix/subtitles.go` | `src/notflix/subtitles.rs` |
| `jellyfin/jellyfin.go` | `src/jellyfin/mod.rs` |
| `jellyfin/type.go` | `src/jellyfin/types.rs` |
| `jellyfin/jfitem.go` | `src/jellyfin/jfitem.rs` |
| `jellyfin/auth.go` | `src/jellyfin/auth.rs` |
| `jellyfin/system.go` | `src/jellyfin/system.rs` |
| `jellyfin/user.go` | `src/jellyfin/user.rs` |
| `jellyfin/item.go` | `src/jellyfin/item.rs` |
| `jellyfin/show.go` | `src/jellyfin/show.rs` |
| `jellyfin/userdata.go` | `src/jellyfin/userdata.rs` |
| `jellyfin/genre.go` | `src/jellyfin/genre.rs` |
| `jellyfin/studio.go` | `src/jellyfin/studio.rs` |
| `jellyfin/person.go` | `src/jellyfin/person.rs` |
| `jellyfin/session.go` | `src/jellyfin/session.rs` |
| `jellyfin/device.go` | `src/jellyfin/device.rs` |
| `jellyfin/playlist.go` | `src/jellyfin/playlist.rs` |
| `jellyfin/branding.go` | `src/jellyfin/branding.rs` |
| `jellyfin/localization.go` | `src/jellyfin/localization.rs` |

## Notes

- Always use `cargo check` to verify changes compile
- Do not use `cargo build` or `cargo run`
- Preserve comments from Go code
- Use `#[serde(rename_all = "PascalCase")]` for Jellyfin API types
- Use `#[serde(skip_serializing_if = "Option::is_none")]` for omitempty fields
- Prefer `..Default::default()` for large struct initialization
