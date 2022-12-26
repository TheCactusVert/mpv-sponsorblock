use crate::config::Config;
use crate::sponsorblock;
use crate::sponsorblock::action::Action;
use crate::sponsorblock::category::Category;
use crate::sponsorblock::segment::{Segment, Segments};
use crate::utils::get_youtube_id;

use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

#[derive(Default)]
struct SortedSegments {
    skippable: Segments,
    mutable: Segments,
    poi: Option<Segment>,
    full: Option<Segment>,
}

#[derive(Default)]
pub struct Actions {
    config: Config,
    segments: Arc<Mutex<SortedSegments>>,
    handle: Option<JoinHandle<()>>,
}

impl Actions {
    pub fn start(&mut self, path: String) {
        let inner_self = self.segments.clone();
        let config = self.config.clone();

        let id = match get_youtube_id(path) {
            Some(v) => v,
            None => return,
        };

        self.handle = Some(thread::spawn(move || {
            let mut segments = sponsorblock::fetch_segments(&config, id).unwrap_or_default();

            // Lock only when segments are found
            let mut s = inner_self.lock().unwrap();

            // The sgments will be searched multiple times by seconds.
            // It's more efficient to split them before.

            (*s).skippable = segments.drain_filter(|s| s.action == Action::Skip).collect();
            log::debug!("Found {} skippable segment(s).", (*s).skippable.len());

            (*s).mutable = segments.drain_filter(|s| s.action == Action::Mute).collect();
            log::debug!("Found {} muttable segment(s).", (*s).mutable.len());

            (*s).poi = segments.drain_filter(|s| s.action == Action::Poi).next();
            log::debug!("Highlight {}.", if (*s).poi.is_some() { "found" } else { "not found" });

            (*s).full = segments.drain_filter(|s| s.action == Action::Full).next();
            log::debug!("Category {}.", if (*s).full.is_some() { "found" } else { "not found" });
        }));
    }

    pub fn join(&mut self) {
        if let Some(handle) = self.handle.take() {
            handle.join().unwrap();
        }
    }

    pub fn get_skip_segment(&self, time_pos: f64) -> Option<Segment> {
        self.segments
            .lock()
            .unwrap()
            .skippable
            .iter()
            .find(|s| s.is_in_segment(time_pos))
            .cloned()
    }

    pub fn get_mute_segment(&self, time_pos: f64) -> Option<Segment> {
        self.segments
            .lock()
            .unwrap()
            .mutable
            .iter()
            .find(|s| s.is_in_segment(time_pos))
            .cloned()
    }

    pub fn get_video_poi(&self) -> Option<f64> {
        self.segments.lock().unwrap().poi.as_ref().map(|s| s.segment[0])
    }

    pub fn get_video_category(&self) -> Option<Category> {
        self.segments.lock().unwrap().full.as_ref().map(|s| s.category)
    }
}
