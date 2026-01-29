use axum::{
    extract::Request,
    middleware::Next,
    response::Response,
};
use tracing::info;

/// Middleware to normalize request paths
/// 1. Removes redundant slashes (// -> /)
/// 2. Strips /emby prefix for Jellyfin compatibility
pub async fn normalize_path_middleware(
    mut req: Request,
    next: Next,
) -> Response {
    let uri = req.uri();
    let path = uri.path();
    
    // Remove double slashes
    let mut normalized = path.to_string();
    while normalized.contains("//") {
        normalized = normalized.replace("//", "/");
    }
    
    // Strip /emby prefix
    if normalized.starts_with("/emby/") {
        normalized = normalized.trim_start_matches("/emby").to_string();
    }
    
    // Update request URI if path changed
    if normalized != path {
        let mut parts = uri.clone().into_parts();
        parts.path_and_query = Some(
            format!("{}{}", 
                normalized,
                uri.query().map(|q| format!("?{}", q)).unwrap_or_default()
            ).parse().unwrap()
        );
        *req.uri_mut() = axum::http::Uri::from_parts(parts).unwrap();
    }
    
    next.run(req).await
}

/// Middleware to log HTTP requests
pub async fn log_request_middleware(
    req: Request,
    next: Next,
) -> Response {
    let method = req.method().clone();
    let uri = req.uri().clone();
    
    let response = next.run(req).await;
    
    let status = response.status();
    info!("{} {} - {}", method, uri, status);
    
    response
}
