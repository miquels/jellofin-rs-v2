# Jellofin-rs Porting Progress

## Current Status: ~16% Complete (6/16 phases)

### ✅ Completed Phases

#### Phase 1: Project Setup and Core Infrastructure
- **Files Created:**
  - `Cargo.toml` - All dependencies (axum 0.8, tokio, sqlx, tantivy, image, etc.)
  - `src/lib.rs` - Module exports
  - `src/bin/main.rs` - CLI with clap
  - `src/server.rs` - HTTP server with axum
  - `src/server/config.rs` - YAML configuration
  - `src/server/middleware.rs` - Path normalization and logging
  - `README.md`, `.gitignore`, `jellofin-server.example.yaml`

- **Status:** ✅ Complete, `cargo check` passes

#### Phase 2: ID Hash Module
- **Files Created:**
  - `src/idhash/mod.rs` - SHA256-based 20-char ID generation

- **Tests:** 4/4 passing
- **Status:** ✅ Complete

#### Phase 3: Database Module
- **Files Created:**
  - `src/database/mod.rs` - Repository traits (async)
  - `src/database/model.rs` - Data models (User, AccessToken, UserData, Playlist, Item)
  - `src/database/sqlite.rs` - SQLite implementation with pooling and caching

- **Features:**
  - Async repository traits with async_trait
  - SQLite connection pooling (sqlx)
  - In-memory caching for access tokens and user data
  - Background sync job (10-second interval)
  - Schema initialization

- **Status:** ✅ Complete, `cargo check` passes

#### Phase 4: Collection Item Types
- **Files Created:**
  - `src/collection/item.rs` - Movie, Show, Season, Episode structs
  - `src/collection/metadata.rs` - Metadata stub

- **Key Changes:**
  - Item and ItemRef enums (replaces Go interfaces)
  - Public fields (no accessor methods - Rust idiom)
  - Helper methods only for computed values
  - Subtitle types (Subs, Subtitles)

- **Tests:** 2/2 passing
- **Status:** ✅ Complete

#### Phase 5a: Filename Parsing
- **Files Created:**
  - `src/collection/parsefilename.rs` - Episode filename parsing

- **Patterns Supported:**
  - S01E04 (standard)
  - S01E04E05 (double episodes)
  - 2015.03.08 (date-based)
  - 3x08 or 308 (compact)

- **Tests:** 7/7 passing
- **Status:** ✅ Complete

#### Phase 5b: Collection Types
- **Files Created:**
  - `src/collection/collection.rs` - Collection struct and CollectionType enum

- **Features:**
  - Collection management
  - CollectionDetails aggregation
  - Genre counting
  - Uses HashSet for deduplication

- **Tests:** 4/4 passing
- **Status:** ✅ Complete

### 📊 Statistics
- **Total tests:** 21/21 passing ✅
- **Lines ported:** ~1,814 / 11,127 (~16%)
- **Compilation:** Clean (no warnings)

---

## 🔄 Next Steps: Phase 5c - CollectionRepo and Filesystem Scanning

### Files to Port

1. **`collection/collectionrepo.go` (437 lines) → `src/collection/collectionrepo.rs`**
   - CollectionRepo struct
   - Methods: new(), add_collection(), init(), background()
   - update_collections() - triggers scanning
   - Item lookup: get_item(), get_item_by_id(), get_episode_by_id()
   - next_up() logic for TV shows
   - build_search_index() integration

2. **`collection/kodifs.go` (579 lines) → `src/collection/kodifs.rs`**
   - build_movies() - Scan movie directories
   - build_shows() - Scan TV show directories
   - Image detection (poster, fanart, banner, logo)
   - NFO file detection and parsing
   - Subtitle detection (.srt, .vtt)
   - Video file detection

3. **`collection/opendir.go` (135 lines) → `src/collection/opendir.rs`**
   - Directory enumeration helpers
   - File filtering utilities

### Key Implementation Notes

**CollectionRepo:**
- Use `ArcSwap<Vec<Collection>>` for thread-safe collection updates
- Background scanning with tokio::spawn
- Integration with search index (Phase 6)

**Filesystem Scanning:**
- Use `std::fs` and `walkdir` crate for directory traversal
- Async file I/O where beneficial
- Pattern matching for file types (video, images, subtitles, NFO)
- Kodi-style directory structure:
  ```
  /movies/Movie Name (2024)/
    movie.nfo
    Movie Name.mkv
    poster.jpg
    fanart.jpg
  
  /shows/Show Name/
    tvshow.nfo
    Season 01/
      S01E01.mkv
      S01E01.nfo
      S01E01-thumb.jpg
  ```

**Dependencies to Add:**
- `walkdir` - Directory traversal
- `arc-swap` - Already in Cargo.toml

### Phase 6: Search Module (~408 lines)
- `collection/search/search.go` → `src/collection/search/mod.rs`
- Tantivy integration
- SearchDocument struct
- search() and similar() methods

### Phase 7: Image Resizer (~344 lines)
- `imageresize/imageresize.go` → `src/imageresize/mod.rs`
- On-demand resizing with caching
- JPEG/PNG support
- Cache key generation

---

## 🔜 Remaining Phases

### Phase 8-10: Partially complete

- session was stopped half way
- need to check which phases are complete
- might be done or mostly done.

### Phase 11-15: API Modules (~5,400 lines)
**Jellyfin API:**
- auth.go -> auth.rs (authentication)
- branding.go -> branding.rs (branding config)
- device.go -> device.rs (device management)
- error.go -> error.rs
- genre.go -> genre.rs (genre filtering)
- item.go -> item.rs (item endpoints)
- jfitem.go -> jfitem.rs (item conversion)
- library.go -> library.rs
- localization.go -> localization.rs (localization)
- movie.go -> movie.rs
- person.go -> person.rs (actor/person endpoints)
- playlist.go -> playlist.rs (playlists)
- session.go -> session.rs (session management)
- show.go -> show.rs (TV show endpoints)
- studio.go -> studio.rs (studio filtering)
- system.go -> system.rs (system info)
- type.go -> types.rs (Jellyfin API types)
- user.go -> user.rs (user management)
- userdata.go -> userdata.rs (playback state)

**Notflix API:**
- apihandler.go → handlers.rs
- apitypes.go → types.rs
- etag.go → etag.rs
- proxy.go → proxy.rs
- subtitles.go → subtitles.rs

### Phase 16: Integration and Testing
- End-to-end testing with Jellyfin clients
- Performance testing
- Documentation updates

---

## 📝 Important Patterns and Conventions

### Naming Conventions
- **Structs:** PascalCase (Movie, Show, Collection)
- **Fields:** snake_case (season_no, file_name)
- **Serde:** `#[serde(rename_all = "PascalCase")]` for Jellyfin API
- **Functions:** snake_case (parse_episode_name, id_hash)

### Rust Idioms Applied
- Public fields instead of getters (data immutable by default)
- Enums instead of interfaces (Item, ItemRef)
- `Option<T>` instead of nil
- `Result<T, E>` for error handling
- `Arc` for shared ownership
- `ArcSwap` for lock-free updates
- `async/await` for I/O operations

### Error Handling
- Use `thiserror` for error types
- Implement `IntoResponse` for axum handlers
- Propagate errors with `?` operator

### Testing
- Unit tests in same file with `#[cfg(test)]`
- Integration tests in `tests/` directory
- Run with `cargo test`

---

## 🎯 Success Criteria

- [ ] All Go code ported to Rust
- [ ] All tests passing
- [ ] `cargo check` passes with no warnings
- [ ] Compatible with Jellyfin clients (web, mobile, TV)
- [ ] Performance comparable or better than Go version
- [ ] Documentation complete

---

## 📚 References

- **Go Source:** `/Users/miquel/Devel/jellofin-rs/jellofin-go/`
- **Rust Target:** `/Users/miquel/Devel/jellofin-rs/src/`
- **Documentation:** `/Users/miquel/Devel/jellofin-rs/Documentation/`
  - `PLAN.md` - Porting guidelines
  - `ARCHITECTURE.md` - System architecture
  - `project-plan.md` - Detailed phase breakdown
  - `PROGRESS.md` - This file

---

**Last Updated:** Phase 5b complete, ready to start Phase 5c (CollectionRepo and filesystem scanning)
