use crate::config::Config;
use crate::sponsorblock;
use crate::sponsorblock::action::Action;
use crate::sponsorblock::category::Category;
use crate::sponsorblock::segment::{Segment, Segments};
use crate::utils::get_youtube_id;

use std::sync::{Arc, Mutex};

use tokio::runtime::Runtime;
use tokio::select;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

#[derive(Default)]
struct SortedSegments {
    skippable: Segments,
    mutable: Segments,
    poi: Option<Segment>,
    full: Option<Segment>,
}

pub struct Worker {
    config: Config,
    sorted_segments: Arc<Mutex<SortedSegments>>,
    rt: Runtime,
    token: CancellationToken,
    join: Option<JoinHandle<()>>,
}

impl Default for Worker {
    fn default() -> Self {
        Self {
            config: Config::default(),
            sorted_segments: Arc::new(Mutex::new(SortedSegments::default())),
            rt: Runtime::new().unwrap(),
            token: CancellationToken::new(),
            join: None,
        }
    }
}

impl Worker {
    pub fn start(mut self, path: String) -> Self {
        assert!(self.join.is_none());

        let config = self.config.clone();
        let sorted_segments = self.sorted_segments.clone();
        let token = self.token.clone();

        self.join =
            get_youtube_id(path).and_then(|id| Some(self.rt.spawn(Self::run(id, config, sorted_segments, token))));
        self
    }

    async fn run(id: String, config: Config, sorted_segments: Arc<Mutex<SortedSegments>>, token: CancellationToken) {
        select! {
            _ = token.cancelled() => {
                log::warn!("Thread cancelled. Segments won't be retrieved");
            },
            segments = sponsorblock::fetch_segments(config, id) => {
                let mut segments = segments.unwrap_or_default();

                // Lock only when segments are found
                let mut sorted_segments = sorted_segments.lock().unwrap();

                // The sgments will be searched multiple times by seconds.
                // It's more efficient to split them before.

                (*sorted_segments).skippable = segments.drain_filter(|s| s.action == Action::Skip).collect();
                log::info!("Found {} skippable segment(s)", (*sorted_segments).skippable.len());

                (*sorted_segments).mutable = segments.drain_filter(|s| s.action == Action::Mute).collect();
                log::info!("Found {} muttable segment(s)", (*sorted_segments).mutable.len());

                (*sorted_segments).poi = segments.drain_filter(|s| s.action == Action::Poi).next();
                log::info!("Highlight {}", if (*sorted_segments).poi.is_some() { "found" } else { "not found" });

                (*sorted_segments).full = segments.drain_filter(|s| s.action == Action::Full).next();
                log::info!("Category {}", if (*sorted_segments).full.is_some() { "found" } else { "not found" });
            }
        };
    }

    pub fn get_skip_segment(&self, time_pos: f64) -> Option<Segment> {
        self.sorted_segments
            .lock()
            .unwrap()
            .skippable
            .iter()
            .find(|s| s.is_in_segment(time_pos))
            .cloned()
    }

    pub fn get_mute_segment(&self, time_pos: f64) -> Option<Segment> {
        self.sorted_segments
            .lock()
            .unwrap()
            .mutable
            .iter()
            .find(|s| s.is_in_segment(time_pos))
            .cloned()
    }

    pub fn get_video_poi(&self) -> Option<f64> {
        self.sorted_segments.lock().unwrap().poi.as_ref().map(|s| s.segment[0])
    }

    pub fn get_video_category(&self) -> Option<Category> {
        self.sorted_segments.lock().unwrap().full.as_ref().map(|s| s.category)
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        log::debug!("Dropping segments");
        self.token.cancel();
        if let Some(join) = self.join.take() {
            self.rt.block_on(join).unwrap();
        }
    }
}
