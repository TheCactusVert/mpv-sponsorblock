use std::collections::HashSet;

use regex::Regex;
use serde_derive::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default = "Config::default_server_address")]
    pub server_address: String,
    #[serde(
        default = "HashSet::default",
        deserialize_with = "Config::deserialize_categories"
    )]
    categories: HashSet<String>,
    #[serde(
        default = "HashSet::default",
        deserialize_with = "Config::deserialize_action_types"
    )]
    action_types: HashSet<String>,
    #[serde(default = "bool::default")]
    pub privacy_api: bool,
}

impl Config {
    fn deserialize_categories<'de, D>(deserializer: D) -> Result<HashSet<String>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let regex = Regex::new(r"^sponsor|selfpromo|interaction|poi_highlight|intro|outro|preview|music_offtopic|filler|exclusive_access+$").unwrap();
        let categories: HashSet<String> = serde::Deserialize::deserialize(deserializer)?;
        Ok(categories
            .into_iter()
            .filter(|v| regex.is_match(v))
            .collect())
    }

    fn deserialize_action_types<'de, D>(deserializer: D) -> Result<HashSet<String>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let regex = Regex::new(r"^skip|mute|poi|full+$").unwrap();
        let categories: HashSet<String> = serde::Deserialize::deserialize(deserializer)?;
        Ok(categories
            .into_iter()
            .filter(|v| regex.is_match(v))
            .collect())
    }

    fn default_server_address() -> String {
        "https://sponsor.ajay.app".to_string()
    }

    fn from_file() -> Option<Self> {
        let config_file = dirs::config_dir()?.join("mpv/sponsorblock.toml");
        Some(toml::from_str(&std::fs::read_to_string(config_file).ok()?).ok()?)
    }

    pub fn get() -> Self {
        Self::from_file().unwrap_or_default()
    }

    pub fn parameters(&self) -> String {
        let categories = self.categories.iter().map(|v| format!("category={}", v));

        let action_types = self
            .action_types
            .iter()
            .map(|v| format!("actionType={}", v));

        categories
            .chain(action_types)
            .collect::<Vec<String>>()
            .join("&")
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server_address: Self::default_server_address(),
            categories: HashSet::default(),
            action_types: HashSet::default(),
            privacy_api: bool::default(),
        }
    }
}
