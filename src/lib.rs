#![feature(drain_filter)]
#![feature(if_let_guard)]

mod config;
mod sponsorblock;
mod utils;
mod worker;

use config::Config;
use mpv_client::{mpv_handle, Event, Handle};
use sponsorblock::Segment;
use worker::Worker;

use std::time::Duration;

use env_logger::Env;

static NAME_PROP_PATH: &str = "path";
static NAME_PROP_TIME: &str = "time-pos";
static NAME_PROP_MUTE: &str = "mute";

const REPL_PROP_TIME: u64 = 1;
const REPL_PROP_MUTE: u64 = 2;

struct State {
    worker: Worker,
    mute_segment: Option<Segment>,
    mute_sponsorblock: bool,
}

impl State {
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

    fn unmute(&mut self, mpv: &Handle) {
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

    fn user_mute(&mut self, value: String) {
        // If muted by the plugin and request unmute then plugin doesn't own mute
        if self.mute_sponsorblock && value == "no" {
            self.mute_sponsorblock = false;
        }
    }
}

// MPV entry point
#[no_mangle]
extern "C" fn mpv_open_cplugin(handle: *mut mpv_handle) -> std::os::raw::c_int {
    // TODO Maybe use MPV logger ?
    let env = Env::new()
        .filter("MPV_SPONSORBLOCK_LOG")
        .write_style("MPV_SPONSORBLOCK_LOG_STYLE");
    env_logger::init_from_env(env);

    // Wrap handle
    let mpv = Handle::from_ptr(handle);

    // Show that the plugin has started
    log::debug!("Starting plugin SponsorBlock [{}]!", mpv.client_name());

    // Read config
    let config = Config::default();

    // State handler of MPV
    let mut state: Option<State> = None;

    loop {
        // Wait for MPV events indefinitely
        match mpv.wait_event(-1.) {
            Event::StartFile(_) => {
                log::trace!("Received start-file event");

                state = Worker::new(config.clone(), mpv.get_property::<String>(NAME_PROP_PATH).unwrap()).and_then(|worker| {
                    mpv.observe_property::<f64>(REPL_PROP_TIME, NAME_PROP_TIME).unwrap();
                    mpv.observe_property::<String>(REPL_PROP_MUTE, NAME_PROP_MUTE).unwrap();

                    Some(State { worker, mute_segment: None, mute_sponsorblock: false })
                });
            }
            Event::PropertyChange(REPL_PROP_TIME, data) if let Some(state) = state.as_mut() => {
                log::trace!("Received {} on reply {}", data.name(), REPL_PROP_TIME);

                if let Some(time_pos) = data.data::<f64>() {
                    if let Some(s) = state.worker.get_skip_segment(time_pos) {
                        state.skip(&mpv, s); // Skip segments are priority
                    } else if let Some(s) = state.worker.get_mute_segment(time_pos) {
                        state.mute(&mpv, s);
                    } else {
                        state.unmute(&mpv);
                    }
                }
            }
            Event::PropertyChange(REPL_PROP_MUTE, data) if let Some(state) = state.as_mut() => {
                log::trace!("Received {} on reply {}", data.name(), REPL_PROP_MUTE);

                if let Some(mute) = data.data::<String>() {
                    state.user_mute(mute);
                }
            }
            Event::EndFile if let Some(state) = state.as_mut() => {
                log::trace!("Received end-file event");

                state.unmute(&mpv);
                mpv.unobserve_property(REPL_PROP_TIME).unwrap();
                mpv.unobserve_property(REPL_PROP_MUTE).unwrap();
            }
            Event::Shutdown => {
                log::trace!("Received shutdown event");

                return 0;
            }
            event => {
                log::trace!("Ignoring {} event", event);
            }
        }
    }
}
