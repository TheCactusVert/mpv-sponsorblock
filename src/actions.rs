use crate::config::Config;
use crate::sponsorblock::action::Action;
use crate::sponsorblock::category::Category;
use crate::sponsorblock::segment::{get_segments, Segment, Segments};
use crate::utils::get_youtube_id;

#[derive(Debug, Default)]
pub struct Actions {
    skippable: Segments,
    mutable: Segments,
    poi: Option<Segment>,
    full: Option<Segment>,
}

impl Actions {
    pub fn load_chapters(&mut self, path: &str, config: &Config) {
        let mut segments = get_youtube_id(path)
            .and_then(|id| get_segments(config, id))
            .unwrap_or_default();

        self.skippable = segments.drain_filter(|s| s.action == Action::Skip).collect();
        self.mutable = segments.drain_filter(|s| s.action == Action::Mute).collect();
        self.poi = segments.drain_filter(|s| s.action == Action::Poi).next();
        self.full = segments.drain_filter(|s| s.action == Action::Full).next();
    }

    pub fn get_skip_segment(&self, time_pos: f64) -> Option<&Segment> {
        self.skippable.iter().find(|s| s.is_in_segment(time_pos))
    }

    pub fn get_mute_segment(&self, time_pos: f64) -> Option<&Segment> {
        self.mutable.iter().find(|s| s.is_in_segment(time_pos))
    }

    pub fn get_video_poi(&self) -> Option<f64> {
        self.poi.as_ref().map(|s| s.segment[0])
    }

    pub fn get_video_category(&self) -> Option<Category> {
        self.full.as_ref().map(|s| s.category)
    }
}
