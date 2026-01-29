use axum::{
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use std::os::unix::fs::MetadataExt;

pub fn check_etag(headers: &HeaderMap, path: &str, metadata: &std::fs::Metadata) -> Option<Response> {
    let mut hasher = Sha256::new();
    hasher.update(path.as_bytes());
    let path_hash = hasher.finalize();

    let etag = format!("\"{:x}.{}.{:x}\"", path_hash, metadata.ino(), metadata.mtime());

    if let Some(if_none_match) = headers.get(header::IF_NONE_MATCH) {
        if if_none_match.to_str().unwrap_or_default().contains(&etag) {
            return Some(
                (
                    StatusCode::NOT_MODIFIED,
                    [
                        (header::ETAG, etag),
                        (
                            header::LAST_MODIFIED,
                            httpdate::fmt_http_date(
                                metadata.modified().unwrap_or_else(|_| std::time::SystemTime::now()),
                            ),
                        ),
                    ],
                )
                    .into_response(),
            );
        }
    }
    None
}

pub fn check_etag_obj(headers: &HeaderMap, ts: DateTime<Utc>) -> Option<Response> {
    let etag = format!("\"{:x}\"", ts.timestamp());

    if let Some(if_none_match) = headers.get(header::IF_NONE_MATCH) {
        if if_none_match.to_str().unwrap_or_default().contains(&etag) {
            return Some(
                (
                    StatusCode::NOT_MODIFIED,
                    [
                        (header::ETAG, etag),
                        (header::LAST_MODIFIED, httpdate::fmt_http_date(ts.into())),
                    ],
                )
                    .into_response(),
            );
        }
    }
    None
}
