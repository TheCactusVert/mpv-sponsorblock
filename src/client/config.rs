use std::collections::HashSet;
use std::io::{Error, ErrorKind};

use regex::Regex;
use serde::{Deserialize, Deserializer};
use serde_with::{serde_as, DeserializeAs};
use sponsorblock_client::{Action, Category};
use url::Url;

const SB_ADDRESS: &str = "https://sponsor.ajay.app";
const YT_PATTERNS: &[&str] = &[r"(?:www\.|m\.|)youtube\.com", r"(?:www\.|)youtu\.be"];

fn build_domains_regex(patterns: &[&str]) -> Result<Regex, regex::Error> {
    assert!(!patterns.is_empty());

    let pattern = format!(
        r"https?://(?:{}).*(?:/|%3D|v=|vi=)([0-9A-z-_]{{11}})(?:[%#?&]|$)",
        patterns.join("|")
    );

    Regex::new(&pattern)
}

fn default_as_true() -> bool {
    true
}

fn default_server() -> Url {
    Url::parse(SB_ADDRESS).unwrap()
}

fn default_domains_regex() -> Regex {
    build_domains_regex(YT_PATTERNS).unwrap()
}

fn from_domains<'de, D>(deserializer: D) -> Result<Regex, D::Error>
where
    D: Deserializer<'de>,
{
    let domains: HashSet<String> = Deserialize::deserialize(deserializer)?;
    let domains_escaped: Vec<String> = domains.into_iter().map(|d| regex::escape(&d)).collect();
    let domains_patterns: Vec<&str> = domains_escaped.iter().map(String::as_str).collect();

    let patterns = [YT_PATTERNS, domains_patterns.as_slice()].concat();

    build_domains_regex(&patterns).map_err(serde::de::Error::custom)
}

#[derive(serde_derive::Deserialize)]
struct ConfigAction {
    #[serde(skip, default = "default_as_true")]
    skip: bool,
    #[serde(default)]
    mute: bool,
    #[serde(skip, default = "default_as_true")]
    full: bool,
    #[serde(skip, default = "default_as_true")]
    poi: bool,
}

impl<'de> DeserializeAs<'de, HashSet<Action>> for ConfigAction {
    fn deserialize_as<D>(deserializer: D) -> Result<HashSet<Action>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let config_action: ConfigAction = Self::deserialize(deserializer)?;

        let mut hashset: HashSet<Action> = HashSet::default();

        if config_action.skip {
            hashset.insert(Action::Skip);
        };
        if config_action.mute {
            hashset.insert(Action::Mute);
        };
        if config_action.full {
            hashset.insert(Action::Full);
        };
        if config_action.poi {
            hashset.insert(Action::Poi);
        };

        Ok(hashset)
    }
}

#[serde_as]
#[derive(serde_derive::Deserialize, Clone, Debug)]
pub struct Config {
    #[serde(default = "default_server")]
    pub server_address: Url,
    #[serde(default)]
    pub categories: HashSet<Category>,
    #[serde(flatten)]
    #[serde_as(as = "ConfigAction")]
    pub action_types: HashSet<Action>,
    #[serde(default)]
    pub privacy_api: bool,
    #[serde(
        default = "default_domains_regex",
        deserialize_with = "from_domains",
        rename = "domains"
    )]
    pub youtube_regex: Regex,
    #[serde(default)]
    pub skip_notice: bool,
}

impl Config {
    pub fn get() -> Self {
        dirs::config_dir()
            .ok_or(Error::new(ErrorKind::NotFound, "configuration directory not found"))
            .and_then(|dir| std::fs::read_to_string(dir.join("mpv/sponsorblock.toml")))
            .and_then(|data| toml::from_str(&data).map_err(|e| Error::new(ErrorKind::InvalidData, e)))
            .unwrap_or_else(|e| {
                log::warn!("Failed to load configuration file: {}. Falling back to default", e);
                Self::default()
            })
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server_address: default_server(),
            categories: HashSet::default(),
            action_types: HashSet::default(),
            privacy_api: bool::default(),
            youtube_regex: default_domains_regex(),
            skip_notice: bool::default(),
        }
    }
}
