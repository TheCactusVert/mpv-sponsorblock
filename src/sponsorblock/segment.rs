use crate::config::Config;
use crate::utils::get_data;

use super::action::Action;
use super::category::Category;

use anyhow::{anyhow, Result};
use cached::proc_macro::cached;
use cached::SizedCache;
use serde_derive::Deserialize;
use sha2::{Digest, Sha256};

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Segment {
    pub category: Category,
    #[serde(rename = "actionType")]
    pub action: Action,
    pub segment: [f64; 2],
    //#[serde(rename = "UUID")]
    //pub uuid: String,
    //pub locked: i64,
    //pub votes: i64,
    //pub video_duration: f64,
    //#[serde(rename = "userID")]
    //pub user_id: String,
    //pub description: String,
}

pub type Segments = Vec<Segment>;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Video {
    #[serde(rename = "videoID")]
    pub video_id: String,
    //pub hash: String,
    pub segments: Segments,
}

type Videos = Vec<Video>;

impl Segment {
    fn get(config: &Config, id: String) -> Result<Segments> {
        log::info!("Getting segments for video {}...", id);

        let buf = get_data(
            &format!(
                "{}/api/skipSegments?videoID={}&{}",
                config.server_address,
                id,
                config.parameters(),
            ),
            config.timeout,
        )?;

        // Parse the string of data into Segments.
        let segments: Segments = serde_json::from_slice(&buf)?;
        Ok(segments)
    }

    fn get_with_privacy(config: &Config, id: String) -> Result<Segments> {
        log::info!("Getting segments for video {} with extra privacy...", id);

        let mut hasher = Sha256::new(); // create a Sha256 object
        hasher.update(&id); // write input message
        let hash = hasher.finalize(); // read hash digest and consume hasher

        let buf = get_data(
            &format!(
                "{}/api/skipSegments/{:.4}?{}",
                config.server_address,
                hex::encode(hash),
                config.parameters(),
            ),
            config.timeout,
        )?;

        // Parse the string of data into Videos.
        let videos: Videos = serde_json::from_slice(&buf)?;
        Ok(videos
            .into_iter()
            .find(|v| v.video_id == id)
            .ok_or(anyhow!("the SponsorBlock API returned invalid data."))?
            .segments)
    }

    pub fn is_in_segment(&self, time: f64) -> bool {
        time >= self.segment[0] && time < self.segment[1]
    }
}

#[cached(
    type = "SizedCache<String, Segments>",
    create = "{ SizedCache::with_size(10) }",
    convert = r#"{ id.clone() }"#,
    option = true
)]
pub fn get_segments(config: &Config, id: String) -> Option<Segments> {
    if config.privacy_api {
        Segment::get_with_privacy(config, id)
    } else {
        Segment::get(config, id)
    }
    .map_err(|e| {
        log::error!("Failed to get segments: {}.", e);
        e
    })
    .ok()
}
