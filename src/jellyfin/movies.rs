use axum::{
    extract::{Query, State},
    response::Json,
    Extension,
};
use std::collections::{HashMap, HashSet};

use super::jellyfin::JellyfinState;
use super::jfitem::convert_items_to_dtos;
use super::types::*;
use crate::collection::item::Movie;
use crate::collection::{CollectionType, Item};
use crate::database::model;

/// GET /Movies/Recommendations - Get movie recommendations
pub async fn movies_recommendations(
    Extension(token): Extension<model::AccessToken>,
    State(state): State<JellyfinState>,
    Query(query_params): Query<HashMap<String, String>>,
) -> Json<Vec<RecommendationDto>> {
    let category_limit = query_params
        .get("categoryLimit")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(5);
    let item_limit = query_params
        .get("itemLimit")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(8);

    // Gather seed movies from recently watched and favorites
    let recent_ids = state
        .repo
        .get_recently_watched(&token.user_id, true, 20)
        .await
        .unwrap_or_default();
    let favorite_ids = state.repo.get_favorites(&token.user_id).await.unwrap_or_default();

    let recent_movies: Vec<Movie> = recent_ids
        .iter()
        .filter_map(|id| match state.collections.get_item_by_id(id) {
            Some((_, Item::Movie(m))) => Some(m),
            _ => None,
        })
        .collect();

    let favorite_movies: Vec<Movie> = favorite_ids
        .iter()
        .filter_map(|id| match state.collections.get_item_by_id(id) {
            Some((_, Item::Movie(m))) => Some(m),
            _ => None,
        })
        .collect();

    // Build candidate pool: all movies from movie collections
    let mut all_movies: Vec<Movie> = Vec::new();
    for c in state.collections.get_collections() {
        if c.collection_type != CollectionType::Movies {
            continue;
        }
        for item in c.items {
            if let Item::Movie(m) = item {
                all_movies.push(m);
            }
        }
    }

    // Track which movies have been recommended to avoid duplicates
    let mut used_ids: HashSet<String> = HashSet::new();
    // Seed movies shouldn't appear in recommendations
    for m in &recent_movies {
        used_ids.insert(m.id.clone());
    }
    for m in &favorite_movies {
        used_ids.insert(m.id.clone());
    }

    let mut groups: Vec<RecommendationDto> = Vec::new();

    // "Similar to recently played" groups
    for seed in &recent_movies {
        if groups.len() >= category_limit {
            break;
        }
        let seed_genres: HashSet<&str> = seed.metadata.genres.iter().map(|s| s.as_str()).collect();
        let seed_directors: HashSet<&str> = seed.metadata.directors.iter().map(|s| s.as_str()).collect();

        let matches = find_similar_movies(&all_movies, &seed_genres, &seed_directors, &used_ids, item_limit);
        if matches.is_empty() {
            continue;
        }
        for m in &matches {
            used_ids.insert(m.id.clone());
        }

        let items_as_items: Vec<Item> = matches.into_iter().map(Item::Movie).collect();
        let items = convert_items_to_dtos(&items_as_items, &state, &token.user_id).await;

        let name = seed.metadata.title.as_deref().unwrap_or(&seed.name);
        groups.push(RecommendationDto {
            items,
            recommendation_type: "SimilarToRecentlyPlayed".to_string(),
            category_id: seed.id.clone(),
            baseline_item_name: name.to_string(),
        });
    }

    // "Similar to liked item" groups
    for seed in &favorite_movies {
        if groups.len() >= category_limit {
            break;
        }
        let seed_genres: HashSet<&str> = seed.metadata.genres.iter().map(|s| s.as_str()).collect();
        let seed_directors: HashSet<&str> = seed.metadata.directors.iter().map(|s| s.as_str()).collect();

        let matches = find_similar_movies(&all_movies, &seed_genres, &seed_directors, &used_ids, item_limit);
        if matches.is_empty() {
            continue;
        }
        for m in &matches {
            used_ids.insert(m.id.clone());
        }

        let items_as_items: Vec<Item> = matches.into_iter().map(Item::Movie).collect();
        let items = convert_items_to_dtos(&items_as_items, &state, &token.user_id).await;

        let name = seed.metadata.title.as_deref().unwrap_or(&seed.name);
        groups.push(RecommendationDto {
            items,
            recommendation_type: "SimilarToLikedItem".to_string(),
            category_id: seed.id.clone(),
            baseline_item_name: name.to_string(),
        });
    }

    // Director-based groups from recently watched
    for seed in &recent_movies {
        if groups.len() >= category_limit {
            break;
        }
        for director in &seed.metadata.directors {
            if groups.len() >= category_limit {
                break;
            }
            let matches: Vec<Movie> = all_movies
                .iter()
                .filter(|m| !used_ids.contains(&m.id) && m.metadata.directors.contains(director))
                .take(item_limit)
                .cloned()
                .collect();
            if matches.is_empty() {
                continue;
            }
            for m in &matches {
                used_ids.insert(m.id.clone());
            }

            let items_as_items: Vec<Item> = matches.into_iter().map(Item::Movie).collect();
            let items = convert_items_to_dtos(&items_as_items, &state, &token.user_id).await;

            groups.push(RecommendationDto {
                items,
                recommendation_type: "HasDirectorFromRecentlyPlayed".to_string(),
                category_id: seed.id.clone(),
                baseline_item_name: director.clone(),
            });
        }
    }

    // Actor-based groups from recently watched
    for seed in &recent_movies {
        if groups.len() >= category_limit {
            break;
        }
        for actor in seed.metadata.actors.iter().take(3) {
            if groups.len() >= category_limit {
                break;
            }
            let matches: Vec<Movie> = all_movies
                .iter()
                .filter(|m| !used_ids.contains(&m.id) && m.metadata.actors.contains(actor))
                .take(item_limit)
                .cloned()
                .collect();
            if matches.is_empty() {
                continue;
            }
            for m in &matches {
                used_ids.insert(m.id.clone());
            }

            let items_as_items: Vec<Item> = matches.into_iter().map(Item::Movie).collect();
            let items = convert_items_to_dtos(&items_as_items, &state, &token.user_id).await;

            groups.push(RecommendationDto {
                items,
                recommendation_type: "HasActorFromRecentlyPlayed".to_string(),
                category_id: seed.id.clone(),
                baseline_item_name: actor.clone(),
            });
        }
    }

    Json(groups)
}

/// Find movies similar to a seed by genre and director overlap.
/// Returns up to `limit` movies sorted by overlap score (descending).
fn find_similar_movies(
    candidates: &[Movie],
    seed_genres: &HashSet<&str>,
    seed_directors: &HashSet<&str>,
    exclude: &HashSet<String>,
    limit: usize,
) -> Vec<Movie> {
    if seed_genres.is_empty() && seed_directors.is_empty() {
        return Vec::new();
    }

    let mut scored: Vec<(&Movie, usize)> = candidates
        .iter()
        .filter(|m| !exclude.contains(&m.id))
        .filter_map(|m| {
            let genre_score = m
                .metadata
                .genres
                .iter()
                .filter(|g| seed_genres.contains(g.as_str()))
                .count();
            let director_score = m
                .metadata
                .directors
                .iter()
                .filter(|d| seed_directors.contains(d.as_str()))
                .count();
            let score = genre_score + director_score * 2;
            if score > 0 {
                Some((m, score))
            } else {
                None
            }
        })
        .collect();

    scored.sort_by(|a, b| b.1.cmp(&a.1));
    scored.into_iter().take(limit).map(|(m, _)| m.clone()).collect()
}
