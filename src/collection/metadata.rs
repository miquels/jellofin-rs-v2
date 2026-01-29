use chrono::{DateTime, Utc};
use std::time::Duration;

/// Metadata holds metadata information for media items
#[derive(Debug, Clone, Default)]
pub struct Metadata {
    // Will be fully implemented in Phase 4
    title: String,
    plot: String,
    genres: Vec<String>,
    studios: Vec<String>,
    year: i32,
    rating: f32,
    official_rating: String,
    premiered: Option<DateTime<Utc>>,
    duration: Duration,
    video_codec: String,
    video_bitrate: i32,
    video_frame_rate: f64,
    video_height: i32,
    video_width: i32,
    audio_codec: String,
    audio_bitrate: i32,
    audio_channels: i32,
    audio_language: String,
}

impl Metadata {
    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn plot(&self) -> &str {
        &self.plot
    }
    
    pub fn set_plot(&mut self, plot: String) {
        self.plot = plot;
    }

    pub fn genres(&self) -> &[String] {
        &self.genres
    }

    pub fn studios(&self) -> &[String] {
        &self.studios
    }

    pub fn year(&self) -> i32 {
        self.year
    }

    pub fn rating(&self) -> f32 {
        self.rating
    }

    pub fn official_rating(&self) -> &str {
        &self.official_rating
    }

    pub fn premiered(&self) -> Option<DateTime<Utc>> {
        self.premiered
    }

    pub fn duration(&self) -> Duration {
        self.duration
    }

    pub fn video_codec(&self) -> &str {
        &self.video_codec
    }

    pub fn video_bitrate(&self) -> i32 {
        self.video_bitrate
    }

    pub fn video_frame_rate(&self) -> f64 {
        self.video_frame_rate
    }

    pub fn video_height(&self) -> i32 {
        self.video_height
    }

    pub fn video_width(&self) -> i32 {
        self.video_width
    }

    pub fn audio_codec(&self) -> &str {
        &self.audio_codec
    }

    pub fn audio_bitrate(&self) -> i32 {
        self.audio_bitrate
    }

    pub fn audio_channels(&self) -> i32 {
        self.audio_channels
    }

    pub fn audio_language(&self) -> &str {
        &self.audio_language
    }
}
