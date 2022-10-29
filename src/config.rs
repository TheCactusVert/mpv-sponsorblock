use crate::sponsorblock::action::Action;
use crate::sponsorblock::category::Category;

use std::{collections::HashSet, time::Duration};

use anyhow::{anyhow, Result};
use serde_derive::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default = "Config::default_server_address")]
    pub server_address: String,
    #[serde(default = "Config::default_timeout")]
    pub timeout: Duration,
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

    fn default_timeout() -> Duration {
        Duration::from_secs(1)
    }

    fn from_file() -> Result<Self> {
        let config_file = dirs::config_dir()
            .ok_or(anyhow!("failed to find config directory"))?
            .join("mpv/sponsorblock.toml");
        Ok(toml::from_str(&std::fs::read_to_string(config_file)?)?)
    }

    pub fn get() -> Self {
        match Self::from_file() {
            Ok(c) => c,
            Err(e) => {
                log::error!("Failed to read config file: {}", e);
                Config::default()
            }
        }
    }

    pub fn parameters(&self) -> String {
        let categories = self.categories.iter().map(|v| format!("category={}", v));
        let action_types = self.action_types.iter().map(|v| format!("actionType={}", v));

        categories.chain(action_types).collect::<Vec<String>>().join("&")
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server_address: Self::default_server_address(),
            timeout: Self::default_timeout(),
            categories: HashSet::default(),
            action_types: HashSet::default(),
            privacy_api: bool::default(),
        }
    }
}
