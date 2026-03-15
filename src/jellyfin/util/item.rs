use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use rand::prelude::*;
use std::collections::HashMap;
use tracing::warn;

use crate::collection::Item;
use crate::idhash::*;
use super::types::*;

// ---------------------------------------------------------------------------
// Item-based filtering (operates on native types, not BaseItemDto)
// ---------------------------------------------------------------------------

pub(crate) fn apply_query_items_filter(items: Vec<Item>, query_params: &HashMap<String, String>) -> Vec<Item> {
    items
        .into_iter()
        .filter(|item| apply_query_item_filter(item, query_params))
        .collect()
}

fn apply_query_item_filter(item: &Item, qp: &HashMap<String, String>) -> bool {

    // includeItemTypes
    if let Some(types) = qp.get("includeItemTypes") {
        let type_list: Vec<&str> = types.split(',').collect();
        if !type_list.contains(&item.jf_type()) {
            return false;
        }
    }

    // excludeItemTypes
    if let Some(types) = qp.get("excludeItemTypes") {
        let type_list: Vec<&str> = types.split(',').collect();
        if type_list.contains(&item.jf_type()) {
            return false;
        }
    }

    // isHd
    if let Some(hd) = qp.get("isHd") {
        let want_hd = hd.eq_ignore_ascii_case("true");
        if item.is_hd() != want_hd {
            return false;
        }
    }

    // is4K
    if let Some(k4) = qp.get("is4K") {
        let want_4k = k4.eq_ignore_ascii_case("true");
        if item.is_4k() != want_4k {
            return false;
        }
    }

    // ids
    if let Some(ids) = qp.get("ids") {
        let id = item.id();
        let id_list: Vec<&str> = ids.split(',').collect();
        if !id_list.contains(&id.as_str()) {
            return false;
        }
    }

    // excludeItemIds
    if let Some(exclude_ids) = qp.get("excludeItemIds") {
        let id = item.id();
        for eid in exclude_ids.split(',') {
            if id == eid {
                return false;
            }
        }
    }

    // genreIds (pipe-separated)
    if let Some(genre_ids) = qp.get("genreIds") {
        let item_genre_ids: Vec<String> = item
            .genres()
            .iter()
            .map(|g| id_hash_prefix(ITEM_PREFIX_GENRE, g))
            .collect();
        let mut keep = false;
        for gid in genre_ids.split('|') {
            if item_genre_ids.iter().any(|ig| ig == gid) {
                keep = true;
                break;
            }
        }
        if !keep {
            return false;
        }
    }

    // studioIds (pipe-separated)
    if let Some(studio_ids) = qp.get("studioIds") {
        let item_studio_ids: Vec<String> = item
            .studios()
            .iter()
            .map(|s| id_hash_prefix(ITEM_PREFIX_STUDIO, s))
            .collect();
        let mut keep = false;
        for sid in studio_ids.split('|') {
            if item_studio_ids.iter().any(|is| is == sid) {
                keep = true;
                break;
            }
        }
        if !keep {
            return false;
        }
    }

    // parentIndexNumber
    if let Some(pin_str) = qp.get("parentIndexNumber") {
        if let Ok(pin) = pin_str.parse::<i32>() {
            if item.parent_index_number() != Some(pin) {
                return false;
            }
        }
    }

    // indexNumber
    if let Some(in_str) = qp.get("indexNumber") {
        if let Ok(idx) = in_str.parse::<i32>() {
            if item.index_number() != Some(idx) {
                return false;
            }
        }
    }

    // nameStartsWith (case-insensitive)
    if let Some(prefix) = qp.get("nameStartsWith") {
        if !item.sort_name().to_lowercase().starts_with(&prefix.to_lowercase()) {
            return false;
        }
    }

    // nameStartsWithOrGreater (case-insensitive)
    if let Some(bound) = qp.get("nameStartsWithOrGreater") {
        if item.sort_name().to_lowercase() < bound.to_lowercase() {
            return false;
        }
    }

    // nameLessThan (case-insensitive)
    if let Some(bound) = qp.get("nameLessThan") {
        if item.sort_name().to_lowercase() > bound.to_lowercase() {
            return false;
        }
    }

    // genres (by name, pipe-separated)
    if let Some(include_genres) = qp.get("genres") {
        let item_genres = item.genres();
        let mut keep = false;
        for g in include_genres.split('|') {
            if item_genres.iter().any(|ig| ig == g) {
                keep = true;
                break;
            }
        }
        if !keep {
            return false;
        }
    }

    // studios (by name, pipe-separated)
    if let Some(include_studios) = qp.get("studios") {
        let item_studios = item.studios();
        let mut keep = false;
        for s in include_studios.split('|') {
            if item_studios.iter().any(|is| is == s) {
                keep = true;
                break;
            }
        }
        if !keep {
            return false;
        }
    }

    // officialRatings (pipe-separated)
    if let Some(ratings) = qp.get("officialRatings") {
        let mut keep = false;
        for r in ratings.split('|') {
            if item.official_rating() == Some(r) {
                keep = true;
                break;
            }
        }
        if !keep {
            return false;
        }
    }

    // minCommunityRating
    if let Some(min_str) = qp.get("minCommunityRating") {
        if let Ok(min) = min_str.parse::<f32>() {
            if item.community_rating().unwrap_or(0.0) < min {
                return false;
            }
        }
    }

    // minPremiereDate
    if let Some(date_str) = qp.get("minPremiereDate") {
        if let Some(min_date) = parse_iso8601_date(date_str) {
            match item.premiere_date() {
                Some(pd) if pd >= min_date => {}
                _ => return false,
            }
        }
    }

    // maxPremiereDate
    if let Some(date_str) = qp.get("maxPremiereDate") {
        if let Some(max_date) = parse_iso8601_date(date_str) {
            match item.premiere_date() {
                Some(pd) if pd <= max_date => {}
                _ => return false,
            }
        }
    }

    // years (comma-separated)
    if let Some(years_str) = qp.get("years") {
        let mut keep = false;
        for y in years_str.split(',') {
            if let Ok(year) = y.parse::<i32>() {
                if item.production_year() == Some(year) {
                    keep = true;
                    break;
                }
            }
        }
        if !keep {
            return false;
        }
    }

    // isPlayed (requires user_data)
    if let Some(played_str) = qp.get("isPlayed") {
        let want_played = played_str.eq_ignore_ascii_case("true");
        let is_played = item.get_user_data().map(|ud| ud.played).unwrap_or(false);
        if want_played != is_played {
            return false;
        }
    }

    // isFavorite (requires user_data)
    if let Some(fav_str) = qp.get("isFavorite") {
        let want_fav = fav_str.eq_ignore_ascii_case("true");
        let is_fav = item.get_user_data().map(|ud| ud.favorite).unwrap_or(false);
        if want_fav != is_fav {
            return false;
        }
    }

    // filters (comma-separated, e.g. "IsFavorite", "IsFavoriteOrLikes")
    if let Some(filters) = qp.get("filters") {
        for f in filters.split(',') {
            match f {
                "IsFavorite" | "IsFavoriteOrLikes" => {
                    let is_fav = item.get_user_data().map(|ud| ud.favorite).unwrap_or(false);
                    if !is_fav {
                        return false;
                    }
                }
                _ => {}
            }
        }
    }

    // searchTerm
    if let Some(term) = qp.get("searchTerm") {
        let id = item.id();
        let media = is_jf_movie_id(&id) || is_jf_show_id(&id) || is_jf_season_id(&id) || is_jf_episode_id(&id);
        if media && !item.name().to_lowercase().contains(&term.to_lowercase()) {
            return false;
        }
    }

    true
}

// ---------------------------------------------------------------------------
// Item-based sorting
// ---------------------------------------------------------------------------

pub(crate) fn apply_query_item_sorting(
    items: &mut Vec<Item>,
    query_params: &HashMap<String, String>,
) {
    let sort_by_raw = match query_params.get("sortBy") {
        Some(s) if !s.is_empty() => s.clone(),
        _ => return,
    };
    let sort_fields: Vec<String> = sort_by_raw.split(',').map(|s| s.to_lowercase()).collect();

    let descending = query_params
        .get("sortOrder")
        .map(|s| s.eq_ignore_ascii_case("descending"))
        .unwrap_or(false);

    items.sort_by(|a, b| {
        for field in &sort_fields {
            let ord = match field.as_str() {
                "communityrating" => {
                    let ar = a.community_rating().unwrap_or(0.0);
                    let br = b.community_rating().unwrap_or(0.0);
                    ar.partial_cmp(&br).unwrap_or(std::cmp::Ordering::Equal)
                }
                "datecreated" | "datelastcontentadded" => a.created().cmp(&b.created()),
                "dateplayed" => {
                    let ad = a.get_user_data().map(|ud| ud.timestamp);
                    let bd = b.get_user_data().map(|ud| ud.timestamp);
                    ad.cmp(&bd)
                }
                "indexnumber" => a.index_number().cmp(&b.index_number()),
                "isfavoriteorliked" => {
                    let af = a.get_user_data().map(|ud| ud.favorite).unwrap_or(false);
                    let bf = b.get_user_data().map(|ud| ud.favorite).unwrap_or(false);
                    af.cmp(&bf)
                }
                "isfolder" => a.is_folder().cmp(&b.is_folder()),
                "isplayed" => {
                    let ap = a.get_user_data().map(|ud| ud.played).unwrap_or(false);
                    let bp = b.get_user_data().map(|ud| ud.played).unwrap_or(false);
                    ap.cmp(&bp)
                }
                "isunplayed" => {
                    let ap = !a.get_user_data().map(|ud| ud.played).unwrap_or(false);
                    let bp = !b.get_user_data().map(|ud| ud.played).unwrap_or(false);
                    ap.cmp(&bp)
                }
                "officialrating" => a.official_rating().cmp(&b.official_rating()),
                "parentindexnumber" => a.parent_index_number().cmp(&b.parent_index_number()),
                "premieredate" => a.premiere_date().cmp(&b.premiere_date()),
                "productionyear" => a.production_year().cmp(&b.production_year()),
                "random" => {
                    let mut rng = rand::thread_rng();
                    if rng.gen_bool(0.5) {
                        std::cmp::Ordering::Less
                    } else {
                        std::cmp::Ordering::Greater
                    }
                }
                "runtime" => a.run_time_ticks().cmp(&b.run_time_ticks()),
                "name" | "seriessortname" | "sortname" | "default" => {
                    a.sort_name().cmp(b.sort_name())
                }
                other => {
                    warn!("apply_query_item_sorting: unknown sort field: {}", other);
                    std::cmp::Ordering::Equal
                }
            };
            if ord != std::cmp::Ordering::Equal {
                return if descending { ord.reverse() } else { ord };
            }
        }
        std::cmp::Ordering::Equal
    });
}

// ---------------------------------------------------------------------------
// Item-based pagination
// ---------------------------------------------------------------------------

pub(crate) fn apply_query_item_pagination(
    items: Vec<Item>,
    query_params: &HashMap<String, String>,
) -> (Vec<Item>, i32) {
    let start_index = query_params
        .get("startIndex")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(0);
    let limit = query_params.get("limit").and_then(|v| v.parse::<usize>().ok());

    let total = items.len();
    if start_index >= total {
        return (Vec::new(), start_index as i32);
    }

    let end = if let Some(l) = limit {
        std::cmp::min(start_index + l, total)
    } else {
        total
    };

    let paged: Vec<Item> = items.into_iter().skip(start_index).take(end - start_index).collect();
    (paged, start_index as i32)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Parse an ISO 8601 date string into a DateTime<Utc>.
/// Tries multiple formats: RFC3339, datetime, date-only, year-month, year.
pub fn parse_iso8601_date(input: &str) -> Option<DateTime<Utc>> {
    // Try RFC3339 / full datetime with timezone
    if let Ok(dt) = DateTime::parse_from_rfc3339(input) {
        return Some(dt.with_timezone(&Utc));
    }
    // Try "2006-01-02 15:04:05"
    if let Ok(ndt) = NaiveDateTime::parse_from_str(input, "%Y-%m-%d %H:%M:%S") {
        return Some(ndt.and_utc());
    }
    // Try "2006-01-02"
    if let Ok(nd) = NaiveDate::parse_from_str(input, "%Y-%m-%d") {
        return nd.and_hms_opt(0, 0, 0).map(|ndt| ndt.and_utc());
    }
    // Try "2006-01"
    if input.len() == 7 {
        if let Ok(nd) = NaiveDate::parse_from_str(&format!("{}-01", input), "%Y-%m-%d") {
            return nd.and_hms_opt(0, 0, 0).map(|ndt| ndt.and_utc());
        }
    }
    // Try "2006" (year only)
    if input.len() == 4 {
        if let Ok(year) = input.parse::<i32>() {
            return NaiveDate::from_ymd_opt(year, 1, 1)
                .and_then(|nd| nd.and_hms_opt(0, 0, 0))
                .map(|ndt| ndt.and_utc());
        }
    }
    None
}

// ---------------------------------------------------------------------------
// BaseItemDto-based filtering
// ---------------------------------------------------------------------------

pub(crate) fn apply_item_filter(i: &BaseItemDto, qp: &HashMap<String, String>) -> bool {
    // includeItemTypes
    if let Some(types) = qp.get("includeItemTypes") {
        let type_list: Vec<&str> = types.split(',').collect();
        if !type_list.contains(&i.item_type.as_str()) {
            return false;
        }
    }

    // excludeItemTypes
    if let Some(types) = qp.get("excludeItemTypes") {
        let type_list: Vec<&str> = types.split(',').collect();
        if type_list.contains(&i.item_type.as_str()) {
            return false;
        }
    }

    // isHd
    if let Some(hd) = qp.get("isHd") {
        let want_hd = hd.eq_ignore_ascii_case("true");
        if i.is_hd != want_hd {
            return false;
        }
    }

    // is4K
    if let Some(k4) = qp.get("is4K") {
        let want_4k = k4.eq_ignore_ascii_case("true");
        if i.is_4k != want_4k {
            return false;
        }
    }

    // ids
    if let Some(ids) = qp.get("ids") {
        let id_list: Vec<&str> = ids.split(',').collect();
        if !id_list.contains(&i.id.as_str()) {
            return false;
        }
    }

    // excludeItemIds
    if let Some(exclude_ids) = qp.get("excludeItemIds") {
        for eid in exclude_ids.split(',') {
            if i.id == eid {
                return false;
            }
        }
    }

    // genreIds (pipe-separated)
    if let Some(genre_ids) = qp.get("genreIds") {
        let mut keep = false;
        for gid in genre_ids.split('|') {
            for genre_item in &i.genre_items {
                if genre_item.id == gid {
                    keep = true;
                }
            }
        }
        if !keep {
            return false;
        }
    }

    // studioIds (pipe-separated)
    if let Some(studio_ids) = qp.get("studioIds") {
        let mut keep = false;
        for sid in studio_ids.split('|') {
            for studio in &i.studios {
                if studio.id == sid {
                    keep = true;
                }
            }
        }
        if !keep {
            return false;
        }
    }

    // seriesId
    if let Some(series_id) = qp.get("seriesId") {
        if i.series_id.as_deref() != Some(series_id.as_str()) {
            return false;
        }
    }

    // seasonId
    if let Some(season_id) = qp.get("seasonId") {
        if i.season_id.as_deref() != Some(season_id.as_str()) {
            return false;
        }
    }

    // parentIndexNumber
    if let Some(pin_str) = qp.get("parentIndexNumber") {
        if let Ok(pin) = pin_str.parse::<i32>() {
            if i.parent_index_number != Some(pin) {
                return false;
            }
        }
    }

    // indexNumber
    if let Some(in_str) = qp.get("indexNumber") {
        if let Ok(idx) = in_str.parse::<i32>() {
            if i.index_number != Some(idx) {
                return false;
            }
        }
    }

    // nameStartsWith (case-insensitive)
    if let Some(prefix) = qp.get("nameStartsWith") {
        let sort = i.sort_name.as_deref().unwrap_or(&i.name);
        if !sort.to_lowercase().starts_with(&prefix.to_lowercase()) {
            return false;
        }
    }

    // nameStartsWithOrGreater (case-insensitive)
    if let Some(bound) = qp.get("nameStartsWithOrGreater") {
        let sort = i.sort_name.as_deref().unwrap_or(&i.name);
        if sort.to_lowercase() < bound.to_lowercase() {
            return false;
        }
    }

    // nameLessThan (case-insensitive)
    if let Some(bound) = qp.get("nameLessThan") {
        let sort = i.sort_name.as_deref().unwrap_or(&i.name);
        if sort.to_lowercase() > bound.to_lowercase() {
            return false;
        }
    }

    // genres (by name, pipe-separated)
    if let Some(include_genres) = qp.get("genres") {
        let mut keep = false;
        for g in include_genres.split('|') {
            if i.genres.contains(&g.to_string()) {
                keep = true;
            }
        }
        if !keep {
            return false;
        }
    }

    // studios (by name, pipe-separated)
    if let Some(include_studios) = qp.get("studios") {
        let mut keep = false;
        for s in include_studios.split('|') {
            for studio in &i.studios {
                if studio.name == s {
                    keep = true;
                }
            }
        }
        if !keep {
            return false;
        }
    }

    // officialRatings (pipe-separated)
    if let Some(ratings) = qp.get("officialRatings") {
        let mut keep = false;
        for r in ratings.split('|') {
            if i.official_rating.as_deref() == Some(r) {
                keep = true;
            }
        }
        if !keep {
            return false;
        }
    }

    // minCommunityRating
    if let Some(min_str) = qp.get("minCommunityRating") {
        if let Ok(min) = min_str.parse::<f32>() {
            if i.community_rating.unwrap_or(0.0) < min {
                return false;
            }
        }
    }

    // minCriticRating
    if let Some(min_str) = qp.get("minCriticRating") {
        if let Ok(min) = min_str.parse::<f32>() {
            if i.critic_rating.unwrap_or(0.0) < min {
                return false;
            }
        }
    }

    // minPremiereDate
    if let Some(date_str) = qp.get("minPremiereDate") {
        if let Some(min_date) = parse_iso8601_date(date_str) {
            if let Some(ref pd) = i.premiere_date {
                if *pd < min_date {
                    return false;
                }
            } else {
                return false;
            }
        }
    }

    // maxPremiereDate
    if let Some(date_str) = qp.get("maxPremiereDate") {
        if let Some(max_date) = parse_iso8601_date(date_str) {
            if let Some(ref pd) = i.premiere_date {
                if *pd > max_date {
                    return false;
                }
            } else {
                return false;
            }
        }
    }

    // years (comma-separated)
    if let Some(years_str) = qp.get("years") {
        let mut keep = false;
        for y in years_str.split(',') {
            if let Ok(year) = y.parse::<i32>() {
                if i.production_year == Some(year) {
                    keep = true;
                }
            }
        }
        if !keep {
            return false;
        }
    }

    // isPlayed
    if let Some(played_str) = qp.get("isPlayed") {
        let want_played = played_str.eq_ignore_ascii_case("true");
        let is_played = i.user_data.as_ref().map(|ud| ud.played).unwrap_or(false);
        if want_played != is_played {
            return false;
        }
    }

    // isFavorite
    if let Some(fav_str) = qp.get("isFavorite") {
        let want_fav = fav_str.eq_ignore_ascii_case("true");
        let is_fav = i.user_data.as_ref().map(|ud| ud.is_favorite).unwrap_or(false);
        if want_fav != is_fav {
            return false;
        }
    }

    // filters (comma-separated, e.g. "IsFavorite", "IsFavoriteOrLikes")
    if let Some(filters) = qp.get("filters") {
        for f in filters.split(',') {
            match f {
                "IsFavorite" | "IsFavoriteOrLikes" => {
                    let is_fav = i.user_data.as_ref().map(|ud| ud.is_favorite).unwrap_or(false);
                    if !is_fav {
                        return false;
                    }
                }
                _ => {}
            }
        }
    }

    // searchTerm
    if let Some(term) = qp.get("searchTerm") {
        let media = is_jf_movie_id(&i.id) || is_jf_show_id(&i.id) || is_jf_season_id(&i.id) || is_jf_episode_id(&i.id);
        if media && !i.name.to_lowercase().contains(&term.to_lowercase()) {
            return false;
        }
    }

    true
}

// ---------------------------------------------------------------------------
// BaseItemDto-based sorting
// ---------------------------------------------------------------------------

pub(crate) fn apply_item_sorting(
    mut items: Vec<BaseItemDto>,
    query_params: &HashMap<String, String>,
) -> Vec<BaseItemDto> {
    let sort_by_raw = match query_params.get("sortBy") {
        Some(s) if !s.is_empty() => s.clone(),
        _ => return items,
    };
    let sort_fields: Vec<String> = sort_by_raw.split(',').map(|s| s.to_lowercase()).collect();

    let descending = query_params
        .get("sortOrder")
        .map(|s| s.eq_ignore_ascii_case("descending"))
        .unwrap_or(false);

    items.sort_by(|a, b| {
        let a_sort = a.sort_name.as_deref().unwrap_or(&a.name);
        let b_sort = b.sort_name.as_deref().unwrap_or(&b.name);

        for field in &sort_fields {
            let ord = match field.as_str() {
                "communityrating" => {
                    let ar = a.community_rating.unwrap_or(0.0);
                    let br = b.community_rating.unwrap_or(0.0);
                    ar.partial_cmp(&br).unwrap_or(std::cmp::Ordering::Equal)
                }
                "criticrating" => {
                    let ar = a.critic_rating.unwrap_or(0.0);
                    let br = b.critic_rating.unwrap_or(0.0);
                    ar.partial_cmp(&br).unwrap_or(std::cmp::Ordering::Equal)
                }
                "datecreated" | "datelastcontentadded" => a.date_created.cmp(&b.date_created),
                "dateplayed" => {
                    let ad = a.user_data.as_ref().and_then(|ud| ud.last_played_date);
                    let bd = b.user_data.as_ref().and_then(|ud| ud.last_played_date);
                    ad.cmp(&bd)
                }
                "indexnumber" => a.index_number.cmp(&b.index_number),
                "isfavoriteorliked" => {
                    let af = a.user_data.as_ref().map(|ud| ud.is_favorite).unwrap_or(false);
                    let bf = b.user_data.as_ref().map(|ud| ud.is_favorite).unwrap_or(false);
                    af.cmp(&bf)
                }
                "isfolder" => a.is_folder.cmp(&b.is_folder),
                "isplayed" => {
                    let ap = a.user_data.as_ref().map(|ud| ud.played).unwrap_or(false);
                    let bp = b.user_data.as_ref().map(|ud| ud.played).unwrap_or(false);
                    ap.cmp(&bp)
                }
                "isunplayed" => {
                    let ap = !a.user_data.as_ref().map(|ud| ud.played).unwrap_or(false);
                    let bp = !b.user_data.as_ref().map(|ud| ud.played).unwrap_or(false);
                    ap.cmp(&bp)
                }
                "officialrating" => a.official_rating.cmp(&b.official_rating),
                "parentindexnumber" => a.parent_index_number.cmp(&b.parent_index_number),
                "premieredate" => a.premiere_date.cmp(&b.premiere_date),
                "productionyear" => a.production_year.cmp(&b.production_year),
                "random" => {
                    let mut rng = rand::thread_rng();
                    if rng.gen_bool(0.5) {
                        std::cmp::Ordering::Less
                    } else {
                        std::cmp::Ordering::Greater
                    }
                }
                "runtime" => a.run_time_ticks.cmp(&b.run_time_ticks),
                "name" | "seriessortname" | "sortname" | "default" => a_sort.cmp(b_sort),
                other => {
                    warn!("apply_item_sorting: unknown sort field: {}", other);
                    std::cmp::Ordering::Equal
                }
            };
            if ord != std::cmp::Ordering::Equal {
                return if descending { ord.reverse() } else { ord };
            }
        }
        std::cmp::Ordering::Equal
    });
    items
}

// ---------------------------------------------------------------------------
// BaseItemDto-based pagination
// ---------------------------------------------------------------------------

pub(crate) fn apply_item_pagination(
    items: Vec<BaseItemDto>,
    query_params: &HashMap<String, String>,
) -> (Vec<BaseItemDto>, i32) {
    let start_index = query_params
        .get("startIndex")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(0);
    let limit = query_params.get("limit").and_then(|v| v.parse::<usize>().ok());

    let total = items.len();
    if start_index >= total {
        return (Vec::new(), start_index as i32);
    }

    let end = if let Some(l) = limit {
        std::cmp::min(start_index + l, total)
    } else {
        total
    };

    (items[start_index..end].to_vec(), start_index as i32)
}
