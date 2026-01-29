use axum::{http::header, http::HeaderMap, response::Json};

use super::types::*;

/// GET /Localization/Cultures
pub async fn localization_cultures() -> (HeaderMap, Json<Vec<Language>>) {
    let mut headers = HeaderMap::new();
    headers.insert(header::CACHE_CONTROL, "max-age=3600".parse().unwrap());

    let languages = vec![Language {
        display_name: "English".to_string(),
        name: "English".to_string(),
        three_letter_iso_language_name: "eng".to_string(),
        three_letter_iso_language_names: vec!["eng".to_string()],
        two_letter_iso_language_name: "en".to_string(),
    }];

    (headers, Json(languages))
}

/// GET /Localization/Options
pub async fn localization_options() -> (HeaderMap, Json<Vec<LocalizationOption>>) {
    let mut headers = HeaderMap::new();
    headers.insert(header::CACHE_CONTROL, "max-age=3600".parse().unwrap());

    let options = vec![LocalizationOption {
        name: "English".to_string(),
        value: "en-US".to_string(),
    }];

    (headers, Json(options))
}

/// GET /Localization/ParentalRatings
pub async fn localization_parental_ratings() -> (HeaderMap, Json<Vec<ParentalRating>>) {
    let mut headers = HeaderMap::new();
    headers.insert(header::CACHE_CONTROL, "max-age=3600".parse().unwrap());

    let ratings = vec![ParentalRating {
        name: "Unrated".to_string(),
        value: 0,
    }];

    (headers, Json(ratings))
}

/// GET /Localization/Countries
pub async fn localization_countries() -> (HeaderMap, Json<Vec<Country>>) {
    let mut headers = HeaderMap::new();
    headers.insert(header::CACHE_CONTROL, "max-age=3600".parse().unwrap());

    let countries = vec![Country {
        name: "United States".to_string(),
        two_letter_iso_region_name: "US".to_string(),
        three_letter_iso_region_name: "USA".to_string(),
    }];

    (headers, Json(countries))
}
