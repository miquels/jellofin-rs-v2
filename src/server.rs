mod config;
mod middleware;

pub use config::Config;

use axum::{
    Router,
    routing::{get, post},
    response::IntoResponse,
};
use std::net::SocketAddr;
use std::sync::Arc;
use std::path::PathBuf;
use tower_http::{
    trace::TraceLayer,
    compression::CompressionLayer,
};
use tracing::info;

use crate::collection::CollectionRepo;
use crate::database::sqlite::SqliteRepository;
use crate::imageresize::ImageResizer;
use crate::notflix::NotflixState;
use crate::jellyfin::{JellyfinAuthState, JellyfinState};

/// Application state shared across all handlers
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub collections: Arc<CollectionRepo>,
    pub repo: Arc<SqliteRepository>,
    pub image_resizer: Arc<ImageResizer>,
}

/// Main entry point - loads config and starts server
pub async fn run(config_path: String) -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"))
        )
        .init();

    info!("Using config file {}", config_path);

    // Load configuration
    let config = Config::from_file(&config_path)?;
    info!("Configuration loaded successfully");

    // Initialize database
    let db_path = config.database.path.clone().unwrap_or_else(|| "jellofin.db".to_string());
    let repo = Arc::new(SqliteRepository::new(&db_path).await?);
    info!("Database initialized at {}", db_path);

    // Initialize collection repository
    let collections = Arc::new(CollectionRepo::new());
    info!("Collection repository initialized");

    // Initialize image resizer
    let cache_dir = PathBuf::from(config.cachedir.clone().unwrap_or_else(|| "./cache".to_string()));
    let image_resizer = Arc::new(ImageResizer::new(cache_dir)?);
    info!("Image resizer initialized");

    // TODO: Initialize collections from config
    // for collection_config in &config.collections {
    //     collections.add_collection(...);
    // }

    // Create application state
    let state = AppState {
        config: Arc::new(config),
        collections,
        repo,
        image_resizer,
    };

    // Build router
    let app = build_router(state.clone());

    // Determine bind address
    let addr: SocketAddr = format!("{}:{}", 
        state.config.listen.address, 
        state.config.listen.port
    ).parse()?;

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
    let server_id = state.config.server_id().unwrap_or_else(|| "jellofin-rs-server".to_string());
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
    };

    // Notflix API routes (no auth required)
    let notflix_routes = Router::new()
        .route("/api/collections", get(crate::notflix::collections_handler))
        .route("/api/collection/:id", get(crate::notflix::collection_handler))
        .route("/api/collection/:coll/items", get(crate::notflix::items_handler))
        .route("/api/collection/:coll/item/:item", get(crate::notflix::item_handler))
        .route("/api/collection/:id/genres", get(crate::notflix::genres_handler))
        .with_state(notflix_state);

    // Jellyfin public routes (no auth required)
    let jellyfin_public = Router::new()
        .route("/Users/AuthenticateByName", post(crate::jellyfin::authenticate_by_name))
        .route("/QuickConnect/Enabled", get(crate::jellyfin::quick_connect_enabled))
        .with_state(jellyfin_auth_state.clone());

    // Jellyfin system and user routes (some public, some protected)
    let jellyfin_api = Router::new()
        // Public system routes
        .route("/System/Info/Public", get(crate::jellyfin::system_info_public))
        .route("/System/Ping", get(crate::jellyfin::system_ping))
        .route("/health", get(crate::jellyfin::health))
        // Protected routes
        .nest("/", Router::new()
            .route("/System/Info", get(crate::jellyfin::system_info))
            .route("/Users", get(crate::jellyfin::users_all))
            .route("/Users/Me", get(crate::jellyfin::users_me))
            .route("/Users/:id", get(crate::jellyfin::users_by_id))
            .route("/Users/:id/Views", get(crate::jellyfin::user_views))
            .route("/Users/Public", get(crate::jellyfin::users_public))
            .route("/Plugins", get(crate::jellyfin::plugins))
            .route("/Branding/Configuration", get(crate::jellyfin::branding_configuration))
            
            // Item routes
            .route("/Items", get(crate::jellyfin::items_query))
            .route("/Items/Latest", get(crate::jellyfin::items_latest))
            .route("/Items/Counts", get(crate::jellyfin::items_counts))
            .route("/Items/Suggestions", get(crate::jellyfin::items_suggestions))
            .route("/Items/Resume", get(crate::jellyfin::items_resume))
            .route("/Items/:item", get(crate::jellyfin::item_details))
            .route("/Items/:item/Similar", get(crate::jellyfin::items_similar))
            .route("/Items/:item/Ancestors", get(crate::jellyfin::item_ancestors))
            
            // Search / Hints
            .route("/Search/Hints", get(crate::jellyfin::search_hints))
            
            // Show routes
            .route("/Shows/NextUp", get(crate::jellyfin::shows_next_up))
            .route("/Shows/:id/Seasons", get(crate::jellyfin::show_seasons))
            .route("/Shows/:id/Episodes", get(crate::jellyfin::show_episodes))
            
            // User-prefixed routes
            .route("/Users/:user/Items", get(crate::jellyfin::items_query))
            .route("/Users/:user/Items/Latest", get(crate::jellyfin::items_latest))
            .route("/Users/:user/Items/Resume", get(crate::jellyfin::items_resume))
            .route("/Users/:user/Items/Suggestions", get(crate::jellyfin::items_suggestions))
            .route("/Users/:user/Items/:item", get(crate::jellyfin::item_details))
            .route("/Users/:user/Items/:item/Similar", get(crate::jellyfin::items_similar))
            .route("/Users/:user/Items/Filters", get(crate::jellyfin::item_filters))
            .route("/Users/:user/Items/Filters2", get(crate::jellyfin::item_filters2))
            
            .layer(mw::from_fn_with_state(
                jellyfin_auth_state,
                crate::jellyfin::auth_middleware
            ))
        )
        .with_state(jellyfin_state);

    // Combine all routes
    Router::new()
        .route("/robots.txt", get(robots_handler))
        .merge(notflix_routes)
        .merge(jellyfin_public)
        .merge(jellyfin_api)
        // Apply global middleware
        .layer(mw::from_fn(middleware::normalize_path_middleware))
        .layer(mw::from_fn(middleware::log_request_middleware))
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
