use crate::sponsorblock::action::Action;
use crate::sponsorblock::category::Category;

use std::collections::HashSet;
use std::io::{Error, ErrorKind};

use serde_derive::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub server_address: String,
    #[serde(default)]
    categories: HashSet<Category>,
    #[serde(default)]
    action_types: HashSet<Action>,
    #[serde(default)]
    pub privacy_api: bool,
}

impl Config {
    pub fn parameters(&self) -> String {
        let categories = self.categories.iter().map(|v| format!("category={}", v));
        let action_types = self.action_types.iter().map(|v| format!("actionType={}", v));

        categories.chain(action_types).collect::<Vec<String>>().join("&")
    }
}

impl Default for Config {
    fn default() -> Self {
        dirs::config_dir()
            .ok_or(Error::new(ErrorKind::NotFound, "configuration directory not found"))
            .and_then(|dir| std::fs::read_to_string(dir.join("mpv/sponsorblock.toml")))
            .and_then(|data| toml::from_str(&data).map_err(|e| Error::new(ErrorKind::InvalidData, e)))
            .unwrap_or_else(|e| {
                log::warn!("Failed to load configuration file: {}. Falling back to default.", e);
                toml::from_str(include_str!("../sponsorblock.toml")).unwrap()
            })
    }
}
