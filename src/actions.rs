use crate::config::Config;
use crate::mpv::EventProperty;
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

    pub fn drop_segments(&mut self) {
        self.segments = None;
    }

    pub fn skip_segments(&self, mpv_event: EventProperty) -> Option<&Segment> {
        let time_pos: f64 = mpv_event.get_data()?;
        match &self.segments {
            Some(segments) => segments
                .iter()
                .filter(|s| s.is_action_skip() && time_pos >= s.segment[0] && time_pos < s.segment[1])
                .next(),
            None => None,
        }
    }
}
