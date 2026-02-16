mod config;
mod middleware;

pub use config::Config;

use axum::{
    response::IntoResponse,
    routing::{delete, get, post},
    Router,
};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tower_http::{compression::CompressionLayer, trace::TraceLayer};
use tracing::info;

use crate::collection::CollectionRepo;
use crate::database::sqlite::SqliteRepository;
use crate::imageresize::ImageResizer;
use crate::jellyfin::{JellyfinAuthState, JellyfinState};
use crate::notflix::NotflixState;

/// Application state shared across all handlers
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub collections: Arc<CollectionRepo>,
    pub repo: Arc<SqliteRepository>,
    pub image_resizer: Arc<ImageResizer>,
    pub debug: bool,
}

/// Main entry point - loads config and starts server
pub async fn run(config_path: String, debug: bool) -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing with JSON formatting
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .json()
        .init();

    info!("Using config file {}", config_path);

    // Load configuration
    let config = Config::from_file(&config_path)?;
    info!("Configuration loaded successfully");

    // Initialize database
    let db_dir = config.dbdir.clone();
    
    // Create directory if it doesn't exist
    if db_dir != "." {
        std::fs::create_dir_all(&db_dir).map_err(|e| format!("Failed to create db dir '{}': {}", db_dir, e))?;
    }

    let db_path = std::path::Path::new(&db_dir).join("tink-items.db");
    let db_path_str = db_path.to_str().ok_or("Invalid database path")?.to_string();

    let repo = Arc::new(SqliteRepository::new(&db_path_str).await?);
    info!("Database initialized at {}", db_path_str);

    // Initialize collection repository
    let collections = Arc::new(CollectionRepo::new());
    info!("Collection repository initialized");

    // Initialize image resizer
    let cache_dir = PathBuf::from(config.cachedir.clone());
    let image_resizer = Arc::new(ImageResizer::new(cache_dir)?);
    info!("Image resizer initialized");

    // Initialize collections from config
    for collection_config in &config.collections {
        collections
            .add_collection(
                collection_config.name.clone(),
                Some(collection_config.id.clone()),
                &collection_config.collection_type,
                collection_config.directory.clone(),
                collection_config.hls_server.clone().unwrap_or_default(),
            )
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    }

    // Scan collections
    collections.init();

    // Start background background scan
    collections.background();

    // Create application state
    let state = AppState {
        config: Arc::new(config),
        collections,
        repo,
        image_resizer,
        debug,
    };

    // Build router
    let app = build_router(state.clone());

    // Determine bind address
    let addr: SocketAddr = format!("{}:{}", state.config.listen.address, state.config.listen.port).parse()?;

    info!("Starting server on {}", addr);

    // Start server with or without TLS
    if let (Some(cert), Some(key)) = (&state.config.listen.tls_cert, &state.config.listen.tls_key) {
        info!("TLS enabled");
        start_tls_server(app, addr, cert, key).await?;
    } else {
        info!("TLS disabled");
        start_server(app, addr).await?;
    }

    Ok(())
}

/// Build the axum router with all routes and middleware
fn build_router(state: AppState) -> Router {
    use axum::middleware as mw;

    // Create Notflix state
    let notflix_state = NotflixState {
        collections: state.collections.clone(),
        image_resizer: state.image_resizer.clone(),
        app_dir: state.config.app_dir().unwrap_or_else(|| "./app".to_string()),
    };

    // Create Jellyfin auth state
    let server_id = state
        .config
        .server_id()
        .unwrap_or_else(|| "jellofin-rs-server".to_string());
    let jellyfin_auth_state = JellyfinAuthState {
        repo: state.repo.clone(),
        server_id: server_id.clone(),
        auto_register: state.config.auto_register().unwrap_or(true),
    };

    // Create Jellyfin API state
    let jellyfin_state = JellyfinState {
        repo: state.repo.clone(),
        collections: state.collections.clone(),
        server_id: server_id.clone(),
        server_name: state.config.server_name().unwrap_or_else(|| "Jellofin-rs".to_string()),
        image_resizer: state.image_resizer.clone(),
        config: state.config.clone(),
    };

    // Notflix API routes (no auth required)
    let notflix_routes = Router::new()
        .route("/api/collections", get(crate::notflix::collections_handler))
        .route("/api/collection/{id}", get(crate::notflix::collection_handler))
        .route("/api/collection/{coll}/items", get(crate::notflix::items_handler))
        .route("/api/collection/{coll}/item/{item}", get(crate::notflix::item_handler))
        .route("/api/collection/{id}/genres", get(crate::notflix::genres_handler))
        .route("/data/{source}/{*path}", get(crate::notflix::data_handler))
        .route("/v/{*path}", get(crate::notflix::index_handler))
        .with_state(notflix_state);

    // Jellyfin public routes (no auth required)
    let jellyfin_public = Router::new()
        .route("/Branding/Configuration", get(crate::jellyfin::branding_configuration))
        .route("/Branding/Css", get(crate::jellyfin::branding_css))
        .route("/Branding/Css.css", get(crate::jellyfin::branding_css))
        .route("/QuickConnect/Enabled", get(crate::jellyfin::quick_connect_enabled))
        .route("/QuickConnect/Initiate", post(crate::jellyfin::quick_connect_initiate))
        .route("/QuickConnect/Connect", get(crate::jellyfin::quick_connect_connect))
        .route("/Users/AuthenticateByName", post(crate::jellyfin::authenticate_by_name))
        .route("/socket", get(crate::jellyfin::socket_handler))
        .route("/", get(crate::jellyfin::root_handler))
        .with_state(jellyfin_auth_state.clone());
    
    // Jellyfin images (Public, uses JellyfinState)
    let jellyfin_images_public = Router::new()
        .route("/Items/{item}/Images/{type}", get(crate::jellyfin::get_item_image))
        .route("/Items/{item}/Images/{type}/{index}", get(crate::jellyfin::get_item_image_indexed))
        .with_state(jellyfin_state.clone());

    // Jellyfin system and user routes (some public, some protected)
    let jellyfin_api = Router::new()
        // Public system routes
        .route("/System/Info/Public", get(crate::jellyfin::system_info_public))
        .route("/System/Ping", get(crate::jellyfin::system_ping))
        .route("/health", get(crate::jellyfin::health))
        // Protected routes
        .merge(
            Router::new()
                // Devices
                .route("/Devices", get(crate::jellyfin::devices_get).delete(crate::jellyfin::devices_delete))
                .route("/Devices/Info", get(crate::jellyfin::devices_info))
                .route("/Devices/Options", get(crate::jellyfin::devices_options))
                // Display preferences.
                // TODO .route("/DisplayPreferences/usersettings", get(super::system::display_preferences))
                // Genre metadata.
                .route("/Genres", get(crate::jellyfin::genres_all))
                .route("/Genres/{name}", get(crate::jellyfin::genre_details))
                // Item routes
                .route("/Items", get(crate::jellyfin::items_query))
                .route("/Items/Counts", get(crate::jellyfin::items_counts))
                .route("/Items/Latest", get(crate::jellyfin::items_latest))
                .route("/Items/Resume", get(crate::jellyfin::items_resume))
                .route("/Items/Suggestions", get(crate::jellyfin::items_suggestions))
                .route("/Items/{item}", get(crate::jellyfin::item_details))
                .route("/Items/{item}/Ancestors", get(crate::jellyfin::item_ancestors))
                .route("/Items/{item}/Similar", get(crate::jellyfin::items_similar))
                // TODO .route("/Items/{item}/PlaybackInfo", get(crate::jellyfin::item_playbackinfo))
                // TODO .route("/Items/{item}/SpecialFeatures", get(crate::jellyfin::item_special_features))
                // TODO .route("/Items/{item}/ThemeSongs", get(crate::jellyfin::item_themesongs))
                // TODO .route("/Items/{item}/Images/{image_type}", get(super::item::get_image))
                // TODO .route("/Items/{item}/Images/{image_type}/{index}", get(super::item::get_image_indexed))
                // TODO .route("/Items/Filters", get(crate::jellyfin::item_filters))
                // TODO .route("/Items/Filters2", get(crate::jellyfin::item_filters2))
                // Library routes
                .route("/Library/VirtualFolders", get(crate::jellyfin::library_virtual_folders))
                // Localization routes
                .route("/Localization/Cultures", get(crate::jellyfin::localization_cultures))
                .route("/Localization/Countries", get(crate::jellyfin::localization_countries))
                .route("/Localization/Options", get(crate::jellyfin::localization_options))
                .route("/Localization/ParentalRatings", get(crate::jellyfin::localization_parental_ratings))
                // Media segment routes.
                .route("/MediaSegments/{item}", get(crate::jellyfin::media_segments_handler))
                // Movie routes
                .route("/Movies/Recommendations", get(crate::jellyfin::movies_recommendations))
                // Person routes.
                .route("/Persons", get(crate::jellyfin::persons_all))
                .route("/Persons/{name}", get(crate::jellyfin::person_details))
                // Playlists
                .route("/Playlists", post(crate::jellyfin::create_playlist))
                .route("/Playlists/{playlist}", get(crate::jellyfin::get_playlist) .post(crate::jellyfin::update_playlist))
                .route("/Playlists/{playlist}/Items", get(crate::jellyfin::get_playlist_items))
                .route("/Playlists/{playlist}/Items", post(crate::jellyfin::add_playlist_items))
                .route("/Playlists/{playlist}/Items", delete(crate::jellyfin::delete_playlist_items))
                .route("/Playlists/{playlist}/Items/", post(crate::jellyfin::add_playlist_items))
                .route("/Playlists/{playlist}/Items/{item}/Move/{index}", get(crate::jellyfin::move_playlist_item))
                .route("/Playlists/{playlist}/Users", get(crate::jellyfin::get_playlist_all_users))
                .route("/Playlists/{playlist}/Users/{user}", get(crate::jellyfin::get_playlist_users))
                // Plugin routes.
                .route("/Plugins", get(crate::jellyfin::plugins))
                // Search / Hints
                .route("/Search/Hints", get(crate::jellyfin::search_hints))
                // Sessions.
                .route("/Sessions", get(crate::jellyfin::sessions))
                .route("/Sessions/Capabilities", post(crate::jellyfin::sessions_capabilities))
                .route("/Sessions/Capabilities/Full", post(crate::jellyfin::sessions_capabilities_full))
                .route("/Sessions/Playing", post(crate::jellyfin::sessions_playing))
                .route("/Sessions/Playing/Progress", post(crate::jellyfin::sessions_playing_progress))
                .route("/Sessions/Playing/Stopped", post(crate::jellyfin::sessions_playing_stopped))
                // Playing items
                // TODO .route("/PlayingItems/{item}", delete(super::userdata::delete_playing_item))
                // Show routes.
                .route("/Shows/NextUp", get(crate::jellyfin::shows_next_up))
                .route("/Shows/{id}/Episodes", get(crate::jellyfin::show_episodes))
                .route("/Shows/{id}/Seasons", get(crate::jellyfin::show_seasons))
                // Studios.
                .route("/Studios", get(crate::jellyfin::studios_all))
                .route("/Studios/{name}", get(crate::jellyfin::studio_details))
                // System.
                .route("/System/Info", get(crate::jellyfin::system_info))
                // Users.
                .route("/Users", get(crate::jellyfin::users_all))
                .route("/Users/Me", get(crate::jellyfin::users_me))
                .route("/Users/Public", get(crate::jellyfin::users_public))
                .route("/Users/{id}", get(crate::jellyfin::users_by_id))
                .route("/Users/{id}/Views", get(crate::jellyfin::user_views))
                // TODO .route("/Users/{id}/Images/{image_type}", get(super::user::get_user_image))
                .route("/Users/{user}/FavoriteItems/{item}", post(crate::jellyfin::user_favorite_items_post))
                .route("/Users/{user}/FavoriteItems/{item}", delete(crate::jellyfin::user_favorite_items_delete))
                .route("/Users/{user}/Items", get(crate::jellyfin::items_query))
                .route("/Users/{user}/Items/Latest", get(crate::jellyfin::items_latest))
                .route("/Users/{user}/Items/Resume", get(crate::jellyfin::items_resume))
                .route("/Users/{user}/Items/Suggestions", get(crate::jellyfin::items_suggestions))
                .route("/Users/{user}/Items/{item}", get(crate::jellyfin::item_details))
                .route("/Users/{user}/Items/{item}/Similar", get(crate::jellyfin::items_similar))
                .route("/Users/{user}/Items/{item}/UserData", get(crate::jellyfin::users_item_userdata))
                .route("/Users/{user}/Items/Filters", get(crate::jellyfin::item_filters))
                .route("/Users/{user}/Items/Filters2", get(crate::jellyfin::item_filters2))
                .route("/Users/{user}/PlayedItems/{item}", post(crate::jellyfin::users_played_items_post))
                .route("/Users/{user}/PlayedItems/{item}", delete(crate::jellyfin::users_played_items_delete))
                // TODO .route("/Users/{user}/PlayingItems/{item}/Progress", post(super::userdata::update_playback_position))
                // Authenticated QuickConnect route.
                .route("/QuickConnect/Authorize", post(crate::jellyfin::quick_connect_authorize))
                // Video routes
                // TODO .route("/Videos/{id}/{index}/Subtitles", get(super::video::stream_subtitle))
                // TODO .route("/Videos/{id}/Subtitles/{index}/Stream", get(super::video::stream_subtitle))
                .route("/Videos/{item}/stream", get(crate::jellyfin::video_stream_handler))
                .route("/Videos/{item}/stream.{container}", get(crate::jellyfin::video_stream_handler))
                .route("/videos/{item}/stream", get(crate::jellyfin::video_stream_handler))
                .route("/videos/{item}/stream.{container}", get(crate::jellyfin::video_stream_handler))
                // Legacy/Alias Routes
                .route("/UserViews", get(crate::jellyfin::user_views_query))
                // TODO .route("/UserViews/GroupingOptions", get(super::user::get_grouping_options))
                .route("/UserItems/Resume", get(crate::jellyfin::items_resume)) // Alias for legacy/specific clients
                .route("/UserItems/{item}/UserData", get(crate::jellyfin::users_item_userdata_simple))
                // TODO .route("/UserItems/{item}/UserData", post(crate::jellyfin::users_item_userdata_simple))
                .route("/UserFavoriteItems/{item}", post(crate::jellyfin::user_favorite_items_post_simple))
                .route("/UserFavoriteItems/{item}", delete(crate::jellyfin::user_favorite_items_delete_simple))
                .route("/UserPlayedItems/{item}", post(crate::jellyfin::users_played_items_post_simple))
                .route("/UserPlayedItems/{item}", delete(crate::jellyfin::users_played_items_delete_simple))
                // Authentication layer.
                .layer(mw::from_fn_with_state(
                    jellyfin_auth_state,
                    crate::jellyfin::auth_middleware,
                )),
        )
        .with_state(jellyfin_state);

    // Combine all routes
    Router::new()
        .route("/robots.txt", get(robots_handler))
        .merge(notflix_routes)
        .merge(jellyfin_public)
        .merge(jellyfin_images_public)
        .merge(jellyfin_api)
        // Apply global middleware
        .layer(mw::from_fn(middleware::normalize_path_middleware))
        .layer(mw::from_fn(middleware::add_cors_headers_middleware))
        .layer(mw::from_fn(middleware::etag_validation_middleware))
        .layer(mw::from_fn_with_state(state, middleware::log_request_middleware))
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
}

/// Start HTTP server
async fn start_server(app: Router, addr: SocketAddr) -> Result<(), Box<dyn std::error::Error>> {
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

/// Start HTTPS server with TLS
async fn start_tls_server(
    app: Router,
    addr: SocketAddr,
    cert_path: &str,
    key_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    use axum_server::tls_rustls::RustlsConfig;

    let config = RustlsConfig::from_pem_file(cert_path, key_path).await?;

    axum_server::bind_rustls(addr, config)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}

/// Robots.txt handler
async fn robots_handler() -> impl IntoResponse {
    "User-agent: *\nDisallow: /\n"
}
