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
use crate::database::Repository;
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

    tracing_subscriber::fmt().with_env_filter(filter).json().init();

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
    repo.start_background_jobs();
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
    let router = build_router(state.clone());

    // Determine bind address
    let addr: SocketAddr = format!("{}:{}", state.config.listen.address, state.config.listen.port).parse()?;

    info!("Starting server on {}", addr);

    // Wrap with normalize middleware BEFORE routing so URI rewriting affects route matching.
    // Router::layer() runs middleware AFTER routing, which is too late for path normalization.

    // Start server with or without TLS
    if let (Some(cert), Some(key)) = (&state.config.listen.tls_cert, &state.config.listen.tls_key) {
        info!("TLS enabled");
        let app = middleware::NormalizePathService::new(router);

        use axum_server::tls_rustls::RustlsConfig;
        let config = RustlsConfig::from_pem_file(cert, key).await?;
        axum_server::bind_rustls(addr, config)
            .serve(axum::ServiceExt::<axum::http::Request<axum::body::Body>>::into_make_service(app))
            .await?;
    } else {
        info!("TLS disabled");
        let app = middleware::NormalizePathService::new(router);

        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(
            listener,
            axum::ServiceExt::<axum::http::Request<axum::body::Body>>::into_make_service(app),
        )
        .await?;
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
        quick_connect: state.config.quick_connect().unwrap_or(false),
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
    #[rustfmt::skip]
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
    #[rustfmt::skip]
    let jellyfin_public = Router::new()
        .route("/branding/configuration", get(crate::jellyfin::branding_configuration))
        .route("/branding/css", get(crate::jellyfin::branding_css))
        .route("/branding/css.css", get(crate::jellyfin::branding_css))
        .route("/quickconnect/enabled", get(crate::jellyfin::quick_connect_enabled))
        .route("/quickconnect/initiate", post(crate::jellyfin::quick_connect_initiate))
        .route("/quickconnect/connect", get(crate::jellyfin::quick_connect_connect))
        .route("/users/authenticatebyname", post(crate::jellyfin::authenticate_by_name))
        .route("/users/authenticatewithquickconnect", post(crate::jellyfin::authenticate_with_quick_connect))
        .route("/socket", get(crate::jellyfin::socket_handler))
        .route("/", get(crate::jellyfin::root_handler))
        .with_state(jellyfin_auth_state.clone());

    // Jellyfin images (Public, uses JellyfinState)
    #[rustfmt::skip]
    let jellyfin_images_public = Router::new()
        .route("/items/{item}/images/{type}", get(crate::jellyfin::get_item_image))
        .route("/items/{item}/images/{type}/{index}", get(crate::jellyfin::get_item_image_indexed))
        .route("/users/{id}/images/{type}", get(crate::jellyfin::get_user_image))
        .route("/genres/{name}/images/{type}", get(crate::jellyfin::get_genre_image))
        .route("/studios/{name}/images/{type}", get(crate::jellyfin::get_studio_image))
        .route("/persons/{name}/images/{type}", get(crate::jellyfin::get_person_image))
        .with_state(jellyfin_state.clone());

    // Jellyfin system and user routes (some public, some protected)
    #[rustfmt::skip]
    let jellyfin_api = Router::new()
        // Public system routes
        .route("/system/info/public", get(crate::jellyfin::system_info_public))
        .route("/system/ping", get(crate::jellyfin::system_ping))
        .route("/health", get(crate::jellyfin::health))
        .route("/getutctime", get(crate::jellyfin::get_utc_time))
        // Protected routes
        .merge(
            Router::new()
                // Devices
                .route("/devices", get(crate::jellyfin::devices_get).delete(crate::jellyfin::devices_delete))
                .route("/devices/info", get(crate::jellyfin::devices_info))
                .route("/devices/options", get(crate::jellyfin::devices_options))
                // Display preferences.
                .route("/displaypreferences/{id}", get(crate::jellyfin::display_preferences).post(crate::jellyfin::display_preferences))
                // Genre metadata.
                .route("/genres", get(crate::jellyfin::genres_all))
                .route("/genres/{name}", get(crate::jellyfin::genre_details))
                // Item routes
                .route("/items", get(crate::jellyfin::items_query))
                .route("/items/root", get(crate::jellyfin::items_root))
                .route("/items/counts", get(crate::jellyfin::items_counts))
                .route("/items/latest", get(crate::jellyfin::items_latest))
                .route("/items/resume", get(crate::jellyfin::items_resume))
                .route("/items/suggestions", get(crate::jellyfin::items_suggestions))
                .route("/items/filters", get(crate::jellyfin::item_filters))
                .route("/items/filters2", get(crate::jellyfin::item_filters2))
                .route("/items/{item}", get(crate::jellyfin::item_details).delete(crate::jellyfin::items_delete))
                .route("/items/{item}/ancestors", get(crate::jellyfin::item_ancestors))
                .route("/items/{item}/intros", get(crate::jellyfin::items_intros))
                .route("/items/{item}/localtrailers", get(crate::jellyfin::items_local_trailers))
                .route("/items/{item}/thememedia", get(crate::jellyfin::items_theme_media))
                .route("/items/{item}/refresh", post(crate::jellyfin::items_refresh))
                .route("/items/{item}/remoteimages", get(crate::jellyfin::items_remote_images))
                .route("/items/{item}/images/{type}", post(crate::jellyfin::post_item_image).delete(crate::jellyfin::delete_item_image))
                .route("/items/{item}/playbackinfo", get(crate::jellyfin::items_playback_info).post(crate::jellyfin::items_playback_info))
                .route("/items/{item}/similar", get(crate::jellyfin::items_similar))
                .route("/items/{item}/specialfeatures", get(crate::jellyfin::items_special_features))
                // Library routes
                .route("/library/virtualfolders", get(crate::jellyfin::library_virtual_folders))
                .route("/library/mediafolders", get(crate::jellyfin::library_media_folders))
                .route("/library/refresh", post(crate::jellyfin::library_refresh))
                // Localization routes
                .route("/localization/cultures", get(crate::jellyfin::localization_cultures))
                .route("/localization/countries", get(crate::jellyfin::localization_countries))
                .route("/localization/options", get(crate::jellyfin::localization_options))
                .route("/localization/parentalratings", get(crate::jellyfin::localization_parental_ratings))
                // Media segment routes.
                .route("/mediasegments/{item}", get(crate::jellyfin::media_segments_handler))
                // Movie routes
                .route("/movies/recommendations", get(crate::jellyfin::movies_recommendations))
                // Person routes.
                .route("/persons", get(crate::jellyfin::persons_all))
                .route("/persons/{name}", get(crate::jellyfin::person_details))
                // Playlists
                .route("/playlists", post(crate::jellyfin::create_playlist))
                .route("/playlists/{playlist}", get(crate::jellyfin::get_playlist) .post(crate::jellyfin::update_playlist))
                .route("/playlists/{playlist}/items", get(crate::jellyfin::get_playlist_items))
                .route("/playlists/{playlist}/items", post(crate::jellyfin::add_playlist_items))
                .route("/playlists/{playlist}/items", delete(crate::jellyfin::delete_playlist_items))
                .route("/playlists/{playlist}/items/", post(crate::jellyfin::add_playlist_items))
                .route("/playlists/{playlist}/items/{item}/move/{index}", get(crate::jellyfin::move_playlist_item))
                .route("/playlists/{playlist}/users", get(crate::jellyfin::get_playlist_all_users))
                .route("/playlists/{playlist}/users/{user}", get(crate::jellyfin::get_playlist_users))
                // Plugin routes.
                .route("/plugins", get(crate::jellyfin::plugins))
                // Search / Hints
                .route("/search/hints", get(crate::jellyfin::search_hints))
                // Sessions.
                .route("/sessions", get(crate::jellyfin::sessions))
                .route("/sessions/capabilities", get(crate::jellyfin::sessions_capabilities).post(crate::jellyfin::sessions_capabilities))
                .route("/sessions/capabilities/full", get(crate::jellyfin::sessions_capabilities_full).post(crate::jellyfin::sessions_capabilities_full))
                .route("/sessions/playing", post(crate::jellyfin::sessions_playing))
                .route("/sessions/playing/progress", post(crate::jellyfin::sessions_playing_progress))
                .route("/sessions/playing/stopped", post(crate::jellyfin::sessions_playing_stopped))
                // Playing items
                // TODO .route("/playingitems/{item}", delete(super::userdata::delete_playing_item))
                // Show routes.
                .route("/shows/nextup", get(crate::jellyfin::shows_next_up))
                .route("/shows/{id}/episodes", get(crate::jellyfin::show_episodes))
                .route("/shows/{id}/seasons", get(crate::jellyfin::show_seasons))
                // Studios.
                .route("/studios", get(crate::jellyfin::studios_all))
                .route("/studios/{name}", get(crate::jellyfin::studio_details))
                // System.
                .route("/system/info", get(crate::jellyfin::system_info))
                .route("/system/endpoint", get(crate::jellyfin::system_endpoint))
                .route("/system/logs", get(crate::jellyfin::system_logs))
                .route("/system/restart", post(crate::jellyfin::system_restart))
                .route("/system/shutdown", post(crate::jellyfin::system_shutdown))
                .route("/scheduledtasks", get(crate::jellyfin::scheduled_tasks))
                .route("/playback/bitratetest", get(crate::jellyfin::playback_bitrate_test))
                // SyncPlay stubs
                .route("/syncplay/list", get(crate::jellyfin::sync_play_list))
                .route("/syncplay/new", post(crate::jellyfin::sync_play_new))
                // Users.
                .route("/users", get(crate::jellyfin::users_all).post(crate::jellyfin::users_update))
                .route("/users/me", get(crate::jellyfin::users_me))
                .route("/users/new", post(crate::jellyfin::users_new))
                .route("/users/password", post(crate::jellyfin::users_password))
                .route("/users/public", get(crate::jellyfin::users_public))
                .route("/users/{id}", get(crate::jellyfin::users_by_id).delete(crate::jellyfin::users_delete))
                .route("/users/{id}/configuration", post(crate::jellyfin::users_configuration_post))
                .route("/users/{id}/policy", post(crate::jellyfin::users_policy_post))
                .route("/userimage", post(crate::jellyfin::post_user_image).delete(crate::jellyfin::delete_user_image))
                .route("/genres/{name}/images/{type}", post(crate::jellyfin::post_genre_image))
                .route("/studios/{name}/images/{type}", post(crate::jellyfin::post_studio_image))
                .route("/persons/{name}/images/{type}", post(crate::jellyfin::post_person_image))
                .route("/users/{id}/views", get(crate::jellyfin::user_views))
                .route("/users/{id}/groupingoptions", get(crate::jellyfin::user_grouping_options))
                .route("/users/{user}/favoriteitems/{item}", post(crate::jellyfin::user_favorite_items_post))
                .route("/users/{user}/favoriteitems/{item}", delete(crate::jellyfin::user_favorite_items_delete))
                .route("/users/{user}/items", get(crate::jellyfin::items_query))
                .route("/users/{user}/items/latest", get(crate::jellyfin::items_latest))
                .route("/users/{user}/items/resume", get(crate::jellyfin::items_resume))
                .route("/users/{user}/items/suggestions", get(crate::jellyfin::items_suggestions))
                .route("/users/{user}/items/{item}", get(crate::jellyfin::item_details))
                .route("/users/{user}/items/{item}/similar", get(crate::jellyfin::items_similar))
                .route("/users/{user}/items/{item}/userdata", get(crate::jellyfin::users_item_userdata))
                .route("/users/{user}/items/filters", get(crate::jellyfin::item_filters))
                .route("/users/{user}/items/filters2", get(crate::jellyfin::item_filters2))
                .route("/users/{user}/playeditems/{item}", post(crate::jellyfin::users_played_items_post))
                .route("/users/{user}/playeditems/{item}", delete(crate::jellyfin::users_played_items_delete))
                // TODO .route("/users/{user}/playingitems/{item}/progress", post(super::userdata::update_playback_position))
                // Authenticated QuickConnect route.
                .route("/quickconnect/authorize", post(crate::jellyfin::quick_connect_authorize))
                // Video routes
                // TODO .route("/videos/{id}/{index}/subtitles", get(super::video::stream_subtitle))
                // TODO .route("/videos/{id}/subtitles/{index}/stream", get(super::video::stream_subtitle))
                .route("/videos/{item}/stream", get(crate::jellyfin::video_stream_handler))
                .route("/videos/{item}/stream.{container}", get(crate::jellyfin::video_stream_handler))
                // Legacy/Alias Routes
                .route("/userviews", get(crate::jellyfin::user_views_query))
                .route("/userviews/groupingoptions", get(crate::jellyfin::user_grouping_options))
                .route("/useritems/resume", get(crate::jellyfin::items_resume)) // Alias for legacy/specific clients
                .route("/useritems/{item}/userdata", get(crate::jellyfin::users_item_userdata_simple))
                // TODO .route("/useritems/{item}/userdata", post(crate::jellyfin::users_item_userdata_simple))
                .route("/userfavoriteitems/{item}", post(crate::jellyfin::user_favorite_items_post_simple))
                .route("/userfavoriteitems/{item}", delete(crate::jellyfin::user_favorite_items_delete_simple))
                .route("/userplayeditems/{item}", post(crate::jellyfin::users_played_items_post_simple))
                .route("/userplayeditems/{item}", delete(crate::jellyfin::users_played_items_delete_simple))
                // Authentication layer.
                .layer(mw::from_fn_with_state(
                    jellyfin_auth_state,
                    crate::jellyfin::auth_middleware,
                )),
        )
        .with_state(jellyfin_state);

    // Combine all routes
    #[rustfmt::skip]
    Router::new()
        .route("/robots.txt", get(robots_handler))
        .merge(notflix_routes)
        .merge(jellyfin_public)
        .merge(jellyfin_images_public)
        .merge(jellyfin_api)
        // Apply global middleware
        .layer(mw::from_fn_with_state(state.clone(), middleware::ip_acl_middleware))
        .layer(mw::from_fn(middleware::add_cors_headers_middleware))
        .layer(mw::from_fn(middleware::etag_validation_middleware))
        .layer(mw::from_fn_with_state(state, middleware::log_request_middleware))
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
}

/// Robots.txt handler
async fn robots_handler() -> impl IntoResponse {
    "User-agent: *\nDisallow: /\n"
}
