use axum::{extract::{Request, State}, middleware::Next, response::Response};
use tracing::info;
use crate::server::AppState;

/// Middleware to normalize request paths
/// 1. Removes redundant slashes (// -> /)
/// 2. Strips /emby prefix for Jellyfin compatibility
pub async fn normalize_path_middleware(mut req: Request, next: Next) -> Response {
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
            format!(
                "{}{}",
                normalized,
                uri.query().map(|q| format!("?{}", q)).unwrap_or_default()
            )
            .parse()
            .unwrap(),
        );
        *req.uri_mut() = axum::http::Uri::from_parts(parts).unwrap();
    }

    next.run(req).await
}

pub async fn log_request_middleware(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Response {
    let debug_logs = state.debug;
    let method = req.method().clone();
    let uri = req.uri().clone();

    // Log POST request body for debugging
    let (parts, body) = req.into_parts();
    let bytes = axum::body::to_bytes(body, usize::MAX)
        .await
        .unwrap_or_default();

    if debug_logs {
        info!("Request: {} {}", method, uri);
        for (name, value) in &parts.headers {
            info!("Req Header: {}: {:?}", name, value);
        }
    }

    if method == axum::http::Method::POST && !bytes.is_empty() {
        if let Ok(body_str) = std::str::from_utf8(&bytes) {
            info!(
                method = %method,
                url = %uri,
                body = %body_str,
                "POST request"
            );
        }
    } else if !debug_logs {
        info!(
            method = %method,
            url = %uri,
            "Request started"
        );
    }

    // Reconstruct request with body
    let req = Request::from_parts(parts, axum::body::Body::from(bytes));
    let response = next.run(req).await;

    let status = response.status().as_u16();

    if debug_logs {
        info!("Response: {} {} Status: {}", method, uri, status);
        for (name, value) in response.headers() {
            info!("Res Header: {}: {:?}", name, value);
        }
    }

    // Check Content-Type to decide whether to buffer body
    let content_type = response
        .headers()
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|ct| ct.to_lowercase())
        .unwrap_or_default();

    let is_text = content_type.contains("json")
        || content_type.contains("text")
        || content_type.contains("xml")
        || content_type.contains("application/x-www-form-urlencoded");

    if debug_logs {
        info!(
            "Deciding body logging for Content-Type: '{}', is_text: {}",
            content_type, is_text
        );
    }

    if is_text {
        // Buffer text/json responses for debugging logging
        let (parts, body) = response.into_parts();
        let bytes = axum::body::to_bytes(body, usize::MAX)
            .await
            .unwrap_or_default();
        let length = bytes.len();

        let body_str_res = std::str::from_utf8(&bytes);

        if debug_logs {
            match body_str_res {
                Ok(body_str) => info!("Res Body: {}", body_str),
                Err(e) => info!("Res Body skipped: Invalid UTF-8 sequence: {}", e),
            }
        }

        // Standard logging
        if let Ok(body_str) = body_str_res {
            info!(
                method = %method,
                url = %uri,
                status = status,
                length = length,
                res_body = %body_str,
                "HTTP request (Logged)"
            );
        } else {
            info!(
                method = %method,
                url = %uri,
                status = status,
                length = length,
                "HTTP request"
            );
        }

        Response::from_parts(parts, axum::body::Body::from(bytes))
    } else {
        // Do NOT buffer video/binary streams - pass through directly
        info!(
            method = %method,
            url = %uri,
            status = status,
            type = %content_type,
            "HTTP request (Streamed)"
        );
        response
    }
}

pub async fn add_cors_headers_middleware(req: Request, next: Next) -> Response {
    // Handle OPTIONS requests for CORS preflight
    if req.method() == axum::http::Method::OPTIONS {
        let mut response = Response::new(axum::body::Body::empty());
        *response.status_mut() = axum::http::StatusCode::OK;

        let headers = response.headers_mut();
        headers.insert("Access-Control-Allow-Origin", "*".parse().unwrap());
        headers.insert(
            "Access-Control-Allow-Methods",
            "GET, HEAD, OPTIONS, POST, PUT, DELETE".parse().unwrap(),
        );
        headers.insert(
            "Access-Control-Allow-Headers",
            "Content-Type, Authorization, Range, x-playback-session-id"
                .parse()
                .unwrap(),
        );
        headers.insert(
            "Access-Control-Expose-Headers",
            "ETag, Content-Length, Content-Range".parse().unwrap(),
        );

        return response;
    }

    let mut response = next.run(req).await;

    let headers = response.headers_mut();
    headers.insert("Access-Control-Allow-Origin", "*".parse().unwrap());
    headers.insert(
        "Access-Control-Allow-Methods",
        "GET, HEAD, OPTIONS, POST, PUT, DELETE".parse().unwrap(),
    );
    headers.insert(
        "Access-Control-Allow-Headers",
        "Content-Type, Authorization, Range, x-playback-session-id"
            .parse()
            .unwrap(),
    );
    headers.insert(
        "Access-Control-Expose-Headers",
        "ETag, Content-Length, Content-Range".parse().unwrap(),
    );
    headers.insert(
        "Cache-Control",
        "public, max-age=86400, stale-while-revalidate=600"
            .parse()
            .unwrap(),
    );

    response
}

pub async fn etag_validation_middleware(req: Request, next: Next) -> Response {
    // Get the If-None-Match header from the request
    let if_none_match = req
        .headers()
        .get(axum::http::header::IF_NONE_MATCH)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let response = next.run(req).await;

    // If there's an If-None-Match header, check it against the response ETag
    if let Some(client_etag) = if_none_match {
        if let Some(response_etag) = response.headers().get(axum::http::header::ETAG) {
            if let Ok(response_etag_str) = response_etag.to_str() {
                // Compare ETags (handle both strong and weak ETags)
                if etags_match(&client_etag, response_etag_str) {
                    // ETags match - return 304 Not Modified with empty body
                    let mut not_modified = Response::new(axum::body::Body::empty());
                    *not_modified.status_mut() = axum::http::StatusCode::NOT_MODIFIED;

                    // Copy relevant headers from original response
                    let headers = not_modified.headers_mut();
                    if let Some(etag) = response.headers().get(axum::http::header::ETAG) {
                        headers.insert(axum::http::header::ETAG, etag.clone());
                    }
                    if let Some(cache_control) =
                        response.headers().get(axum::http::header::CACHE_CONTROL)
                    {
                        headers.insert(axum::http::header::CACHE_CONTROL, cache_control.clone());
                    }
                    if let Some(vary) = response.headers().get(axum::http::header::VARY) {
                        headers.insert(axum::http::header::VARY, vary.clone());
                    }

                    return not_modified;
                }
            }
        }
    }

    response
}

fn etags_match(client_etag: &str, server_etag: &str) -> bool {
    // Handle multiple ETags in If-None-Match (comma-separated)
    for etag in client_etag.split(',') {
        let etag = etag.trim();

        // Check for exact match
        if etag == server_etag {
            return true;
        }

        // Handle weak ETag comparison (W/"..." matches "...")
        let client_stripped = etag.strip_prefix("W/").unwrap_or(etag);
        let server_stripped = server_etag.strip_prefix("W/").unwrap_or(server_etag);

        if client_stripped == server_stripped {
            return true;
        }
    }

    false
}
