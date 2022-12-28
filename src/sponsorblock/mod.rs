mod action;
mod category;
mod segment;

use crate::config::Config;

pub use action::Action;
pub use category::Category;
pub use segment::{Segment, Segments};

use cached::proc_macro::cached;
use cached::SizedCache;
use reqwest::StatusCode;

#[cached(
    type = "SizedCache<String, Segments>",
    create = "{ SizedCache::with_size(10) }",
    convert = r#"{ id.clone() }"#,
    option = true
)]
pub async fn fetch(config: Config, id: String) -> Option<Segments> {
    let segments = if config.privacy_api {
        Segment::fetch_with_privacy(config.server_address, id, config.categories, config.action_types).await
    } else {
        Segment::fetch(config.server_address, id, config.categories, config.action_types).await
    };

    match segments {
        Ok(v) => Some(v),
        Err(e) if e.status() == Some(StatusCode::NOT_FOUND) => None,
        Err(e) => {
            log::error!("Failed to get segments: {}", e);
            None
        }
    }
}
