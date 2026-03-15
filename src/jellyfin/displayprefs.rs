use axum::{extract::Path, response::Json};

use super::types::*;

/// GET /DisplayPreferences/{id} - Get display preferences
pub async fn display_preferences(
    Path(id): Path<String>,
) -> Json<DisplayPreferencesResponse> {
    Json(DisplayPreferencesResponse {
        id,
        sort_by: "SortName".to_string(),
        remember_indexing: false,
        primary_image_height: 250,
        primary_image_width: 250,
        custom_prefs: DisplayPreferencesCustomPrefs {
            chromecast_version: "stable".to_string(),
            skip_forward_length: "30000".to_string(),
            skip_back_length: "10000".to_string(),
            enable_next_video_info_overlay: "False".to_string(),
            tvhome: "null".to_string(),
            dashboard_theme: "null".to_string(),
        },
        scroll_direction: "Horizontal".to_string(),
        show_backdrop: true,
        remember_sorting: false,
        sort_order: "Ascending".to_string(),
        show_sidebar: false,
        client: "emby".to_string(),
    })
}
