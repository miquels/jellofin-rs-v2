use super::types::HTTPError;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Json},
};
use std::collections::HashMap;

/// apierror returns a structured HTTP error response.
pub fn apierror(status: StatusCode, msg: &str) -> impl IntoResponse {
    let mut status_type_map = HashMap::new();
    status_type_map.insert(400, "https://tools.ietf.org/html/rfc9110#section-15.5.1");
    status_type_map.insert(401, "https://tools.ietf.org/html/rfc9110#section-15.5.2");
    status_type_map.insert(403, "https://tools.ietf.org/html/rfc9110#section-15.5.3");
    status_type_map.insert(404, "https://tools.ietf.org/html/rfc9110#section-15.5.5");
    status_type_map.insert(405, "https://tools.ietf.org/html/rfc9110#section-15.5.6");
    status_type_map.insert(500, "https://tools.ietf.org/html/rfc9110#section-15.6.1");

    let status_val = status.as_u16() as i32;
    let r#type = status_type_map.get(&status_val).map(|s| s.to_string());

    let error_response = HTTPError {
        status: status_val,
        r#type,
        title: Some(msg.to_string()),
        errors: None,
        trace_id: None,
    };

    (status, Json(error_response))
}
