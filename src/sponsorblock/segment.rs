use serde_derive::Deserialize;
use serde_derive::Serialize;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkipSegment {
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

pub type SkipSegments = Vec<SkipSegment>;

