use serde::Deserialize;
use std::fs;
use std::path::Path;

use chrono::Datelike;
use quick_xml::de::from_str;
use tracing::warn;

use super::metadata::Metadata;
use crate::jellyfin::parse_iso8601_date;

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

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "lowercase")]
struct MovieNfo {
    title: Option<String>,
    #[allow(dead_code)]
    originaltitle: Option<String>,
    #[allow(dead_code)]
    sorttitle: Option<String>,
    rating: Option<f32>,
    year: Option<i32>,
    plot: Option<String>,
    tagline: Vec<String>,
    mpaa: Option<String>,
    genre: Vec<String>,
    studio: Vec<String>,
    actor: Vec<NfoActor>,
    director: Vec<String>,
    premiered: Option<String>,
    fileinfo: Option<FileInfo>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "lowercase")]
struct ShowNfo {
    title: Option<String>,
    rating: Option<f32>,
    year: Option<i32>,
    plot: Option<String>,
    tagline: Vec<String>,
    mpaa: Option<String>,
    genre: Vec<String>,
    studio: Vec<String>,
    actor: Vec<NfoActor>,
    director: Vec<String>,
    premiered: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "lowercase")]
struct EpisodeNfo {
    title: Option<String>,
    rating: Option<f32>,
    plot: Option<String>,
    #[allow(dead_code)]
    aired: Option<String>,
    fileinfo: Option<FileInfo>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "lowercase")]
struct FileInfo {
    streamdetails: Option<StreamDetails>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "lowercase")]
struct StreamDetails {
    video: Option<VideoDetails>,
    audio: Option<AudioDetails>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "lowercase")]
struct VideoDetails {
    codec: Option<String>,
    width: Option<i32>,
    height: Option<i32>,
    durationinseconds: Option<i32>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "lowercase")]
struct AudioDetails {
    codec: Option<String>,
    language: Option<String>,
    channels: Option<i32>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "lowercase")]
struct NfoActor {
    name: String,
    #[allow(dead_code)]
    role: Option<String>,
}

// --- Conversions ---

impl From<MovieNfo> for Metadata {
    fn from(nfo: MovieNfo) -> Self {
        let premiered = nfo.premiered.as_ref().and_then(|d| parse_iso8601_date(d));
        let year = premiered.map(|d| d.year()).or(nfo.year);

        let mut m = Metadata {
            title: nfo.title,
            plot: nfo.plot,
            rating: nfo.rating,
            year: year,
            premiered: premiered,
            official_rating: nfo.mpaa,
            genres: nfo.genre,
            studios: nfo.studio,
            actors: nfo.actor.into_iter().map(|a| a.name).collect(),
            directors: nfo.director,
            taglines: nfo.tagline,
            ..Default::default()
        };

        if let Some(fi) = nfo.fileinfo {
            apply_file_info(&mut m, fi);
        }

        m
    }
}

impl From<ShowNfo> for Metadata {
    fn from(nfo: ShowNfo) -> Self {
        let premiered = nfo.premiered.as_ref().and_then(|d| parse_iso8601_date(d));
        let year = premiered.map(|d| d.year()).or(nfo.year);

        Metadata {
            title: nfo.title,
            plot: nfo.plot,
            rating: nfo.rating,
            premiered: premiered,
            year: year,
            official_rating: nfo.mpaa,
            genres: nfo.genre,
            studios: nfo.studio,
            actors: nfo.actor.into_iter().map(|a| a.name).collect(),
            directors: nfo.director,
            taglines: nfo.tagline,
            ..Default::default()
        }
    }
}

impl From<EpisodeNfo> for Metadata {
    fn from(nfo: EpisodeNfo) -> Self {
        let mut m = Metadata {
            title: nfo.title,
            plot: nfo.plot,
            rating: nfo.rating,
            ..Default::default()
        };

        if let Some(fi) = nfo.fileinfo {
            apply_file_info(&mut m, fi);
        }

        m
    }
}

fn apply_file_info(m: &mut Metadata, fi: FileInfo) {
    if let Some(sd) = fi.streamdetails {
        if let Some(v) = sd.video {
            m.video_codec = v.codec;
            m.video_width = v.width;
            m.video_height = v.height;
            m.duration = v.durationinseconds.map(|d| std::time::Duration::from_secs(d as u64));
        }
        if let Some(a) = sd.audio {
            m.audio_codec = a.codec;
            m.audio_language = a.language;
            m.audio_channels = a.channels;
        }
    }
}
