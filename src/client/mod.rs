mod worker;

use crate::config::Config;
use worker::Worker;

use std::ops::Deref;
use std::time::Duration;

use mpv_client::{mpv_handle, Format, Handle};
use regex::Regex;
use sponsorblock_client::Segment;

static NAME_PROP_PATH: &str = "path";
static NAME_PROP_TIME: &str = "time-pos";
static NAME_PROP_MUTE: &str = "mute";

pub const REPL_PROP_TIME: u64 = 1;
pub const REPL_PROP_MUTE: u64 = 2;

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

    pub fn start_file(&mut self) {
        let path: String = self.get_property(NAME_PROP_PATH).unwrap();
        if let Some(id) = self.get_youtube_id(&path) {
            let client_parent = self.client_name();

            self.worker.start(
                self.create_client(format!("{}-worker", client_parent)).unwrap(),
                client_parent.to_string(),
                self.config.clone(),
                id.to_string(),
            );

            self.observe_property(REPL_PROP_TIME, NAME_PROP_TIME, f64::MPV_FORMAT)
                .unwrap();
            self.observe_property(REPL_PROP_MUTE, NAME_PROP_MUTE, bool::MPV_FORMAT)
                .unwrap();
        }
    }

    pub fn time_change(&mut self, time_pos: f64) {
        if let Some(s) = self.worker.get_skip_segment(time_pos) {
            self.skip(s); // Skip segments are priority
        } else if let Some(s) = self.worker.get_mute_segment(time_pos) {
            self.mute(s);
        } else {
            self.reset();
        }
    }

    pub fn mute_change(&mut self, mute: bool) {
        // If muted by the plugin and request unmute then plugin doesn't own mute
        if self.mute_sponsorblock && !mute {
            self.mute_sponsorblock = false;
        }
    }

    pub fn client_message(&mut self, args: &[&str]) {
        match args {
            ["key-binding", "poi", "u-", ..] => self.poi_requested(),
            ["segments-fetched"] => self.segments_fetched(),
            _ => {}
        };
    }

    pub fn end_file(&mut self) {
        self.worker.stop();
        self.reset();
        self.unobserve_property(REPL_PROP_TIME).unwrap();
        self.unobserve_property(REPL_PROP_MUTE).unwrap();
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

    fn skip(&self, working_segment: Segment) {
        self.set_property(NAME_PROP_TIME, working_segment.segment[1]).unwrap();
        log::info!("Skipped segment {}", working_segment);
        if self.config.skip_notice {
            self.osd_message(format!("Skipped segment {}", working_segment), Duration::from_secs(8))
                .unwrap();
        }
    }

    fn mute(&mut self, working_segment: Segment) {
        // Working only if entering a new segment
        if self.mute_segment == Some(working_segment.clone()) {
            return;
        }

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

    fn reset(&mut self) {
        // Working only if exiting segment
        if self.mute_segment.is_none() {
            return;
        }

        // If muted the by plugin then unmute
        if self.mute_sponsorblock {
            self.set_property(NAME_PROP_MUTE, false).unwrap();
            log::info!("Unmutting");
            self.mute_sponsorblock = false;
        } else {
            log::info!("Muttable segment(s) ended but mute value was modified. Ignoring");
        }

        self.mute_segment = None
    }

    fn poi_requested(&self) {
        if let Some(time_pos) = self.worker.get_video_poi() {
            self.set_property(NAME_PROP_TIME, time_pos).unwrap();
            log::info!("Jumping to highlight at {}", time_pos);
            if self.config.skip_notice {
                self.osd_message(format!("Jumping to highlight at {}", time_pos), Duration::from_secs(8))
                    .unwrap();
            }
        }
    }

    fn segments_fetched(&self) {
        if let Some(category) = self.worker.get_video_category() {
            let message = format!(
                "This entire video is labeled as '{}' and is too tightly integrated to be able to separate",
                category
            );
            self.osd_message(message, Duration::from_secs(10)).unwrap();
        }
    }
}

impl Deref for Client {
    type Target = Handle;

    #[inline]
    fn deref(&self) -> &Handle {
        &self.mpv
    }
}
