use crate::config::Config;
use crate::sponsorblock::segment::{get_segments, Segment, Segments};
use crate::utils::get_youtube_id;

#[derive(Default)]
pub struct Actions {
    segments: Option<Segments>,
}

impl Actions {
    pub fn load_segments(&mut self, path: &str, config: &Config) {
        self.segments = get_youtube_id(path).and_then(|id| get_segments(config, id));
    }

    pub fn get_segment(&self, time: f64) -> Option<&Segment> {
        self.segments
            .iter()
            .flatten()
            .filter(|s| s.is_action_skip() && s.is_in_segment(time))
            .next()
    }
}
