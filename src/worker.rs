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
    fn new(mut segments: Segments) -> Self {
        let skippable: Segments = segments.drain_filter(|s| s.action == Action::Skip).collect();
        let mutable: Segments = segments.drain_filter(|s| s.action == Action::Mute).collect();
        let poi: Option<Segment> = segments.drain_filter(|s| s.action == Action::Poi).next();
        let full: Option<Segment> = segments.drain_filter(|s| s.action == Action::Full).next();

        Self {
            skippable,
            mutable,
            poi,
            full,
        }
    }
}

pub struct Worker {
    sorted_segments: Arc<Mutex<Option<SortedSegments>>>,
    rt: Runtime,
    token: CancellationToken,
    join: JoinHandle<()>,
}

impl Worker {
    pub fn new(config: Config, path: String) -> Option<Self> {
        let id = get_youtube_id(path)?; // If not a YT video then do nothing

        let sorted_segments = Arc::new(Mutex::new(None));
        let rt = Runtime::new().unwrap();
        let token = CancellationToken::new();
        let join = rt.spawn(Self::run(config, id, sorted_segments.clone(), token.clone()));

        Some(Worker {
            sorted_segments,
            rt,
            token,
            join,
        })
    }

    async fn fetch(config: Config, id: String) -> Option<Segments> {
        match if config.privacy_api {
            sponsorblock::fetch_with_privacy(config.server_address, id, config.categories, config.action_types).await
        } else {
            sponsorblock::fetch(config.server_address, id, config.categories, config.action_types).await
        } {
            Ok(v) => Some(v),
            Err(e) if e.status() == Some(StatusCode::NOT_FOUND) => None,
            Err(e) => {
                log::error!("Failed to get segments: {}", e);
                None
            }
        }
    }

    async fn run(
        config: Config,
        id: String,
        sorted_segments: Arc<Mutex<Option<SortedSegments>>>,
        token: CancellationToken,
    ) {
        select! {
            _ = token.cancelled() => {
                log::debug!("Thread cancelled. Segments couldn't be retrieved in time");
            },
            segments = Self::fetch(config, id) => {
                let mut sorted_segments = sorted_segments.lock().unwrap();
                (*sorted_segments) = Some({
                    let sorted_segments = SortedSegments::new(segments.unwrap_or_default());
                    log::info!("Found {} skippable segment(s)", sorted_segments.skippable.len());
                    log::info!("Found {} muttable segment(s)", sorted_segments.mutable.len());
                    log::info!("Highlight {}", if sorted_segments.poi.is_some() { "found" } else { "not found" });
                    log::info!("Category {}", if sorted_segments.full.is_some() { "found" } else { "not found" });
                    sorted_segments
                });
            }
        };
    }

    pub fn get_skip_segment(&self, time_pos: f64) -> Option<Segment> {
        self.sorted_segments.lock().unwrap().as_ref().and_then(|s| {
            s.skippable
                .iter()
                .find(|s| time_pos >= s.segment[0] && time_pos < (s.segment[1] - 0.1_f64))
                .cloned()
        })
    }

    pub fn get_mute_segment(&self, time_pos: f64) -> Option<Segment> {
        self.sorted_segments.lock().unwrap().as_ref().and_then(|s| {
            s.mutable
                .iter()
                .find(|s| time_pos >= s.segment[0] && time_pos < (s.segment[1] - 0.1_f64))
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

impl Drop for Worker {
    fn drop(&mut self) {
        log::debug!("Stopping worker");
        self.token.cancel();
        self.rt.block_on(&mut self.join).unwrap();
    }
}
