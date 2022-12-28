use super::Action;
use super::Category;

use reqwest::{Result, Url};
use serde_derive::Deserialize;
use sha2::{Digest, Sha256};

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Segment {
    pub category: Category,
    #[serde(rename = "actionType")]
    pub action: Action,
    pub segment: [f64; 2],
    #[serde(rename = "UUID")]
    pub uuid: String,
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
    //#[serde(rename = "videoID")]
    //pub video_id: String,
    #[serde(with = "hex")]
    pub hash: [u8; 32],
    pub segments: Segments,
}

type Videos = Vec<Video>;

impl Segment {
    pub(super) async fn fetch<C, A>(server_address: Url, id: String, categories: C, action_types: A) -> Result<Segments>
    where
        C: IntoIterator<Item = Category>,
        A: IntoIterator<Item = Action>,
    {
        let mut url = server_address.join("/api/skipSegments").unwrap();

        url.query_pairs_mut()
            .append_pair("videoID", &id)
            .extend_pairs(categories.into_iter().map(|v| ("category", v.to_string())))
            .extend_pairs(action_types.into_iter().map(|v| ("actionType", v.to_string())));

        Ok(reqwest::get(url).await?.error_for_status()?.json::<Segments>().await?)
    }

    pub(super) async fn fetch_with_privacy<C, A>(
        server_address: Url,
        id: String,
        categories: C,
        action_types: A,
    ) -> Result<Segments>
    where
        C: IntoIterator<Item = Category>,
        A: IntoIterator<Item = Action>,
    {
        let mut hasher = Sha256::new(); // create a Sha256 object
        hasher.update(id); // write input message
        let hash = hasher.finalize(); // read hash digest and consume hasher

        let mut url = server_address
            .join("/api/skipSegments/")
            .unwrap()
            .join(&hex::encode(hash)[0..4])
            .unwrap();

        url.query_pairs_mut()
            .extend_pairs(categories.into_iter().map(|v| ("category", v.to_string())))
            .extend_pairs(action_types.into_iter().map(|v| ("actionType", v.to_string())));

        Ok(reqwest::get(url)
            .await?
            .error_for_status()?
            .json::<Videos>()
            .await?
            .into_iter()
            .find(|v| v.hash == hash.as_slice())
            .map_or(Segments::default(), |v| v.segments))
    }

    pub fn is_in_segment(&self, time: f64) -> bool {
        time >= self.segment[0] && time < (self.segment[1] - 0.1_f64)
    }
}

impl std::fmt::Display for Segment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {} - {}", self.category, self.segment[0], self.segment[1])
    }
}
