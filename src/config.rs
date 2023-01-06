use std::collections::HashSet;
use std::io::{Error, ErrorKind};

use serde::{Deserialize, Deserializer};
use sponsorblock_client::{Action, Category};
use url::Url;

fn default_server() -> Url {
    Url::parse("https://sponsor.ajay.app").unwrap()
}

fn from_unescaped<'de, D>(deserializer: D) -> Result<HashSet<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let domains: HashSet<&str> = Deserialize::deserialize(deserializer)?;
    Ok(domains.into_iter().map(|domain| regex::escape(domain)).collect())
}

#[derive(Debug, serde_derive::Deserialize, Clone)]
pub struct Config {
    #[serde(default = "default_server")]
    pub server_address: Url,
    #[serde(default)]
    pub categories: HashSet<Category>,
    #[serde(default)]
    pub action_types: HashSet<Action>,
    #[serde(default)]
    pub privacy_api: bool,
    #[serde(default, deserialize_with = "from_unescaped", rename = "domains")]
    pub domains_escaped: HashSet<String>,
    #[serde(default)]
    pub skip_notice: bool,
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
