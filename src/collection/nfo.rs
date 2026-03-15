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
    bitrate: Option<i32>,
    codec: Option<String>,
    width: Option<i32>,
    height: Option<i32>,
    duration: Option<f32>,
    durationinseconds: Option<i32>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "lowercase")]
struct AudioDetails {
    bitrate: Option<i32>,
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

fn calc_duration(secs: Option<i32>, mins: Option<f32>) -> Option<std::time::Duration> {
    if let Some(s) = secs {
        Some(std::time::Duration::from_secs(s as u64))
    } else if let Some(m) = mins {
        Some(std::time::Duration::from_secs((m * 60.0) as u64))
    } else {
        None
    }
}

fn apply_file_info(m: &mut Metadata, fi: FileInfo) {
    if let Some(sd) = fi.streamdetails {
        if let Some(v) = sd.video {
            m.video_codec = v.codec;
            m.video_width = v.width;
            m.video_height = v.height;
            // If it's smaller than 500_0000, it's in kbps, otherwise it's in bps
            m.video_bitrate = v.bitrate.map(|b| if b < 500_0000 { b * 1000 } else { b });
            m.duration = calc_duration(v.durationinseconds, v.duration);
        }
        if let Some(a) = sd.audio {
            m.audio_codec = a.codec;
            m.audio_language = a.language;
            m.audio_channels = a.channels;
            // If it's smaller than 100_000, it's in kbps, otherwise it's in bps
            m.audio_bitrate = a.bitrate.map(|b| if b < 100_000 { b * 1000 } else { b });
        }
    }
}
