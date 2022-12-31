use crate::config::Config;
use crate::utils::get_youtube_id;

use std::sync::{Arc, Mutex};

use reqwest::StatusCode;
use sponsorblock::*;
use sponsorblock_client as sponsorblock;
use tokio::runtime::Runtime;
use tokio::select;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

struct SortedSegments {
    skippable: Segments,
    mutable: Segments,
    poi: Option<Segment>,
    full: Option<Segment>,
}

impl SortedSegments {
    fn from(mut segments: Segments) -> Self {
        let skippable: Segments = segments.drain_filter(|s| s.action == Action::Skip).collect();
        let mutable: Segments = segments.drain_filter(|s| s.action == Action::Mute).collect();
        let poi: Option<Segment> = segments.drain_filter(|s| s.action == Action::Poi).next();
        let full: Option<Segment> = segments.drain_filter(|s| s.action == Action::Full).next();

        log::info!("Found {} skippable segment(s)", skippable.len());
        log::info!("Found {} muttable segment(s)", mutable.len());
        log::info!("Highlight {}", if poi.is_some() { "found" } else { "not found" });
        log::info!("Category {}", if full.is_some() { "found" } else { "not found" });

        Self {
            skippable,
            mutable,
            poi,
            full,
        }
    }
}

type SharedSortedSegments = Arc<Mutex<Option<SortedSegments>>>;

pub struct SponsorBlockWorker {
    sorted_segments: SharedSortedSegments,
    rt: Runtime,
    token: CancellationToken,
    join: JoinHandle<()>,
}

impl SponsorBlockWorker {
    pub fn new(config: Config, path: String) -> Option<Self> {
        let id = get_youtube_id(path)?; // If not a YT video then do nothing

        log::trace!("Starting worker");

        let sorted_segments = SharedSortedSegments::default();
        let rt = Runtime::new().unwrap();
        let token = CancellationToken::new();
        let join = rt.spawn(Self::run(config, id, sorted_segments.clone(), token.clone()));

        Some(SponsorBlockWorker {
            sorted_segments,
            rt,
            token,
            join,
        })
    }

    async fn run(config: Config, id: String, sorted_segments: SharedSortedSegments, token: CancellationToken) {
        let segments = select! {
            s = Self::fetch(config, id) => s,
            _ = token.cancelled() => return,
        };

        // Lock only when data is received
        let mut sorted_segments = sorted_segments.lock().unwrap();
        (*sorted_segments) = segments.and_then(|s| Some(SortedSegments::from(s)));
    }

    async fn fetch(config: Config, id: String) -> Option<Segments> {
        let segments = if config.privacy_api {
            sponsorblock::fetch_with_privacy(config.server_address, id, config.categories, config.action_types).await
        } else {
            sponsorblock::fetch(config.server_address, id, config.categories, config.action_types).await
        };

        match segments {
            Ok(v) => {
                log::trace!("Segments found");
                Some(v)
            }
            Err(e) if e.status() == Some(StatusCode::NOT_FOUND) => {
                log::info!("No segments found");
                None
            }
            Err(e) => {
                log::error!("Failed to get segments: {}", e);
                None
            }
        }
    }

    pub fn get_skip_segment(&self, time_pos: f64) -> Option<Segment> {
        self.sorted_segments.lock().unwrap().as_ref().and_then(|s| {
            s.skippable
                .iter()
                .find(|s| time_pos >= s.segment[0] && time_pos < (s.segment[1] - 0.1_f64)) // Fix for a stupid bug when times are too precise
                .cloned()
        })
    }

    pub fn get_mute_segment(&self, time_pos: f64) -> Option<Segment> {
        self.sorted_segments.lock().unwrap().as_ref().and_then(|s| {
            s.mutable
                .iter()
                .find(|s| time_pos >= s.segment[0] && time_pos < s.segment[1])
                .cloned()
        })
    }

    pub fn get_video_poi(&self) -> Option<f64> {
        self.sorted_segments
            .lock()
            .unwrap()
            .as_ref()
            .and_then(|s| s.poi.as_ref().map(|s| s.segment[0]))
    }

    pub fn get_video_category(&self) -> Option<Category> {
        self.sorted_segments
            .lock()
            .unwrap()
            .as_ref()
            .and_then(|s| s.full.as_ref().map(|s| s.category))
    }
}

impl Drop for SponsorBlockWorker {
    fn drop(&mut self) {
        self.token.cancel();
        log::trace!("Stopping worker");
        self.rt.block_on(&mut self.join).unwrap();
    }
}
