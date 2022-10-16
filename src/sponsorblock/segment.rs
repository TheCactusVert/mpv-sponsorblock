use crate::config::Config;
use crate::utils::get_data;

use serde_derive::Deserialize;
use sha2::{Digest, Sha256};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Segment {
    pub category: String,
    #[serde(rename = "actionType")]
    pub action: String,
    pub segment: [f64; 2],
    #[serde(rename = "UUID")]
    pub uuid: String,
    pub locked: i64,
    pub votes: i64,
    pub video_duration: f64,
    #[serde(rename = "userID")]
    pub user_id: String,
    pub description: String,
}

pub type Segments = Vec<Segment>;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Video {
    #[serde(rename = "videoID")]
    pub video_id: String,
    pub segments: Segments,
}

type Videos = Vec<Video>;

impl Segment {
    pub fn get_segments(config: &Config, id: String) -> Option<Segments> {
        log::info!("Getting segments for video {}.", id);

        let buf = get_data(&format!(
            "{}/api/skipSegments?videoID={}&{}",
            config.server_address,
            id,
            config.parameters(),
        ))
        .map_err(|e| {
            log::error!("Failed to get SponsorBlock data: {}.", e.to_string());
            e
        })
        .ok()?;

        // Parse the string of data into Segments.
        serde_json::from_slice(&buf).ok()
    }

    pub fn get_segments_with_privacy(config: &Config, id: String) -> Option<Segments> {
        log::info!("Getting segments for video {} with extra privacy.", id);

        let mut hasher = Sha256::new(); // create a Sha256 object
        hasher.update(&id); // write input message
        let hash = hasher.finalize(); // read hash digest and consume hasher

        let buf = get_data(&format!(
            "{}/api/skipSegments/{:.4}?{}",
            config.server_address,
            hex::encode(hash),
            config.parameters(),
        ))
        .map_err(|e| {
            log::error!("Failed to get SponsorBlock data: {}.", e.to_string());
            e
        })
        .ok()?;

        // Parse the string of data into Videos.
        let videos: Videos = serde_json::from_slice(&buf).ok()?;
        Some(videos.into_iter().find(|v| v.video_id == id)?.segments)
    }
}
