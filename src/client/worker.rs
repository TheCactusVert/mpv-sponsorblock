use crate::config::Config;

use std::sync::{Arc, Mutex};

use mpv_client::Client;
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

pub struct Worker {
    sorted_segments: SharedSortedSegments,
    rt: Runtime,
    thread: Option<(CancellationToken, JoinHandle<()>)>,
}

impl Worker {
    pub fn new() -> Self {
        Self {
            sorted_segments: SharedSortedSegments::default(),
            rt: Runtime::new().unwrap(),
            thread: None,
        }
    }

    pub fn start(&mut self, client: Client, client_parent: String, config: Config, id: String) {
        let token = CancellationToken::new();
        let join = self.rt.spawn(Self::run(
            client,
            client_parent,
            config,
            id,
            self.sorted_segments.clone(),
            token.clone(),
        ));

        self.thread = Some((token, join));
    }

    pub fn stop(&mut self) {
        if let Some(mut thread) = self.thread.take() {
            thread.0.cancel();
            self.rt.block_on(&mut thread.1).unwrap();
        }

        *self.sorted_segments.lock().unwrap() = None;
    }

    async fn run(
        client: Client,
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
        *sorted_segments.lock().unwrap() = match segments {
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
