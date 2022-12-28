use crate::sponsorblock::Action;
use crate::sponsorblock::Category;

use std::collections::HashSet;
use std::io::{Error, ErrorKind};

use reqwest::Url;
use serde_derive::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub server_address: Url,
    #[serde(default)]
    pub categories: HashSet<Category>,
    #[serde(default)]
    pub action_types: HashSet<Action>,
    #[serde(default)]
    pub privacy_api: bool,
}

impl Default for Config {
    fn default() -> Self {
        dirs::config_dir()
            .ok_or(Error::new(ErrorKind::NotFound, "configuration directory not found"))
            .and_then(|dir| std::fs::read_to_string(dir.join("mpv/sponsorblock.toml")))
            .and_then(|data| toml::from_str(&data).map_err(|e| Error::new(ErrorKind::InvalidData, e)))
            .unwrap_or_else(|e| {
                log::warn!("Failed to load configuration file: {}. Falling back to default", e);
                toml::from_str(include_str!("../sponsorblock.toml")).unwrap()
            })
    }
}
