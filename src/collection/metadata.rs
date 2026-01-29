use chrono::{DateTime, Utc};
use std::time::Duration;

/// Metadata holds metadata information for media items
#[derive(Debug, Clone, Default)]
pub struct Metadata {
    pub title: String,
    pub plot: String,
    pub genres: Vec<String>,
    pub studios: Vec<String>,
    pub year: Option<i32>,
    pub rating: f32,
    pub official_rating: String,
    pub premiered: Option<DateTime<Utc>>,
    pub duration: Duration,
    pub video_codec: String,
    pub video_bitrate: i32,
    pub video_frame_rate: f64,
    pub video_height: i32,
    pub video_width: i32,
    pub audio_codec: String,
    pub audio_bitrate: i32,
    pub audio_channels: i32,
    pub audio_language: String,
}

impl Metadata {
    pub fn runtime_ticks(&self) -> i64 {
        self.duration.as_micros() as i64 * 10
    }

    pub fn duration(&self) -> Duration {
        self.duration
    }
}
