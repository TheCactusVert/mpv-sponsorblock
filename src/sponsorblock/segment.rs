use crate::config::Config;
use crate::utils::fetch_data;

use super::action::Action;
use super::category::Category;

use anyhow::Result;
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

static NOT_FOUND: &'static [u8] = b"Not Found";

impl Segment {
    pub(super) fn fetch(config: &Config, id: String) -> Result<Option<Segments>> {
        let buf = fetch_data(
            &format!(
                "{}/api/skipSegments?videoID={}&{}",
                config.server_address,
                id,
                config.parameters(),
            ),
            config.timeout,
        )?;

        if buf == NOT_FOUND {
            Ok(None)
        } else {
            // Parse the string of data into Segments.
            let segments: Segments = serde_json::from_slice(&buf)?;
            Ok(Some(segments))
        }
    }

    pub(super) fn fetch_with_privacy(config: &Config, id: String) -> Result<Option<Segments>> {
        let mut hasher = Sha256::new(); // create a Sha256 object
        hasher.update(&id); // write input message
        let hash = hasher.finalize(); // read hash digest and consume hasher

        let buf = fetch_data(
            &format!(
                "{}/api/skipSegments/{:.4}?{}",
                config.server_address,
                hex::encode(hash),
                config.parameters(),
            ),
            config.timeout,
        )?;

        if buf == NOT_FOUND {
            Ok(None)
        } else {
            // Parse the string of data into Videos.
            let videos: Videos = serde_json::from_slice(&buf)?;
            Ok(videos
                .into_iter()
                .find(|v| v.video_id == id)
                .and_then(|v| Some(v.segments)))
        }
    }

    pub fn is_in_segment(&self, time: f64) -> bool {
        time >= self.segment[0] && time < self.segment[1]
    }
}

impl std::fmt::Display for Segment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "segment [{}] {} - {}",
            self.category, self.segment[0], self.segment[1]
        )
    }
}
