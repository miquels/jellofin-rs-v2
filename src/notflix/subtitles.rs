use axum::{
    http::{header, HeaderMap, StatusCode},
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct SubEntry {
    pub id: i32,
    pub start: i32,
    pub end: i32,
    pub lines: Vec<String>,
}

pub fn open_sub(headers: &HeaderMap, name: &str) -> impl IntoResponse {
    let path = Path::new(name);
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");

    let mut srt_path = name.to_string();
    if ext == "vtt" {
        let stem = name.strip_suffix(".vtt").unwrap_or(name);
        srt_path = format!("{}.srt", stem);
    }

    if !Path::new(&srt_path).exists() {
        return StatusCode::NOT_FOUND.into_response();
    }

    let (subs, is_utf8) = match parse_srt(&srt_path) {
        Ok(s) => s,
        Err(_) => return StatusCode::NOT_FOUND.into_response(),
    };

    let charset = if is_utf8 { "charset=utf-8" } else { "charset=ISO-8859-1" };
    let accept = headers.get(header::ACCEPT).and_then(|v| v.to_str().ok()).unwrap_or("");

    if accept.contains("application/json") {
        let json = serde_json::to_string_pretty(&subs).unwrap_or_else(|_| "[]".to_string());
        return ([(header::CONTENT_TYPE, format!("application/json; {}", charset))], json).into_response();
    }

    if ext == "srt" && !accept.contains("text/vtt") {
        let content = fs::read_to_string(&srt_path).unwrap_or_default();
        return ([(header::CONTENT_TYPE, format!("text/plain; {}", charset))], content).into_response();
    }

    // Default to VTT
    let mut lines = vec!["WEBVTT".to_string(), "".to_string()];
    for sub in subs {
        let tm = format!("{} --> {}", vtt_time(sub.start), vtt_time(sub.end));
        lines.push(tm);
        for line in sub.lines {
            lines.push(line);
        }
        lines.push("".to_string());
    }
    let vtt = lines.join("\n");
    ([(header::CONTENT_TYPE, format!("text/vtt; {}", charset))], vtt).into_response()
}

fn vtt_time(ms: i32) -> String {
    let s = ms / 1000;
    let h = s / 3600;
    let m = (s / 60) % 60;
    let sec = s % 60;
    let msec = ms % 1000;
    format!("{:02}:{:02}:{:02}.{:03}", h, m, sec, msec)
}

fn parse_srt(path: &str) -> Result<(Vec<SubEntry>, bool), std::io::Error> {
    let file = fs::File::open(path)?;
    let reader = BufReader::new(file);
    let mut subs = Vec::new();
    let mut is_utf8 = false;
    let mut state = 0;
    let mut current_sub = SubEntry {
        id: 0,
        start: 0,
        end: 0,
        lines: Vec::new(),
    };

    for line in reader.lines() {
        let mut line = line?;

        // Handle BOM
        if line.starts_with("\u{FEFF}") {
            is_utf8 = true;
            line = line.strip_prefix("\u{FEFF}").unwrap().to_string();
        }

        match state {
            0 => {
                if let Ok(id) = line.trim().parse::<i32>() {
                    current_sub.id = id;
                    state = 1;
                }
            }
            1 => {
                let parts: Vec<&str> = line.split(" --> ").collect();
                if parts.len() == 2 {
                    current_sub.start = parse_time(parts[0]);
                    current_sub.end = parse_time(parts[1]);
                    state = 2;
                }
            }
            2 => {
                if line.trim().is_empty() {
                    subs.push(current_sub);
                    current_sub = SubEntry {
                        id: 0,
                        start: 0,
                        end: 0,
                        lines: Vec::new(),
                    };
                    state = 0;
                } else {
                    current_sub.lines.push(line);
                }
            }
            _ => {}
        }
    }

    if !current_sub.lines.is_empty() {
        subs.push(current_sub);
    }

    Ok((subs, is_utf8))
}

fn parse_time(time_str: &str) -> i32 {
    let parts: Vec<&str> = time_str.trim().split(|c| c == ':' || c == ',').collect();
    if parts.len() == 4 {
        let h: i32 = parts[0].parse().unwrap_or(0);
        let m: i32 = parts[1].parse().unwrap_or(0);
        let s: i32 = parts[2].parse().unwrap_or(0);
        let ms: i32 = parts[3].parse().unwrap_or(0);
        return (h * 3600 + m * 60 + s) * 1000 + ms;
    }
    0
}
