use quick_xml::de::from_str;
use serde::Deserialize;
use std::fs;
use std::path::Path;
use tracing::warn;

use super::metadata::Metadata;

/// Parse movie NFO file
pub fn parse_movie_nfo(path: &Path) -> Option<Metadata> {
    let content = fs::read_to_string(path).ok()?;
    let nfo: MovieNfo = match from_str(&content) {
        Ok(n) => n,
        Err(e) => {
            warn!("Failed to parse movie NFO {}: {}", path.display(), e);
            return None;
        }
    };

    Some(nfo.into())
}

/// Parse TV show NFO file
pub fn parse_show_nfo(path: &Path) -> Option<Metadata> {
    let content = fs::read_to_string(path).ok()?;
    let nfo: ShowNfo = match from_str(&content) {
        Ok(n) => n,
        Err(e) => {
            warn!("Failed to parse show NFO {}: {}", path.display(), e);
            return None;
        }
    };

    Some(nfo.into())
}

/// Parse episode NFO file
pub fn parse_episode_nfo(path: &Path) -> Option<Metadata> {
    let content = fs::read_to_string(path).ok()?;
    let nfo: EpisodeNfo = match from_str(&content) {
        Ok(n) => n,
        Err(e) => {
            warn!("Failed to parse episode NFO {}: {}", path.display(), e);
            return None;
        }
    };

    Some(nfo.into())
}

// --- NFO Structures ---

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
struct MovieNfo {
    title: Option<String>,
    #[allow(dead_code)]
    originaltitle: Option<String>,
    #[allow(dead_code)]
    sorttitle: Option<String>,
    rating: Option<f32>,
    year: Option<i32>,
    plot: Option<String>,
    mpaa: Option<String>,
    #[serde(default)]
    genre: Vec<String>,
    #[serde(default)]
    studio: Vec<String>,
    #[allow(dead_code)]
    premiered: Option<String>,
    fileinfo: Option<FileInfo>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
struct ShowNfo {
    title: Option<String>,
    rating: Option<f32>,
    year: Option<i32>,
    plot: Option<String>,
    mpaa: Option<String>,
    #[serde(default)]
    genre: Vec<String>,
    #[serde(default)]
    studio: Vec<String>,
    #[allow(dead_code)]
    premiered: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
struct EpisodeNfo {
    title: Option<String>,
    rating: Option<f32>,
    plot: Option<String>,
    #[allow(dead_code)]
    aired: Option<String>,
    fileinfo: Option<FileInfo>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
struct FileInfo {
    streamdetails: Option<StreamDetails>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
struct StreamDetails {
    video: Option<VideoDetails>,
    audio: Option<AudioDetails>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
struct VideoDetails {
    codec: Option<String>,
    width: Option<i32>,
    height: Option<i32>,
    durationinseconds: Option<i32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
struct AudioDetails {
    codec: Option<String>,
    language: Option<String>,
    channels: Option<i32>,
}

// --- Conversions ---

impl From<MovieNfo> for Metadata {
    fn from(nfo: MovieNfo) -> Self {
        let mut m = Metadata::default();
        if let Some(t) = nfo.title { m.title = t; }
        if let Some(p) = nfo.plot { m.plot = p; }
        if let Some(r) = nfo.rating { m.rating = r; }
        if let Some(y) = nfo.year { m.year = Some(y); }
        if let Some(mpaa) = nfo.mpaa { m.official_rating = mpaa; }
        m.genres = nfo.genre;
        m.studios = nfo.studio;
        
        if let Some(fi) = nfo.fileinfo {
            apply_file_info(&mut m, fi);
        }

        m
    }
}

impl From<ShowNfo> for Metadata {
    fn from(nfo: ShowNfo) -> Self {
        let mut m = Metadata::default();
        if let Some(t) = nfo.title { m.title = t; }
        if let Some(p) = nfo.plot { m.plot = p; }
        if let Some(r) = nfo.rating { m.rating = r; }
        if let Some(y) = nfo.year { m.year = Some(y); }
        if let Some(mpaa) = nfo.mpaa { m.official_rating = mpaa; }
        m.genres = nfo.genre;
        m.studios = nfo.studio;
        m
    }
}

impl From<EpisodeNfo> for Metadata {
    fn from(nfo: EpisodeNfo) -> Self {
        let mut m = Metadata::default();
        if let Some(t) = nfo.title { m.title = t; }
        if let Some(p) = nfo.plot { m.plot = p; }
        if let Some(r) = nfo.rating { m.rating = r; }
        
        if let Some(fi) = nfo.fileinfo {
            apply_file_info(&mut m, fi);
        }
        
        m
    }
}

fn apply_file_info(m: &mut Metadata, fi: FileInfo) {
    if let Some(sd) = fi.streamdetails {
        if let Some(v) = sd.video {
            if let Some(c) = v.codec { m.video_codec = c; }
            if let Some(w) = v.width { m.video_width = w; }
            if let Some(h) = v.height { m.video_height = h; }
            if let Some(d) = v.durationinseconds { 
                m.duration = std::time::Duration::from_secs(d as u64); 
            }
        }
        if let Some(a) = sd.audio {
             if let Some(c) = a.codec { m.audio_codec = c; }
             if let Some(l) = a.language { m.audio_language = l; }
             if let Some(c) = a.channels { m.audio_channels = c; }
        }
    }
}
