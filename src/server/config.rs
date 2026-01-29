use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub listen: ListenConfig,
    #[serde(default)]
    pub appdir: Option<String>,
    #[serde(default)]
    pub cachedir: Option<String>,
    #[serde(default)]
    pub dbdir: Option<String>,
    #[serde(default)]
    pub database: DatabaseConfig,
    #[serde(default)]
    pub logfile: Option<String>,
    #[serde(default)]
    pub collections: Vec<CollectionConfig>,
    #[serde(default)]
    pub jellyfin: JellyfinConfig,
}

impl Config {
    // Convenience accessors for commonly used fields
    pub fn app_dir(&self) -> Option<String> {
        self.appdir.clone()
    }

    pub fn cache_dir(&self) -> Option<String> {
        self.cachedir.clone()
    }

    pub fn server_id(&self) -> Option<String> {
        self.jellyfin.server_id.clone()
    }

    pub fn server_name(&self) -> Option<String> {
        self.jellyfin.server_name.clone()
    }

    pub fn auto_register(&self) -> Option<bool> {
        Some(self.jellyfin.auto_register)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListenConfig {
    #[serde(default = "default_address")]
    pub address: String,
    #[serde(default = "default_port")]
    pub port: String,
    #[serde(default, rename = "tlscert")]
    pub tls_cert: Option<String>,
    #[serde(default, rename = "tlskey")]
    pub tls_key: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DatabaseConfig {
    #[serde(default)]
    pub sqlite: SqliteConfig,
    #[serde(default)]
    pub path: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SqliteConfig {
    #[serde(default)]
    pub filename: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionConfig {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub collection_type: String,
    pub directory: String,
    #[serde(default, rename = "baseurl")]
    pub base_url: Option<String>,
    #[serde(default, rename = "hlsserver")]
    pub hls_server: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct JellyfinConfig {
    #[serde(default, rename = "serverid")]
    pub server_id: Option<String>,
    #[serde(default, rename = "servername")]
    pub server_name: Option<String>,
    #[serde(default, rename = "autoregister")]
    pub auto_register: bool,
    #[serde(default = "default_image_quality", rename = "imagequalityposter")]
    pub image_quality_poster: u32,
}

fn default_address() -> String {
    "0.0.0.0".to_string()
}

fn default_port() -> String {
    "8096".to_string()
}

fn default_image_quality() -> u32 {
    90
}

impl Config {
    /// Load configuration from YAML file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let contents = fs::read_to_string(path.as_ref()).map_err(|e| ConfigError::Io(e))?;

        let config: Config = serde_yaml::from_str(&contents).map_err(|e| ConfigError::Parse(e))?;

        Ok(config)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Parse error: {0}")]
    Parse(#[from] serde_yaml::Error),
}
