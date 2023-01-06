mod sponsorblock_worker;

use crate::config::Config;
use sponsorblock_worker::SponsorBlockWorker;

use std::time::Duration;

use mpv_client::Handle;
use regex::Regex;
use sponsorblock_client::Segment;

static NAME_PROP_PATH: &str = "path";
static NAME_PROP_TIME: &str = "time-pos";
static NAME_PROP_MUTE: &str = "mute";

pub const REPL_PROP_TIME: u64 = 1;
pub const REPL_PROP_MUTE: u64 = 2;

pub struct EventHandler {
    worker: SponsorBlockWorker,
    mute_segment: Option<Segment>,
    mute_sponsorblock: bool,
}

impl EventHandler {
    pub fn new(mpv: &Handle, config: Config) -> Option<Self> {
        let path = mpv.get_property::<String>(NAME_PROP_PATH).unwrap();
        let id = Self::get_youtube_id(&config, &path)?;

        let worker = SponsorBlockWorker::new(config, id.to_string());

        mpv.observe_property::<f64>(REPL_PROP_TIME, NAME_PROP_TIME).unwrap();
        mpv.observe_property::<String>(REPL_PROP_MUTE, NAME_PROP_MUTE).unwrap();

        Some(Self {
            worker,
            mute_segment: None,
            mute_sponsorblock: false,
        })
    }

    fn get_youtube_id<'a>(config: &Config, path: &'a str) -> Option<&'a str> {
        let mut domains_patterns = vec![r"(?:www\.|m\.|)youtube\.com", r"(?:www\.|)youtu\.be"];
        config.domains_escaped.iter().for_each(|r| domains_patterns.push(r));

        let pattern = format!(
            r"https?://(?:{}).*(?:/|%3D|v=|vi=)([0-9A-z-_]{{11}})(?:[%#?&]|$)",
            domains_patterns.join("|")
        );

        log::trace!("Regex pattern: {}", pattern);

        let regex = Regex::new(&pattern).ok()?;
        let capture = regex.captures(&path.as_ref())?;
        capture.get(1).map(|m| m.as_str())
    }

    pub fn time_change(&mut self, mpv: &Handle, time_pos: f64) {
        if let Some(s) = self.worker.get_skip_segment(time_pos) {
            self.skip(&mpv, s); // Skip segments are priority
        } else if let Some(s) = self.worker.get_mute_segment(time_pos) {
            self.mute(&mpv, s);
        } else {
            self.reset(&mpv);
        }
    }

    pub fn mute_change(&mut self, mute: String) {
        // If muted by the plugin and request unmute then plugin doesn't own mute
        if self.mute_sponsorblock && mute == "no" {
            self.mute_sponsorblock = false;
        }
    }

    pub fn end_file(&mut self, mpv: &Handle) {
        self.reset(&mpv);
        mpv.unobserve_property(REPL_PROP_TIME).unwrap();
        mpv.unobserve_property(REPL_PROP_MUTE).unwrap();
    }

    fn skip(&self, mpv: &Handle, working_segment: Segment) {
        mpv.set_property(NAME_PROP_TIME, working_segment.segment[1]).unwrap();
        log::info!("Skipped segment {}", working_segment);
        mpv.osd_message(format!("Skipped segment {}", working_segment), Duration::from_secs(8))
            .unwrap();
    }

    fn mute(&mut self, mpv: &Handle, working_segment: Segment) {
        // Working only if entering a new segment
        if self.mute_segment == Some(working_segment.clone()) {
            return;
        }

        // If muted by the plugin do it again just for the log or if not muted do it
        if self.mute_sponsorblock || mpv.get_property::<String>(NAME_PROP_MUTE).unwrap() != "yes" {
            mpv.set_property(NAME_PROP_MUTE, "yes".to_string()).unwrap();
            log::info!("Mutting segment {}", working_segment);
            mpv.osd_message(format!("Mutting segment {}", working_segment), Duration::from_secs(8))
                .unwrap();
            self.mute_sponsorblock = true;
        } else {
            log::info!("Muttable segment found but mute was requested by user prior segment. Ignoring");
        }

        self.mute_segment = Some(working_segment);
    }

    fn reset(&mut self, mpv: &Handle) {
        // Working only if exiting segment
        if self.mute_segment.is_none() {
            return;
        }

        // If muted the by plugin then unmute
        if self.mute_sponsorblock {
            mpv.set_property(NAME_PROP_MUTE, "no".to_string()).unwrap();
            log::info!("Unmutting");
            self.mute_sponsorblock = false;
        } else {
            log::info!("Muttable segment(s) ended but mute value was modified. Ignoring");
        }

        self.mute_segment = None
    }
}
