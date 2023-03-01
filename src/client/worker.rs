use super::Config;

use std::sync::{Arc, Mutex};

use mpv_client::Client;
use reqwest::StatusCode;
use sponsorblock_client::*;
use tokio::runtime::Runtime;
use tokio::select;
use tokio::task::JoinHandle;
use tokio_util::either::Either;
use tokio_util::sync::CancellationToken;

type SharedSegments = Arc<Mutex<Option<Segments>>>;

pub struct Worker {
    segments: SharedSegments,
    rt: Runtime,
    thread: Option<(CancellationToken, JoinHandle<()>)>,
}

impl Default for Worker {
    fn default() -> Self {
        Self {
            segments: SharedSegments::default(),
            rt: Runtime::new().unwrap(),
            thread: None,
        }
    }
}

impl Worker {
    pub fn start(&mut self, client: Client, parent: String, config: Config, id: String) {
        assert!(self.thread.is_none());

        let token = CancellationToken::new();
        let join = self.rt.spawn(Self::run(
            client,
            parent,
            config,
            id,
            self.segments.clone(),
            token.clone(),
        ));

        self.thread = Some((token, join));
    }

    pub fn stop(&mut self) {
        if let Some(mut thread) = self.thread.take() {
            thread.0.cancel();
            self.rt.block_on(&mut thread.1).unwrap();
        }

        *self.segments.lock().unwrap() = None;
    }

    async fn run(
        client: Client,
        parent: String,
        config: Config,
        id: String,
        segments: SharedSegments,
        token: CancellationToken,
    ) {
        log::trace!("Fetching segments for {id}");

        let fetch = if config.privacy_api {
            let fun = fetch_with_privacy(config.server_address, id, config.categories, config.action_types);
            Either::Left(fun)
        } else {
            let fun = fetch(config.server_address, id, config.categories, config.action_types);
            Either::Right(fun)
        };

        let result = select! {
            s = fetch => s,
            _ = token.cancelled() => return,
        };

        // Lock only when data is received
        *segments.lock().unwrap() = match result {
            Ok(s) => {
                log::info!("{} segment(s) found", s.len());
                let _ = client.command(["script-message-to", &parent, "segments-fetched"]);
                Some(s)
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
        self.segments
            .lock()
            .unwrap()
            .as_ref()?
            .iter()
            .find(|s| s.action == Action::Skip && time_pos >= s.segment[0] && time_pos < (s.segment[1] - 0.1_f64)) // Fix for a stupid bug when times are too precise
            .cloned()
    }

    pub fn get_mute_segment(&self, time_pos: f64) -> Option<Segment> {
        self.segments
            .lock()
            .unwrap()
            .as_ref()?
            .iter()
            .find(|s| s.action == Action::Mute && time_pos >= s.segment[0] && time_pos < s.segment[1])
            .cloned()
    }

    pub fn get_video_poi(&self) -> Option<f64> {
        self.segments
            .lock()
            .unwrap()
            .as_ref()?
            .iter()
            .find(|s| s.action == Action::Poi)
            .map(|s| s.segment[0])
    }

    pub fn get_video_category(&self) -> Option<Category> {
        self.segments
            .lock()
            .unwrap()
            .as_ref()?
            .iter()
            .find(|s| s.action == Action::Full)
            .map(|s| s.category)
    }
}
