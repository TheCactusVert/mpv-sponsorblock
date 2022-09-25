use curl::easy::Easy;
use serde_derive::{Serialize, Deserialize};

use sha2::{Sha256, Digest};

static SPONSORBLOCK_URL: &str = "https://sponsor.ajay.app";

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
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

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Video {
    #[serde(rename = "videoID")]
    pub video_id: String,
    pub hash: String,
    pub segments: Segments,
}

pub type Videos = Vec<Video>;

pub fn from_api(id: String) -> Option<Segments> {
    let mut buf = Vec::new();
    let mut handle = Easy::new();
    handle.url(&format!("{}/api/skipSegments?videoID={}&category=sponsor&category=selfpromo", SPONSORBLOCK_URL, id)).ok()?;
    {
        let mut transfer = handle.transfer();
        transfer.write_function(|data| {
            buf.extend_from_slice(data);
            Ok(data.len())
        }).ok()?;
        transfer.perform().ok()?;
    }
    
    // Parse the string of data into Segments.
    serde_json::from_slice(&buf).ok()
}

pub fn from_private_api(id: String) -> Option<Segments> {
    let mut hasher = Sha256::new(); // create a Sha256 object
    hasher.update(&id); // write input message
    let hash = hasher.finalize(); // read hash digest and consume hasher

    let mut buf = Vec::new();
    let mut handle = Easy::new();
    handle.url(&format!("{}/api/skipSegments/{:.4}?category=sponsor&category=selfpromo", SPONSORBLOCK_URL, hex::encode(hash))).ok()?;
    {
        let mut transfer = handle.transfer();
        transfer.write_function(|data| {
            buf.extend_from_slice(data);
            Ok(data.len())
        }).ok()?;
        transfer.perform().ok()?;
    }
    
    // Parse the string of data into Segments.
    let videos: Videos = serde_json::from_slice(&buf).ok()?;
    Some(videos.into_iter().find(|v| v.video_id == id)?.segments)
}
