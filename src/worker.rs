use crate::config::Config;
use crate::sponsorblock;
use crate::sponsorblock::action::Action;
use crate::sponsorblock::category::Category;
use crate::sponsorblock::segment::{Segment, Segments};
use crate::utils::get_youtube_id;

use std::sync::{Arc, Mutex};

use tokio::runtime::Runtime;
use tokio::select;
use tokio::task::{self, JoinHandle};
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
    segments: Arc<Mutex<SortedSegments>>,
    rt: Runtime,
    token: CancellationToken,
    join: Option<JoinHandle<()>>,
}

impl Default for Worker {
    fn default() -> Self {
        Self {
            config: Config::default(),
            segments: Arc::new(Mutex::new(SortedSegments::default())),
            rt: Runtime::new().unwrap(),
            token: CancellationToken::new(),
            join: None,
        }
    }
}

impl Worker {
    pub fn start(&mut self, path: String) {
        let config = self.config.clone();
        let inner_self = self.segments.clone();
        let token = self.token.clone();

        self.join = get_youtube_id(path).and_then(|id| Some(self.rt.spawn(Self::run(id, config, inner_self, token))));
    }

    async fn run(id: String, config: Config, sorted_segments: Arc<Mutex<SortedSegments>>, token: CancellationToken) {
        let fut_segments = sponsorblock::fetch_segments(config, id);

        let mut segments = select! {
            _ = token.cancelled() => return,
            s = fut_segments => s.unwrap_or_default(),
        };

        // Lock only when segments are found
        let mut s = sorted_segments.lock().unwrap();

        // The sgments will be searched multiple times by seconds.
        // It's more efficient to split them before.

        (*s).skippable = segments.drain_filter(|s| s.action == Action::Skip).collect();
        log::info!("Found {} skippable segment(s)", (*s).skippable.len());

        (*s).mutable = segments.drain_filter(|s| s.action == Action::Mute).collect();
        log::info!("Found {} muttable segment(s)", (*s).mutable.len());

        (*s).poi = segments.drain_filter(|s| s.action == Action::Poi).next();
        log::info!("Highlight {}", if (*s).poi.is_some() { "found" } else { "not found" });

        (*s).full = segments.drain_filter(|s| s.action == Action::Full).next();
        log::info!("Category {}", if (*s).full.is_some() { "found" } else { "not found" });
    }

    pub fn cancel(&mut self) {
        self.token.cancel();
    }

    pub fn join(&mut self) {
        if let Some(join) = self.join.take() {
            self.rt.block_on(join);
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
