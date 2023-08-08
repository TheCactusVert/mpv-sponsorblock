mod config;

use config::Config;

use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_channel::Sender;
use mpv_client::{mpv_handle, osd, Client as MpvClient, ClientMessage, Event, Format, Handle, Property, Result};
use reqwest::StatusCode;
use sponsorblock_client::*;
use tokio::runtime::Runtime;
use tokio::select;
use tokio_util::either::Either;

type SharedSegments = Arc<Mutex<Option<Segments>>>;

static NAME_PROP_PATH: &str = "path";
static NAME_PROP_TIME: &str = "time-pos";
static NAME_PROP_MUTE: &str = "mute";

const REPL_PROP_TIME: u64 = 1;
const REPL_PROP_MUTE: u64 = 2;

macro_rules! osd_info {
    ($client:expr, $duration:expr, $($arg:tt)*) => {
        log::info!($($arg)*);
        if $client.config.skip_notice {
            let _ = osd!($client, $duration, $($arg)*);
        }
    };
}

enum WorkerEvent {
    Path(String),
    Exit,
}

pub struct Client {
    handle: *mut mpv_handle,
    config: Config,
    segments: SharedSegments,
    mute_segment: Option<Segment>,
    mute_sponsorblock: bool,
}

impl Client {
    pub fn from_ptr(handle: *mut mpv_handle) -> Self {
        Self {
            handle,
            config: Config::get(),
            segments: SharedSegments::default(),
            mute_segment: None,
            mute_sponsorblock: false,
        }
    }

    fn get_youtube_id<'b>(config: &Config, path: &'b str) -> Option<&'b str> {
        let capture = config.youtube_regex.captures(path.as_ref())?;
        capture.get(1).map(|m| m.as_str())
    }

    fn into_segments(s: reqwest::Result<Segments>) -> Option<Segments> {
        match s {
            Ok(s) => {
                log::info!("{} segment(s) found", s.len());
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

    pub fn exec(&mut self) -> Result<()> {
        let config = self.config.clone();
        let (tx, rx) = async_channel::unbounded();

        let segments = self.segments.clone();

        let parent: String = self.name().into();
        let child = format!("{}-worker", parent);
        let mut client: MpvClient = self.create_client(child)?;

        let rt = Runtime::new().unwrap();

        let mut handle = rt.spawn(async move {
            'wait: loop {
                let mut event: WorkerEvent = rx.recv().await.unwrap();

                'event: loop {
                    let path = match &event {
                        WorkerEvent::Path(path) => path,
                        WorkerEvent::Exit => return,
                    };

                    let id = match Self::get_youtube_id(&config, &path) {
                        Some(id) => id,
                        None => continue,
                    };

                    log::trace!("Fetching segments for {id}");

                    let fetch = if config.privacy_api {
                        let fun = fetch_with_privacy(
                            config.server_address.clone(),
                            id.into(),
                            config.categories.clone(),
                            config.action_types.clone(),
                        );
                        Either::Left(fun)
                    } else {
                        let fun = fetch(
                            config.server_address.clone(),
                            id.into(),
                            config.categories.clone(),
                            config.action_types.clone(),
                        );
                        Either::Right(fun)
                    };

                    select! {
                        s = fetch => {
                            *segments.lock().unwrap() = Self::into_segments(s);
                            let _ = client.command(["script-message-to", &parent, "segments-fetched"]);
                            continue 'wait;
                        },
                        e = rx.recv() => {
                            event = e.unwrap();
                            continue 'event;
                        },
                    };
                }
            }
        });

        loop {
            // Wait for MPV events indefinitely
            match self.wait_event(-1.) {
                Event::StartFile(_data) => self.start_file(&tx)?,
                Event::FileLoaded => self.loaded_file()?,
                Event::PropertyChange(REPL_PROP_TIME, data) => self.time_change(data)?,
                Event::PropertyChange(REPL_PROP_MUTE, data) => self.mute_change(data),
                Event::ClientMessage(data) => self.client_message(data)?,
                Event::EndFile(_data) => self.end_file()?,
                Event::Shutdown => break,
                _ => {}
            };
        }

        tx.send_blocking(WorkerEvent::Exit);
        rt.block_on(&mut handle).unwrap();

        Ok(())
    }

    fn start_file(&mut self, tx: &Sender<WorkerEvent>) -> Result<()> {
        log::trace!("Received start-file event");
        tx.send_blocking(WorkerEvent::Path(self.get_property(NAME_PROP_PATH)?));
        Ok(())
    }

    fn loaded_file(&mut self) -> Result<()> {
        log::trace!("Received file-loaded event");
        self.observe_property(REPL_PROP_TIME, NAME_PROP_TIME, f64::MPV_FORMAT)?;
        self.observe_property(REPL_PROP_MUTE, NAME_PROP_MUTE, bool::MPV_FORMAT)?;
        Ok(())
    }

    fn time_change(&mut self, data: Property) -> Result<()> {
        log::trace!("Received property-change event [{data}]");
        // Skipping before a certain time can lead to undefined behaviour
        // https://github.com/TheCactusVert/mpv-sponsorblock/issues/5
        if let Some(time_pos) = data.data().filter(|t| t >= &0.5_f64) {
            if let Some(s) = self.get_skip_segment(time_pos) {
                self.skip(s) // Skip segments are priority
            } else if let Some(s) = self.get_mute_segment(time_pos) {
                self.mute(s)
            } else {
                self.reset()
            }
        } else {
            Ok(())
        }
    }

    fn mute_change(&mut self, data: Property) {
        log::trace!("Received property-change event [{data}]");
        if data.data() == Some(false) {
            self.mute_sponsorblock = false;
        };
    }

    fn client_message(&mut self, data: ClientMessage) -> Result<()> {
        log::trace!("Received client-message event");
        match data.args().as_slice() {
            ["key-binding", "poi", "u-", ..] => self.poi_requested()?,
            ["segments-fetched"] => self.segments_fetched(),
            _ => {}
        };
        Ok(())
    }

    fn end_file(&mut self) -> Result<()> {
        log::trace!("Received end-file event");
        self.unobserve_property(REPL_PROP_TIME)?;
        self.unobserve_property(REPL_PROP_MUTE)?;
        self.reset()?;
        Ok(())
    }

    fn skip(&mut self, working_segment: Segment) -> Result<()> {
        self.set_property(NAME_PROP_TIME, working_segment.segment[1])?;
        osd_info!(self, Duration::from_secs(8), "Skipped segment {working_segment}");
        Ok(())
    }

    fn mute(&mut self, working_segment: Segment) -> Result<()> {
        // Working only if entering a new segment
        if self.mute_segment != Some(working_segment.clone()) {
            // If muted by the plugin do it again just for the log or if not muted do it
            let mute: bool = self.get_property(NAME_PROP_MUTE)?;
            if self.mute_sponsorblock || !mute {
                self.set_property(NAME_PROP_MUTE, true)?;
                self.mute_sponsorblock = true;
                osd_info!(self, Duration::from_secs(8), "Mutting segment {working_segment}");
            } else {
                log::info!("Muttable segment found but mute was requested by user prior segment. Ignoring");
            }

            self.mute_segment = Some(working_segment);
        }

        Ok(())
    }

    fn reset(&mut self) -> Result<()> {
        // Working only if exiting segment
        if self.mute_segment.is_some() {
            // If muted the by plugin then unmute
            if self.mute_sponsorblock {
                self.set_property(NAME_PROP_MUTE, false)?;
                log::info!("Unmutting");
                self.mute_sponsorblock = false;
            } else {
                log::info!("Muttable segment(s) ended but mute value was modified. Ignoring");
            }

            self.mute_segment = None;
        }

        Ok(())
    }

    fn poi_requested(&mut self) -> Result<()> {
        if let Some(time_pos) = self.get_video_poi() {
            self.set_property(NAME_PROP_TIME, time_pos)?;
            osd_info!(self, Duration::from_secs(8), "Jumping to highlight at {time_pos}");
        }
        Ok(())
    }

    fn segments_fetched(&mut self) {
        if let Some(category) = self.get_video_category() {
            let _ = osd!(
                self,
                Duration::from_secs(10),
                "This entire video is labeled as '{category}' and is too tightly integrated to be able to separate"
            );
        }
    }

    fn segment_where<P>(&self, predicate: P) -> Option<Segment>
    where
        P: FnMut(&&Segment) -> bool,
    {
        // cloning is cheap since it is a [f64; 2]
        self.segments.lock().unwrap().as_ref()?.iter().find(predicate).cloned()
    }

    pub fn get_skip_segment(&self, time_pos: f64) -> Option<Segment> {
        self.segment_where(|s| {
            s.action == Action::Skip && time_pos >= s.segment[0] && time_pos < (s.segment[1] - 0.1_f64)
        }) // Fix for a stupid bug when times are too precise
    }

    pub fn get_mute_segment(&self, time_pos: f64) -> Option<Segment> {
        self.segment_where(|s| s.action == Action::Mute && time_pos >= s.segment[0] && time_pos < s.segment[1])
    }

    pub fn get_video_poi(&self) -> Option<f64> {
        self.segment_where(|s| s.action == Action::Poi).map(|s| s.segment[0])
    }

    pub fn get_video_category(&self) -> Option<Category> {
        self.segment_where(|s| s.action == Action::Full).map(|s| s.category)
    }
}

impl Deref for Client {
    type Target = Handle;

    #[inline]
    fn deref(&self) -> &Self::Target {
        Handle::from_ptr(self.handle)
    }
}

impl DerefMut for Client {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        Handle::from_ptr(self.handle)
    }
}
