use crate::config::Config;

use std::sync::{Arc, Mutex};

use mpv_client::Handle;
use reqwest::StatusCode;
use sponsorblock_client::*;
use tokio::runtime::Runtime;
use tokio::select;
use tokio::task::JoinHandle;
use tokio_util::either::Either;
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
    pub fn new(client: Handle, client_parent: String, config: Config, id: String) -> Self {
        log::trace!("Starting worker");

        let sorted_segments = SharedSortedSegments::default();
        let rt = Runtime::new().unwrap();
        let token = CancellationToken::new();
        let join = rt.spawn(Self::run(
            client,
            client_parent,
            config,
            id,
            sorted_segments.clone(),
            token.clone(),
        ));

        SponsorBlockWorker {
            sorted_segments,
            rt,
            token,
            join,
        }
    }

    async fn run(
        client: Handle,
        client_parent: String,
        config: Config,
        id: String,
        sorted_segments: SharedSortedSegments,
        token: CancellationToken,
    ) {
        let fetch = if config.privacy_api {
            Either::Left(fetch_with_privacy(
                config.server_address,
                id,
                config.categories,
                config.action_types,
            ))
        } else {
            Either::Right(fetch(config.server_address, id, config.categories, config.action_types))
        };

        let segments = select! {
            s = fetch => s,
            _ = token.cancelled() => return,
        };

        // Lock only when data is received
        let mut sorted_segments = sorted_segments.lock().unwrap();
        (*sorted_segments) = match segments {
            Ok(s) => {
                let sorted = SortedSegments::from(s);
                log::info!("Found {} skippable segment(s)", sorted.skippable.len());
                log::info!("Found {} muttable segment(s)", sorted.mutable.len());
                log::info!("Highlight {}", if sorted.poi.is_some() { "found" } else { "not found" });
                log::info!("Category {}", if sorted.full.is_some() { "found" } else { "not found" });
                client
                    .command(["script-message-to", client_parent.as_str(), "segments-fetched"])
                    .unwrap();
                Some(sorted)
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
        log::trace!("Stopping worker");
        self.token.cancel();
        self.rt.block_on(&mut self.join).unwrap();
    }
}
