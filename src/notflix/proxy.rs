use axum::{
    body::Body,
    extract::{Path as AxumPath, State},
    http::{HeaderMap, Request, StatusCode},
    response::{IntoResponse, Response},
};
use reqwest::Client;
use tokio::sync::OnceCell;

use super::notflix::NotflixState;

static NET_CLIENT: OnceCell<Client> = OnceCell::const_new();

async fn get_client() -> &'static Client {
    NET_CLIENT
        .get_or_init(|| async {
            Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .unwrap()
        })
        .await
}

pub async fn hls_handler(
    State(state): State<NotflixState>,
    AxumPath((source, path)): AxumPath<(String, String)>,
    req: Request<Body>,
) -> impl IntoResponse {
    if !path.contains(".mp4/") {
        return StatusCode::NOT_FOUND.into_response();
    }

    let collection = match state.collections.get_collection(&source) {
        Some(c) => c,
        None => return StatusCode::NOT_FOUND.into_response(),
    };

    let hls_server = collection.hls_server.clone();
    if hls_server.is_empty() {
        return StatusCode::NOT_FOUND.into_response();
    }

    let target_url = format!("{}{}", hls_server, path);

    let client = get_client().await;

    // Forward headers after removing hop-by-hop headers
    let mut forward_headers = HeaderMap::new();
    for (name, value) in req.headers() {
        if !is_hop_header(name.as_str()) {
            forward_headers.insert(name, value.clone());
        }
    }

    // Add X-Forwarded-For
    // Note: In real production we'd get the actual client IP

    let resp = match client.get(&target_url).headers(forward_headers).send().await {
        Ok(r) => r,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    let mut builder = Response::builder().status(resp.status());

    for (name, value) in resp.headers() {
        if !is_hop_header(name.as_str()) {
            builder = builder.header(name, value);
        }
    }

    let stream = resp.bytes_stream();
    builder.body(Body::from_stream(stream)).unwrap().into_response()
}

fn is_hop_header(name: &str) -> bool {
    matches!(
        name.to_lowercase().as_str(),
        "connection"
            | "keep-alive"
            | "proxy-authenticate"
            | "proxy-authorization"
            | "te"
            | "trailers"
            | "transfer-encoding"
            | "upgrade"
    )
}
