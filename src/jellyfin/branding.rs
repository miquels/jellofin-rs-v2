use super::types::*;
use axum::response::Json;

/// GET /Branding/Configuration - Get branding configuration
pub async fn branding_configuration() -> Json<BrandingConfiguration> {
    Json(BrandingConfiguration {
        login_disclaimer: String::new(),
        custom_css: String::new(),
        splashscreen_enabled: false,
    })
}

/// GET /Branding/Css
/// GET /Branding/Css.css
pub async fn branding_css() -> &'static str {
    ""
}
