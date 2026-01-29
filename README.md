# Jellofin-rs

A Jellyfin-compatible media server written in Rust, ported from the Go implementation.

## Features

- **Jellyfin API** - Compatible with official Jellyfin clients
- **Notflix API** - Legacy custom API support
- **Media Library** - Automatic scanning of movies and TV shows
- **Full-text Search** - Fast search using Tantivy
- **Image Resizing** - On-demand image processing with caching
- **User Management** - Authentication and playback state tracking
- **Playlists** - Create and manage media playlists

## Building

```bash
cargo build --release
```

## Configuration

Copy the example configuration file:

```bash
cp jellofin-server.example.yaml jellofin-server.yaml
```

Edit `jellofin-server.yaml` to configure:
- Server address and port
- Media collection directories
- Database location
- TLS certificates (optional)

## Running

```bash
cargo run --release -- --config jellofin-server.yaml
```

Or using the built binary:

```bash
./target/release/jellofin-server --config jellofin-server.yaml
```

## Project Structure

```
src/
├── lib.rs              # Library entry point
├── bin/main.rs         # Binary entry point
├── collection/         # Media scanning and management
├── database/           # SQLite persistence layer
├── idhash/             # ID generation utilities
├── imageresize/        # Image processing
├── jellyfin/           # Jellyfin API handlers
├── notflix/            # Notflix API handlers
└── server/             # HTTP server and configuration
```

## Documentation

See `Documentation/` directory for:
- `ARCHITECTURE.md` - System architecture overview
- `PLAN.md` - Porting guidelines
- `project-plan.md` - Implementation phases

## License

TBD
