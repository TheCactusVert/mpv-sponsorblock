pub mod action;
pub mod category;
pub mod segment;

use crate::config::Config;

use segment::{Segment, Segments};

use cached::proc_macro::cached;
use cached::SizedCache;

#[cached(
    type = "SizedCache<String, Segments>",
    create = "{ SizedCache::with_size(10) }",
    convert = r#"{ id.clone() }"#,
    option = true
)]
pub fn get_segments(config: &Config, id: String) -> Option<Segments> {
    if config.privacy_api {
        log::debug!("Getting segments for video {} with extra privacy...", id);
        Segment::get_with_privacy(config, id)
    } else {
        log::debug!("Getting segments for video {}...", id);
        Segment::get(config, id)
    }
    .map_err(|e| {
        log::error!("Failed to get segments: {}.", e);
        e
    })
    .ok()
}
