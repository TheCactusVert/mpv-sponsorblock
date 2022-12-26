use crate::config::Config;
use crate::sponsorblock;
use crate::sponsorblock::action::Action;
use crate::sponsorblock::category::Category;
use crate::sponsorblock::segment::{Segment, Segments};
use crate::utils::get_youtube_id;

#[derive(Debug)]
pub struct Actions {
    config: Config,
    skippable: Segments,
    mutable: Segments,
    poi: Option<Segment>,
    full: Option<Segment>,
}

impl Actions {
    pub fn new() -> Self {
        Actions {
            config: Config::get(),
            skippable: Vec::new(),
            mutable: Vec::new(),
            poi: None,
            full: None,
        }
    }

    pub fn load_chapters<S: AsRef<str>>(&mut self, path: S) {
        let mut segments = get_youtube_id(path.as_ref())
            .and_then(|id| sponsorblock::fetch_segments(&self.config, id))
            .unwrap_or_default();

        // The sgments will be searched multiple times by seconds.
        // It's more efficient to split them before.

        self.skippable = segments.drain_filter(|s| s.action == Action::Skip).collect();
        log::debug!("Found {} skippable segment(s).", self.skippable.len());

        self.mutable = segments.drain_filter(|s| s.action == Action::Mute).collect();
        log::debug!("Found {} muttable segment(s).", self.mutable.len());

        self.poi = segments.drain_filter(|s| s.action == Action::Poi).next();
        log::debug!("Highlight {}.", if self.poi.is_some() { "found" } else { "not found" });

        self.full = segments.drain_filter(|s| s.action == Action::Full).next();
        log::debug!("Category {}.", if self.full.is_some() { "found" } else { "not found" });
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
