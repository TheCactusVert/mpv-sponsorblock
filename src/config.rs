use serde_derive::Deserialize;

fn default_server_address() -> String {
    "https://sponsor.ajay.app".to_string()
}

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default="default_server_address")]
    pub server_address: String,
    #[serde(default="bool::default")]
    pub privacy_api: bool,
}

impl Config {
    fn from_file() -> Option<Self> {
        let config_file = dirs::config_dir()?.join("mpv-sponsorblock.toml");
        Some(toml::from_str(&std::fs::read_to_string(config_file).ok()?).ok()?)
    }
    
    pub fn get() -> Self {
        Self::from_file().unwrap_or_default()
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server_address: default_server_address(),
            privacy_api: false,
        }
    }
}
