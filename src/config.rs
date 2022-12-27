use crate::sponsorblock::action::Action;
use crate::sponsorblock::category::Category;

use std::collections::HashSet;
use std::io::{Error, ErrorKind};

use serde_derive::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    #[serde(default = "Config::default_server_address")]
    pub server_address: String,
    #[serde(default = "HashSet::default")]
    categories: HashSet<Category>,
    #[serde(default = "HashSet::default")]
    action_types: HashSet<Action>,
    #[serde(default = "bool::default")]
    pub privacy_api: bool,
}

impl Config {
    fn default_server_address() -> String {
        "https://sponsor.ajay.app".to_string()
    }

    pub fn parameters(&self) -> String {
        let categories = self.categories.iter().map(|v| format!("category={}", v));
        let action_types = self.action_types.iter().map(|v| format!("actionType={}", v));

        categories.chain(action_types).collect::<Vec<String>>().join("&")
    }
}

impl Default for Config {
    fn default() -> Self {
        dirs::config_dir()
            .ok_or(Error::new(
                ErrorKind::NotFound,
                "Configuration directory couldn't be found!",
            ))
            .and_then(|dir| std::fs::read_to_string(dir.join("mpv/sponsorblock.toml")))
            .and_then(|data| toml::from_str(&data).map_err(|e| Error::new(ErrorKind::InvalidData, e)))
            .unwrap_or_else(|e| {
                log::warn!("{}", e);
                Self {
                    server_address: Self::default_server_address(),
                    categories: HashSet::default(),
                    action_types: HashSet::default(),
                    privacy_api: bool::default(),
                }
            })
    }
}
