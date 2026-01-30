use axum::{
    extract::State,
    http::{header, HeaderMap, Request, StatusCode},
    middleware::Next,
    response::Response,
    Json,
};
use bcrypt::{hash, verify, DEFAULT_COST};
use regex::Regex;
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

use crate::database::{model, Repository};
use crate::idhash::id_hash;

use super::types::*;

static AUTH_HEADER_REGEX: OnceLock<Regex> = OnceLock::new();

fn get_auth_regex() -> &'static Regex {
    AUTH_HEADER_REGEX.get_or_init(|| Regex::new(r#"(\w+)="(.*?)""#).unwrap())
}

#[derive(Clone)]
pub struct JellyfinAuthState {
    pub repo: Arc<dyn Repository>,
    pub server_id: String,
    pub auto_register: bool,
}

#[derive(Debug, Clone)]
pub struct AuthSchemeValues {
    pub device: String,
    pub device_id: String,
    pub token: String,
    pub client: String,
    pub client_version: String,
}

/// POST /Users/AuthenticateByName
pub async fn authenticate_by_name(
    State(state): State<JellyfinAuthState>,
    headers: HeaderMap,
    Json(request): Json<AuthenticateUserByNameRequest>,
) -> Result<Json<AuthenticateByNameResponse>, StatusCode> {
    if request.username.is_empty() || request.pw.is_empty() {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let username = request.username.to_lowercase();

    // Try to get user from database
    let mut user = state.repo.get_user(&username).await.ok();

    // Check if user exists or needs creation
    if let Some(mut db_user) = user.take() {
         // Verify password
        if !verify(&request.pw, &db_user.password).unwrap_or(false) {
             // If auto-register is enabled, update password
            if state.auto_register {
                 let hashed_password = hash(&request.pw, DEFAULT_COST).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                 db_user.password = hashed_password;
                 user = Some(db_user);
            } else {
                return Err(StatusCode::UNAUTHORIZED);
            }
        } else {
            user = Some(db_user);
        }
    } else if state.auto_register {
        // Auto-register user
        user = create_user(&state.repo, &username, &request.pw).await.ok();
        if user.is_none() {
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    } else {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let mut user = user.unwrap();

    // Update last login
    user.last_login = chrono::Utc::now();
    user.last_used = chrono::Utc::now();
    if let Err(_) = state.repo.upsert_user(&user).await {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    // Parse auth header
    let emby_header = parse_auth_header(&headers);

    // Generate random token
    let token = generate_random_token();

    // Create access token
    let mut access_token = model::AccessToken {
        token: token.clone(),
        user_id: user.id.clone(),
        device_name: String::new(),
        device_id: String::new(),
        application_name: String::new(),
        application_version: String::new(),
        remote_address: String::new(),
        created: chrono::Utc::now(),
        last_used: chrono::Utc::now(),
    };

    // Update token details from header
    if let Some(ref header) = emby_header {
        access_token.device_name = header.device.clone();
        access_token.device_id = header.device_id.clone();
        access_token.application_name = header.client.clone();
        access_token.application_version = header.client_version.clone();
    }

    if let Err(_) = state.repo.upsert_access_token(&access_token).await {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    let response = AuthenticateByNameResponse {
        access_token: token,
        session_info: make_session_info(&access_token, &user.username, &state.server_id),
        server_id: state.server_id.clone(),
        user: make_user(&user, &state.server_id),
    };

    Ok(Json(response))
}

/// Create a new user
async fn create_user(repo: &Arc<dyn Repository>, username: &str, password: &str) -> Result<model::User, String> {
    let hashed_password = hash(password, DEFAULT_COST).map_err(|e| e.to_string())?;

    let user = model::User {
        id: id_hash(username),
        username: username.to_lowercase(),
        password: hashed_password,
        created: chrono::Utc::now(),
        last_login: chrono::Utc::now(),
        last_used: chrono::Utc::now(),
    };

    repo.upsert_user(&user).await.map_err(|e| e.to_string())?;

    Ok(user)
}

/// Parse MediaBrowser/Emby authorization header
fn parse_auth_header(headers: &HeaderMap) -> Option<AuthSchemeValues> {
    let auth_header = headers
        .get(header::AUTHORIZATION)
        .or_else(|| headers.get("x-emby-authorization"))?
        .to_str()
        .ok()?;

    if !auth_header.starts_with("MediaBrowser ") && !auth_header.starts_with("Emby ") {
        return None;
    }

    let re = get_auth_regex();
    let mut result = AuthSchemeValues {
        device: String::new(),
        device_id: String::new(),
        token: String::new(),
        client: String::new(),
        client_version: String::new(),
    };

    for cap in re.captures_iter(auth_header) {
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

    Some(result)
}

/// Generate random token
fn generate_random_token() -> String {
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
        let params: HashMap<String, String> = url::form_urlencoded::parse(query.as_bytes()).into_owned().collect();

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

/// Make SessionInfo from access token
fn make_session_info(token: &model::AccessToken, username: &str, server_id: &str) -> SessionInfo {
    SessionInfo {
        play_state: PlayState::default(),
        additional_users: Vec::new(),
        capabilities: SessionResponseCapabilities::default(),
        remote_end_point: token.remote_address.clone(),
        playable_media_types: vec!["Video".to_string(), "Audio".to_string()],
        id: token.token.clone(),
        user_id: token.user_id.clone(),
        user_name: username.to_string(),
        client: token.application_name.clone(),
        last_activity_date: token.last_used,
        device_name: token.device_name.clone(),
        device_id: token.device_id.clone(),
        application_version: token.application_version.clone(),
        is_active: true,
        supports_media_control: false,
        supports_remote_control: false,
        server_id: server_id.to_string(),
        supported_commands: Vec::new(),
        has_custom_device_name: false,
        now_playing_queue: Vec::new(),
        now_playing_queue_full_items: Vec::new(),
    }
}

/// Make User response from database user
fn make_user(user: &model::User, server_id: &str) -> User {
    User {
        name: user.username.clone(),
        server_id: server_id.to_string(),
        id: user.id.clone(),
        has_password: true,
        has_configured_password: true,
        has_configured_easy_password: false,
        enable_auto_login: false,
        last_login_date: user.last_login,
        last_activity_date: user.last_used,
        configuration: UserConfiguration::default(),
        policy: UserPolicy::default(),
        primary_image_tag: None,
    }
}

/// GET /QuickConnect/Enabled
pub async fn quick_connect_enabled() -> Json<bool> {
    Json(false)
}

/// POST /QuickConnect/Initiate
pub async fn quick_connect_initiate() -> impl axum::response::IntoResponse {
    StatusCode::NOT_IMPLEMENTED
}

/// GET /QuickConnect/Connect
pub async fn quick_connect_connect() -> impl axum::response::IntoResponse {
    StatusCode::NOT_IMPLEMENTED
}

/// POST /QuickConnect/Authorize
pub async fn quick_connect_authorize() -> impl axum::response::IntoResponse {
    StatusCode::NO_CONTENT
}
