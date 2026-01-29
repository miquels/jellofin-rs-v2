use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collection {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub collection_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Vec<Item>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub id: String,
    pub name: String,
    pub path: String,
    pub baseurl: String,
    #[serde(rename = "type")]
    pub item_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub firstvideo: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lastvideo: Option<i64>,
    #[serde(rename = "sortName", skip_serializing_if = "Option::is_none")]
    pub sort_name: Option<String>,
    pub nfo: ItemNfo,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub banner: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fanart: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folder: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub poster: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rating: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub votes: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub genre: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub year: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumb: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub srtsubs: Option<Vec<Subs>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vttsubs: Option<Vec<Subs>>,
    #[serde(rename = "seasonAllBanner", skip_serializing_if = "Option::is_none")]
    pub season_all_banner: Option<String>,
    #[serde(rename = "seasonAllPoster", skip_serializing_if = "Option::is_none")]
    pub season_all_poster: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seasons: Option<Vec<Season>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemNfo {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plot: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub genre: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub premiered: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mpaa: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aired: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub studio: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rating: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Season {
    pub seasonno: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub banner: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fanart: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub poster: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub episodes: Option<Vec<Episode>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Episode {
    pub name: String,
    pub seasonno: i32,
    pub episodeno: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub double: Option<bool>,
    #[serde(rename = "sortName", skip_serializing_if = "Option::is_none")]
    pub sort_name: Option<String>,
    pub nfo: EpisodeNfo,
    pub video: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumb: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub srtsubs: Option<Vec<Subs>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vttsubs: Option<Vec<Subs>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodeNfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plot: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub season: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub episode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aired: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subs {
    pub lang: String,
    pub path: String,
}
