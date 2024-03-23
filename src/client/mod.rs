mod config;

use config::Config;

use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_channel::Sender;
use mpv_client::{mpv_handle, osd, Client as MpvClient, ClientMessage, Event, Handle, Property, Result};
use regex::Regex;
use sponsorblock_client::*;
use tokio::runtime::Runtime;
use tokio::select;

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
    Cancel,
}

pub struct Client {
    handle: *mut mpv_handle,
    config: Config,
    segments: SharedSegments,
    mute_segment: Option<Segment>,
    mute_sponsorblock: bool,
    is_enabled: bool,
    user_toggle: bool,
}

impl Client {
    pub fn from_ptr(handle: *mut mpv_handle) -> Self {
        Self {
            handle,
            config: Config::get(),
            segments: SharedSegments::default(),
            mute_segment: None,
            mute_sponsorblock: false,
            is_enabled: false,
            user_toggle: true,
        }
    }

    fn get_youtube_id<'b>(r: &Regex, path: &'b str) -> Option<&'b str> {
        let capture = r.captures(path.as_ref())?;
        capture.get(1).map(|m| m.as_str())
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
            let mut sponsorblock = SponsorBlock::new().unwrap();
            sponsorblock.set_server_address(config.server_address);
            sponsorblock.set_private_api(config.privacy_api);

            'wait: loop {
                // Wait for an event
                let mut event: WorkerEvent = rx.recv().await.unwrap();

                'event: loop {
                    // Handle worker event
                    let path = match event {
                        WorkerEvent::Path(path) => path,
                        WorkerEvent::Cancel => return,
                    };

                    // Extract YouTube ID if it exists
                    let id = match Self::get_youtube_id(&config.youtube_regex, &path) {
                        Some(id) => id,
                        None => continue 'wait, // Wait for a new event
                    };

                    log::trace!("Fetching segments for {id}");

                    let seg = select! {
                        // Fetch data
                        s = sponsorblock.fetch(id.into(), config.categories.clone(), config.action_types.clone()) => {
                            s.ok()
                        },
                        // Event received while fetching content
                        e = rx.recv() => {
                            event = e.unwrap();
                            continue 'event; // Handle event and stop all operations
                        },
                    };

                    // Lock segments
                    let mut segments = segments.lock().unwrap();
                    *segments = seg;
                    if segments.is_some() {
                        // Send message to parent
                        let _ = client.command(["script-message-to", &parent, "segments-fetched"]);
                    }
                    continue 'wait; // Wait for a new event
                }
            }
        });

        self.exec_loop(tx.clone());

        // Cancel worker
        tx.send_blocking(WorkerEvent::Cancel).unwrap();
        rt.block_on(&mut handle).unwrap();
        Ok(())
    }

    fn exec_loop(&mut self, tx: async_channel::Sender<WorkerEvent>) {
        loop {
            // Wait for MPV events indefinitely
            let result = match self.wait_event(-1.) {
                Event::StartFile(_data) => self.start_file(&tx),
                Event::PropertyChange(REPL_PROP_TIME, data) => self.time_change(data),
                Event::PropertyChange(REPL_PROP_MUTE, data) => self.mute_change(data),
                Event::ClientMessage(data) => self.client_message(data),
                Event::EndFile(_data) => self.end_file(),
                Event::Shutdown => break,
                _ => Ok(()),
            };

            if let Err(e) = result {
                log::error!("Unhandled error on plugin SponsorBlock [{}]: {}", self.name(), e);
            }
        }
    }

    fn start_file(&mut self, tx: &Sender<WorkerEvent>) -> Result<()> {
        log::trace!("Received start-file event");
        tx.send_blocking(WorkerEvent::Path(self.get_property(NAME_PROP_PATH)?))
            .unwrap();
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

    fn mute_change(&mut self, data: Property) -> Result<()> {
        log::trace!("Received property-change event [{data}]");
        if data.data() == Some(false) {
            self.mute_sponsorblock = false;
        };
        Ok(())
    }

    fn client_message(&mut self, data: ClientMessage) -> Result<()> {
        log::trace!("Received client-message event");
        match data.args().as_slice() {
            ["key-binding", "info", "u-", ..] => self.info_requested(),
            ["key-binding", "poi", "u-", ..] => self.poi_requested(),
            ["key-binding", "toggle", "u-", ..] => self.toggle_requested(),
            ["segments-fetched"] => self.segments_fetched(),
            _ => Ok(()),
        }
    }

    fn end_file(&mut self) -> Result<()> {
        log::trace!("Received end-file event");
        self.disable()?;
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

    fn info_requested(&mut self) -> Result<()> {
        let current_time: f64 = self.get_property(NAME_PROP_TIME)?;
        let segments_list = self
            .segments
            .lock()
            .unwrap()
            .iter()
            .flatten()
            .filter(|s| (s.action == Action::Skip || s.action == Action::Mute) && s.segment[0] >= current_time)
            .map(|s| s.to_string())
            .take(5)
            .collect::<Vec<String>>()
            .join("\n");
        let poi = self.get_video_poi();
        let category = self.get_video_category();
        let enabled = self.is_enabled;

        let _ = osd!(
            self,
            Duration::from_secs(12),
            "Next segments:\n{segments_list}\n\nHighlight: {}\n\nCategory: {}\n\nEnabled: {enabled}",
            poi.map_or_else(|| "None".to_string(), |v| v.to_string()),
            category.map_or_else(|| "None".to_string(), |v| v.to_string()),
        );
        Ok(())
    }

    fn poi_requested(&mut self) -> Result<()> {
        if let Some(time_pos) = self.get_video_poi() {
            self.set_property(NAME_PROP_TIME, time_pos)?;
            osd_info!(self, Duration::from_secs(8), "Jumping to highlight at {time_pos}");
        }
        Ok(())
    }

    fn toggle_requested(&mut self) -> Result<()> {
        self.user_toggle = !self.user_toggle;
        let name = self.name();
        if self.user_toggle {
            let _ = osd!(self, Duration::from_secs(4), "Plugin enabled [{}]", name);
            self.enable()?;
        } else {
            let _ = osd!(self, Duration::from_secs(4), "Plugin disabled [{}]", name);
            self.disable()?;
        }
        Ok(())
    }

    fn segments_fetched(&mut self) -> Result<()> {
        self.enable()?;
        if let Some(category) = self.get_video_category() {
            let _ = osd!(
                self,
                Duration::from_secs(10),
                "This entire video is labeled as '{category}' and is too tightly integrated to be able to separate"
            );
        }
        Ok(())
    }

    fn enable(&mut self) -> Result<()> {
        // The plugin is disabled and user allow plugin to run nad segments are fetched
        if !self.is_enabled && self.user_toggle && self.fetched() {
            self.is_enabled = true;
            self.observe_property::<f64>(REPL_PROP_TIME, NAME_PROP_TIME)?;
            self.observe_property::<bool>(REPL_PROP_MUTE, NAME_PROP_MUTE)?;
        }
        Ok(())
    }

    fn disable(&mut self) -> Result<()> {
        // The plugin is enabled
        if self.is_enabled {
            self.is_enabled = false;
            self.unobserve_property(REPL_PROP_TIME)?;
            self.unobserve_property(REPL_PROP_MUTE)?;
        }
        Ok(())
    }

    fn fetched(&self) -> bool {
        self.segments.lock().unwrap().is_some()
    }

    fn segment_where<P>(&self, predicate: P) -> Option<Segment>
    where
        P: FnMut(&&Segment) -> bool,
    {
        // cloning is cheap since it is a [f64; 2]
        self.segments.lock().unwrap().as_ref()?.iter().find(predicate).cloned()
    }

    fn get_skip_segment(&self, time_pos: f64) -> Option<Segment> {
        self.segment_where(|s| {
            s.action == Action::Skip && time_pos >= s.segment[0] && time_pos < (s.segment[1] - 0.1_f64)
        }) // Fix for a stupid bug when times are too precise
    }

    fn get_mute_segment(&self, time_pos: f64) -> Option<Segment> {
        self.segment_where(|s| s.action == Action::Mute && time_pos >= s.segment[0] && time_pos < s.segment[1])
    }

    fn get_video_poi(&self) -> Option<f64> {
        self.segment_where(|s| s.action == Action::Poi).map(|s| s.segment[0])
    }

    fn get_video_category(&self) -> Option<Category> {
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
