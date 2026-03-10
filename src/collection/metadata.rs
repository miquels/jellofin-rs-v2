use chrono::{DateTime, Utc};
use std::time::Duration;

/// Metadata holds metadata information for media items
#[derive(Debug, Clone, Default)]
pub struct Metadata {
    pub title: Option<String>,
    pub plot: Option<String>,
    pub taglines: Vec<String>,
    pub genres: Vec<String>,
    pub studios: Vec<String>,
    pub year: Option<i32>,
    pub rating: Option<f32>,
    pub official_rating: Option<String>,
    pub premiered: Option<DateTime<Utc>>,
    pub duration: Option<Duration>,
    pub video_codec: Option<String>,
    pub video_bitrate: Option<i32>,
    pub video_frame_rate: Option<f64>,
    pub video_height: Option<i32>,
    pub video_width: Option<i32>,
    pub audio_codec: Option<String>,
    pub audio_bitrate: Option<i32>,
    pub audio_channels: Option<i32>,
    pub audio_language: Option<String>,
}

impl Metadata {
    pub fn runtime_ticks(&self) -> Option<i64> {
        self.duration.map(|d| d.as_micros() as i64 * 10)
    }

    pub fn duration(&self) -> Option<Duration> {
        self.duration
    }
}
