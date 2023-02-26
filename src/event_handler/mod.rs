mod sponsorblock_worker;

use crate::config::Config;
use sponsorblock_worker::SponsorBlockWorker;

use std::time::Duration;

use mpv_client::{Format, Handle};
use regex::Regex;
use sponsorblock_client::Segment;

static NAME_PROP_PATH: &str = "path";
static NAME_PROP_TIME: &str = "time-pos";
static NAME_PROP_MUTE: &str = "mute";

pub const REPL_PROP_TIME: u64 = 1;
pub const REPL_PROP_MUTE: u64 = 2;

pub struct EventHandler<'a> {
    mpv: &'a Handle,
    config: &'a Config,
    worker: SponsorBlockWorker,
    mute_segment: Option<Segment>,
    mute_sponsorblock: bool,
}

impl<'a> EventHandler<'a> {
    pub fn new(mpv: &'a Handle, config: &'a Config) -> Option<Self> {
        let path: String = mpv.get_property(NAME_PROP_PATH).unwrap();
        let id = Self::get_youtube_id(&config, &path)?;

        let client_parent = mpv.client_name();
        let client = mpv.create_weak_client(format!("{}-worker", client_parent)).ok()?;

        let worker = SponsorBlockWorker::new(client, client_parent.to_string(), config.clone(), id.to_string());

        mpv.observe_property(REPL_PROP_TIME, NAME_PROP_TIME, f64::MPV_FORMAT)
            .unwrap();
        mpv.observe_property(REPL_PROP_MUTE, NAME_PROP_MUTE, bool::MPV_FORMAT)
            .unwrap();

        Some(Self {
            mpv,
            config,
            worker,
            mute_segment: None,
            mute_sponsorblock: false,
        })
    }

    fn get_youtube_id<'b>(config: &Config, path: &'b str) -> Option<&'b str> {
        let mut domains_patterns = vec![r"(?:www\.|m\.|)youtube\.com", r"(?:www\.|)youtu\.be"];
        config.domains_escaped.iter().for_each(|r| domains_patterns.push(r));

        let pattern = format!(
            r"https?://(?:{}).*(?:/|%3D|v=|vi=)([0-9A-z-_]{{11}})(?:[%#?&]|$)",
            domains_patterns.join("|")
        );

        log::trace!("YouTube ID regex pattern: {}", pattern);

        let regex = Regex::new(&pattern).ok()?;
        let capture = regex.captures(&path.as_ref())?;
        capture.get(1).map(|m| m.as_str())
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

    fn skip(&self, working_segment: Segment) {
        self.mpv
            .set_property(NAME_PROP_TIME, working_segment.segment[1])
            .unwrap();
        log::info!("Skipped segment {}", working_segment);
        if self.config.skip_notice {
            self.mpv
                .osd_message(format!("Skipped segment {}", working_segment), Duration::from_secs(8))
                .unwrap();
        }
    }

    fn mute(&mut self, working_segment: Segment) {
        // Working only if entering a new segment
        if self.mute_segment == Some(working_segment.clone()) {
            return;
        }

        // If muted by the plugin do it again just for the log or if not muted do it
        let mute: bool = self.mpv.get_property(NAME_PROP_MUTE).unwrap();
        if self.mute_sponsorblock || !mute {
            self.mpv.set_property(NAME_PROP_MUTE, true).unwrap();
            self.mute_sponsorblock = true;
            log::info!("Mutting segment {}", working_segment);
            if self.config.skip_notice {
                self.mpv
                    .osd_message(format!("Mutting segment {}", working_segment), Duration::from_secs(8))
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
            self.mpv.set_property(NAME_PROP_MUTE, false).unwrap();
            log::info!("Unmutting");
            self.mute_sponsorblock = false;
        } else {
            log::info!("Muttable segment(s) ended but mute value was modified. Ignoring");
        }

        self.mute_segment = None
    }

    fn poi_requested(&self) {
        if let Some(time_pos) = self.worker.get_video_poi() {
            self.mpv.set_property(NAME_PROP_TIME, time_pos).unwrap();
            log::info!("Jumping to highlight at {}", time_pos);
            if self.config.skip_notice {
                self.mpv
                    .osd_message(format!("Jumping to highlight at {}", time_pos), Duration::from_secs(8))
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
            self.mpv.osd_message(message, Duration::from_secs(10)).unwrap();
        }
    }
}

impl<'a> Drop for EventHandler<'a> {
    fn drop(&mut self) {
        self.reset();
        self.mpv.unobserve_property(REPL_PROP_TIME).unwrap();
        self.mpv.unobserve_property(REPL_PROP_MUTE).unwrap();
    }
}
