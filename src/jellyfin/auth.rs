use axum::{
    extract::{Query, State},
    http::{header, HeaderMap, Request, StatusCode},
    middleware::Next,
    response::Response,
    Extension, Json,
};
use bcrypt::{hash, verify, DEFAULT_COST};
use regex::Regex;
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

use crate::database::{model, Repository};
use crate::idhash::id_hash;

use super::types::*;
use super::user::make_user;

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
    if let Some(db_user) = user.take() {
         // Verify password
        if !verify(&request.pw, &db_user.password).unwrap_or(false) {
            return Err(StatusCode::UNAUTHORIZED);
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

    let device_id = emby_header.as_ref().map(|h| h.device_id.as_str()).unwrap_or("");

    // Reuse existing token for the same device if one exists
    let access_token = if !device_id.is_empty() {
        if let Ok(mut existing) = state.repo.get_access_token_by_device_id(device_id).await {
            // Update last_used and details, reuse the token string
            existing.user_id = user.id.clone();
            existing.last_used = chrono::Utc::now();
            if let Some(ref h) = emby_header {
                existing.device_name = h.device.clone();
                existing.application_name = h.client.clone();
                existing.application_version = h.client_version.clone();
            }
            let _ = state.repo.upsert_access_token(&existing).await;
            existing
        } else {
            create_new_token(&state.repo, &user.id, emby_header.as_ref()).await?
        }
    } else {
        create_new_token(&state.repo, &user.id, emby_header.as_ref()).await?
    };

    let response = AuthenticateByNameResponse {
        access_token: access_token.token.clone(),
        session_info: make_session_info(&access_token, &user.username, &state.server_id),
        server_id: state.server_id.clone(),
        user: make_user(&user, &state.server_id),
    };

    Ok(Json(response))
}

/// Create and store a fresh access token
async fn create_new_token(
    repo: &Arc<dyn Repository>,
    user_id: &str,
    emby_header: Option<&AuthSchemeValues>,
) -> Result<model::AccessToken, StatusCode> {
    let mut access_token = model::AccessToken {
        token: generate_random_token(),
        user_id: user_id.to_string(),
        device_name: String::new(),
        device_id: String::new(),
        application_name: String::new(),
        application_version: String::new(),
        remote_address: String::new(),
        created: chrono::Utc::now(),
        last_used: chrono::Utc::now(),
    };
    if let Some(h) = emby_header {
        access_token.device_name = h.device.clone();
        access_token.device_id = h.device_id.clone();
        access_token.application_name = h.client.clone();
        access_token.application_version = h.client_version.clone();
    }
    repo.upsert_access_token(&access_token)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(access_token)
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
        properties: model::UserProperties::default(),
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



/// GET /QuickConnect/Enabled
pub async fn quick_connect_enabled(State(state): State<JellyfinAuthState>) -> Json<bool> {
    Json(state.quick_connect)
}

/// POST /QuickConnect/Initiate - Start a new QuickConnect session
pub async fn quick_connect_initiate(
    State(state): State<JellyfinAuthState>,
    headers: HeaderMap,
) -> axum::response::Response {
    use axum::response::IntoResponse;
    if !state.quick_connect {
        return StatusCode::FORBIDDEN.into_response();
    }

    let emby = parse_auth_header(&headers);
    let device_id = emby.as_ref().map(|h| h.device_id.as_str()).unwrap_or("unknown").to_string();

    // Generate 6-digit user-visible code and 32-char secret
    let code = {
        use rand::Rng;
        let n: u32 = rand::thread_rng().gen_range(0..1_000_000);
        format!("{:06}", n)
    };
    let secret = generate_random_token();

    let qc = model::QuickConnectCode {
        user_id: String::new(),
        device_id: device_id.clone(),
        secret: secret.clone(),
        authorized: false,
        code: code.clone(),
        created: chrono::Utc::now(),
    };

    if state.repo.upsert_quick_connect(&qc).await.is_err() {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    Json(serde_json::json!({
        "Secret": secret,
        "Code": code,
        "DeviceId": device_id,
        "Authenticated": false
    }))
    .into_response()
}

/// GET /QuickConnect/Connect - Check QuickConnect status by secret
pub async fn quick_connect_connect(
    State(state): State<JellyfinAuthState>,
    Query(params): Query<HashMap<String, String>>,
) -> axum::response::Response {
    use axum::response::IntoResponse;
    if !state.quick_connect {
        return StatusCode::FORBIDDEN.into_response();
    }

    let secret = match params.get("Secret") {
        Some(s) => s.clone(),
        None => return StatusCode::BAD_REQUEST.into_response(),
    };

    match state.repo.get_quick_connect_by_secret(&secret).await {
        Ok(qc) => Json(serde_json::json!({
            "Secret": qc.secret,
            "Code": qc.code,
            "DeviceId": qc.device_id,
            "Authenticated": qc.authorized
        }))
        .into_response(),
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

/// POST /QuickConnect/Authorize - Authorize a QuickConnect code (requires auth)
/// Uses JellyfinState so it can be placed in the authenticated router section.
pub async fn quick_connect_authorize(
    Extension(token): Extension<model::AccessToken>,
    State(state): State<super::jellyfin::JellyfinState>,
    Query(params): Query<HashMap<String, String>>,
) -> StatusCode {
    if !state.config.quick_connect().unwrap_or(false) {
        return StatusCode::FORBIDDEN;
    }

    let code = match params.get("Code") {
        Some(c) => c.clone(),
        None => return StatusCode::BAD_REQUEST,
    };

    let mut qc = match state.repo.get_quick_connect_by_code(&code).await {
        Ok(q) => q,
        Err(_) => return StatusCode::NOT_FOUND,
    };

    // Check code hasn't expired (10 minute window)
    let age = chrono::Utc::now() - qc.created;
    if age.num_minutes() > 10 {
        return StatusCode::GONE;
    }

    qc.authorized = true;
    qc.user_id = token.user_id.clone();

    match state.repo.upsert_quick_connect(&qc).await {
        Ok(_) => StatusCode::OK,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

/// POST /Users/AuthenticateWithQuickConnect
pub async fn authenticate_with_quick_connect(
    State(state): State<JellyfinAuthState>,
    headers: HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> axum::response::Response {
    use axum::response::IntoResponse;
    if !state.quick_connect {
        return StatusCode::FORBIDDEN.into_response();
    }

    let secret = match body.get("Secret").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => return StatusCode::BAD_REQUEST.into_response(),
    };

    let qc = match state.repo.get_quick_connect_by_secret(&secret).await {
        Ok(q) => q,
        Err(_) => return StatusCode::UNAUTHORIZED.into_response(),
    };

    if !qc.authorized || qc.user_id.is_empty() {
        return StatusCode::UNAUTHORIZED.into_response();
    }

    // Check not expired
    let age = chrono::Utc::now() - qc.created;
    if age.num_minutes() > 10 {
        return StatusCode::GONE.into_response();
    }

    let user = match state.repo.get_user_by_id(&qc.user_id).await {
        Ok(u) => u,
        Err(_) => return StatusCode::UNAUTHORIZED.into_response(),
    };

    let emby_header = parse_auth_header(&headers);
    let access_token = match create_new_token(&state.repo, &user.id, emby_header.as_ref()).await {
        Ok(t) => t,
        Err(s) => return s.into_response(),
    };

    let response = AuthenticateByNameResponse {
        access_token: access_token.token.clone(),
        session_info: make_session_info(&access_token, &user.username, &state.server_id),
        server_id: state.server_id.clone(),
        user: super::user::make_user(&user, &state.server_id),
    };

    Json(response).into_response()
}
