use axum::{
    extract::State,
    http::{header, HeaderMap, Request, StatusCode},
    middleware::Next,
    response::Response,
};
use regex::Regex;
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

use crate::database::Repository;

static AUTH_HEADER_REGEX: OnceLock<Regex> = OnceLock::new();
static AUTH_HEADER_REGEX_UNQUOTED: OnceLock<Regex> = OnceLock::new();

fn get_auth_regex() -> &'static Regex {
    AUTH_HEADER_REGEX.get_or_init(|| Regex::new(r#"(\w+)="(.*?)""#).unwrap())
}

fn get_auth_regex_unquoted() -> &'static Regex {
    // Matches key=value where value may not be quoted (up to comma or end of string)
    AUTH_HEADER_REGEX_UNQUOTED.get_or_init(|| Regex::new(r#"(\w+)=([^,\s"]+)"#).unwrap())
}

#[derive(Clone)]
pub struct JellyfinAuthState {
    pub repo: Arc<dyn Repository>,
    pub server_id: String,
    pub auto_register: bool,
    pub quick_connect: bool,
}

#[derive(Debug, Clone)]
pub struct AuthSchemeValues {
    pub device: String,
    pub device_id: String,
    pub token: String,
    pub client: String,
    pub client_version: String,
}

/// Parse MediaBrowser/Emby authorization header
pub(crate) fn parse_auth_header(headers: &HeaderMap) -> Option<AuthSchemeValues> {
    let auth_header = headers
        .get(header::AUTHORIZATION)
        .or_else(|| headers.get("x-emby-authorization"))?
        .to_str()
        .ok()?;

    if !auth_header.starts_with("MediaBrowser ") && !auth_header.starts_with("Emby ") {
        return None;
    }

    let mut result = AuthSchemeValues {
        device: String::new(),
        device_id: String::new(),
        token: String::new(),
        client: String::new(),
        client_version: String::new(),
    };

    // Try quoted format first (standard), then unquoted (some clients like Swiftfin)
    let re_quoted = get_auth_regex();
    let mut found_any = false;
    for cap in re_quoted.captures_iter(auth_header) {
        if cap.len() == 3 {
            found_any = true;
            match &cap[1] {
                "Client" => result.client = cap[2].to_string(),
                "Version" => result.client_version = cap[2].to_string(),
                "Device" => result.device = cap[2].to_string(),
                "DeviceId" => result.device_id = cap[2].to_string(),
                "Token" => result.token = cap[2].to_string(),
                _ => {}
            }
        }
    }

    if !found_any {
        // Fall back to unquoted format
        for cap in get_auth_regex_unquoted().captures_iter(auth_header) {
            if cap.len() == 3 {
                match &cap[1] {
                    "Client" => result.client = cap[2].to_string(),
                    "Version" => result.client_version = cap[2].to_string(),
                    "Device" => result.device = cap[2].to_string(),
                    "DeviceId" => result.device_id = cap[2].to_string(),
                    "Token" => result.token = cap[2].to_string(),
                    _ => {}
                }
            }
        }
    }

    Some(result)
}

/// Generate random token
pub(crate) fn generate_random_token() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let bytes: [u8; 16] = rng.gen();
    hex::encode(bytes)
}

/// Authentication middleware
pub async fn auth_middleware(
    State(state): State<JellyfinAuthState>,
    headers: HeaderMap,
    mut request: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    // Try to extract token from various sources
    let token = extract_token(&headers, &request);

    if token.is_none() {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let token = token.unwrap();

    // Validate token
    let access_token = state
        .repo
        .get_access_token(&token)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // Store access token in request extensions
    request.extensions_mut().insert(access_token);

    Ok(next.run(request).await)
}

/// Extract token from headers or query parameters
fn extract_token(headers: &HeaderMap, request: &Request<axum::body::Body>) -> Option<String> {
    // Try auth header first
    if let Some(emby_header) = parse_auth_header(headers) {
        if !emby_header.token.is_empty() {
            return Some(emby_header.token);
        }
    }

    // Try x-emby-token header
    if let Some(token) = headers.get("x-emby-token") {
        if let Ok(token_str) = token.to_str() {
            return Some(token_str.to_string());
        }
    }

    // Try x-mediabrowser-token header
    if let Some(token) = headers.get("x-mediabrowser-token") {
        if let Ok(token_str) = token.to_str() {
            return Some(token_str.to_string());
        }
    }

    // Try query parameter ApiKey
    if let Some(query) = request.uri().query() {
        let params: HashMap<String, String> = url::form_urlencoded::parse(query.as_bytes())
            .into_owned()
            .collect();

        if let Some(api_key) = params.get("ApiKey") {
            return Some(api_key.clone());
        }

        // Deprecated: api_key
        if let Some(api_key) = params.get("api_key") {
            return Some(api_key.clone());
        }
    }

    None
}
