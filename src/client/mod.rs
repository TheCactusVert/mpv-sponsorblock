mod config;
mod worker;

use config::Config;
use worker::Worker;

use std::ops::Deref;
use std::time::Duration;

use mpv_client::{mpv_handle, Event, Format, Handle, Result};
use regex::Regex;
use sponsorblock_client::Segment;

static NAME_PROP_PATH: &str = "path";
static NAME_PROP_TIME: &str = "time-pos";
static NAME_PROP_MUTE: &str = "mute";

const REPL_PROP_TIME: u64 = 1;
const REPL_PROP_MUTE: u64 = 2;

pub struct Client {
    mpv: Handle,
    config: Config,
    worker: Worker,
    mute_segment: Option<Segment>,
    mute_sponsorblock: bool,
}

impl Client {
    pub fn from_ptr(handle: *mut mpv_handle) -> Self {
        Self {
            mpv: Handle::from_ptr(handle),
            config: Config::get(),
            worker: Worker::new(),
            mute_segment: None,
            mute_sponsorblock: false,
        }
    }

    pub fn exec(&mut self) -> Result<()> {
        loop {
            // Wait for MPV events indefinitely
            match self.wait_event(-1.) {
                Event::StartFile(_data) => {
                    log::trace!("Received start-file event");
                    self.start_file()?;
                }
                Event::PropertyChange(REPL_PROP_TIME, data) => {
                    log::trace!("Received {} on reply {}", data.name(), REPL_PROP_TIME);
                    if let Some(time) = data.data() {
                        self.time_change(time)?;
                    }
                }
                Event::PropertyChange(REPL_PROP_MUTE, data) => {
                    log::trace!("Received {} on reply {}", data.name(), REPL_PROP_MUTE);
                    if let Some(mute) = data.data() {
                        self.mute_change(mute)?;
                    }
                }
                Event::ClientMessage(data) => {
                    log::trace!("Received client-message event");
                    self.client_message(data.args().as_slice())?;
                }
                Event::EndFile => {
                    log::trace!("Received end-file event");
                    self.end_file()?;
                }
                Event::Shutdown => {
                    log::trace!("Received shutdown event");
                    return Ok(());
                }
                _ => {}
            };
        }
    }

    fn start_file(&mut self) -> Result<()> {
        let path: String = self.get_property(NAME_PROP_PATH)?;
        if let Some(id) = self.get_youtube_id(&path) {
            let client_parent = self.client_name();

            self.worker.start(
                self.create_client(format!("{}-worker", client_parent))?,
                client_parent.to_string(),
                self.config.clone(),
                id.to_string(),
            );

            self.observe_property(REPL_PROP_TIME, NAME_PROP_TIME, f64::MPV_FORMAT)?;
            self.observe_property(REPL_PROP_MUTE, NAME_PROP_MUTE, bool::MPV_FORMAT)?;
        }

        Ok(())
    }

    pub fn time_change(&mut self, time_pos: f64) -> Result<()> {
        if let Some(s) = self.worker.get_skip_segment(time_pos) {
            self.skip(s) // Skip segments are priority
        } else if let Some(s) = self.worker.get_mute_segment(time_pos) {
            self.mute(s)
        } else {
            self.reset()
        }
    }

    fn mute_change(&mut self, mute: bool) -> Result<()> {
        // If muted by the plugin and request unmute then plugin doesn't own mute
        if self.mute_sponsorblock && !mute {
            self.mute_sponsorblock = false;
        };
        Ok(())
    }

    fn client_message(&self, args: &[&str]) -> Result<()> {
        match args {
            ["key-binding", "poi", "u-", ..] => self.poi_requested(),
            ["segments-fetched"] => self.segments_fetched(),
            _ => Ok(()),
        }
    }

    fn end_file(&mut self) -> Result<()> {
        self.worker.stop();
        self.reset()?;
        self.unobserve_property(REPL_PROP_TIME)?;
        self.unobserve_property(REPL_PROP_MUTE)?;
        Ok(())
    }

    fn get_youtube_id<'b>(&self, path: &'b str) -> Option<&'b str> {
        let mut domains_patterns = vec![r"(?:www\.|m\.|)youtube\.com", r"(?:www\.|)youtu\.be"];
        self.config
            .domains_escaped
            .iter()
            .for_each(|r| domains_patterns.push(r));

        let pattern = format!(
            r"https?://(?:{}).*(?:/|%3D|v=|vi=)([0-9A-z-_]{{11}})(?:[%#?&]|$)",
            domains_patterns.join("|")
        );

        log::trace!("YouTube ID regex pattern: {}", pattern);

        let regex = Regex::new(&pattern).ok()?;
        let capture = regex.captures(&path.as_ref())?;
        capture.get(1).map(|m| m.as_str())
    }

    fn skip(&self, working_segment: Segment) -> Result<()> {
        self.set_property(NAME_PROP_TIME, working_segment.segment[1])?;
        log::info!("Skipped segment {}", working_segment);
        if self.config.skip_notice {
            self.osd_message(format!("Skipped segment {}", working_segment), Duration::from_secs(8))?;
        }
        Ok(())
    }

    fn mute(&mut self, working_segment: Segment) -> Result<()> {
        // Working only if entering a new segment
        if self.mute_segment != Some(working_segment.clone()) {
            // If muted by the plugin do it again just for the log or if not muted do it
            let mute: bool = self.get_property(NAME_PROP_MUTE).unwrap();
            if self.mute_sponsorblock || !mute {
                self.set_property(NAME_PROP_MUTE, true).unwrap();
                self.mute_sponsorblock = true;
                log::info!("Mutting segment {}", working_segment);
                if self.config.skip_notice {
                    self.osd_message(format!("Mutting segment {}", working_segment), Duration::from_secs(8))
                        .unwrap();
                }
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

    fn poi_requested(&self) -> Result<()> {
        if let Some(time_pos) = self.worker.get_video_poi() {
            self.set_property(NAME_PROP_TIME, time_pos)?;
            log::info!("Jumping to highlight at {}", time_pos);
            if self.config.skip_notice {
                self.osd_message(format!("Jumping to highlight at {}", time_pos), Duration::from_secs(8))?;
            }
        }
        Ok(())
    }

    fn segments_fetched(&self) -> Result<()> {
        if let Some(category) = self.worker.get_video_category() {
            let message = format!(
                "This entire video is labeled as '{}' and is too tightly integrated to be able to separate",
                category
            );
            self.osd_message(message, Duration::from_secs(10))?;
        }
        Ok(())
    }
}

impl Deref for Client {
    type Target = Handle;

    #[inline]
    fn deref(&self) -> &Handle {
        &self.mpv
    }
}
