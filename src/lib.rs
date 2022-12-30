#![feature(drain_filter)]
#![feature(is_some_and)]

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

fn skip(mpv: &Handle, working_segment: Segment) {
    mpv.set_property(NAME_PROP_TIME, working_segment.segment[1]).unwrap();
    log::info!("Skipped segment {}", working_segment);
    mpv.osd_message(format!("Skipped segment {}", working_segment), Duration::from_secs(8))
        .unwrap();
}

fn mute(mpv: &Handle, working_segment: Segment, current_segment: &Option<Segment>, mute_sponsorblock: &mut bool) {
    // Working only if entering a new segment
    if current_segment
        .as_ref()
        .is_some_and(|ref segment| segment.uuid == working_segment.uuid)
    {
        return;
    }

    // If muted by the plugin do it again just for the log or if not muted do it
    if *mute_sponsorblock || mpv.get_property::<String>(NAME_PROP_MUTE).unwrap() != "yes" {
        mpv.set_property(NAME_PROP_MUTE, "yes".to_string()).unwrap();
        log::info!("Mutting segment {}", working_segment);
        mpv.osd_message(format!("Mutting segment {}", working_segment), Duration::from_secs(8))
            .unwrap();
        *mute_sponsorblock = true;
    } else {
        log::info!("Muttable segment found but mute was requested by user prior segment. Ignoring");
    }
}

fn unmute(mpv: &Handle, current_segment: &Option<Segment>, mute_sponsorblock: &mut bool) {
    // Working only if exiting segment
    if current_segment.is_none() {
        return;
    }

    // If muted the by plugin then unmute
    if *mute_sponsorblock {
        mpv.set_property(NAME_PROP_MUTE, "no".to_string()).unwrap();
        log::info!("Unmutting");
        *mute_sponsorblock = false;
    } else {
        log::info!("Muttable segment(s) ended but mute value was modified. Ignoring");
    }
}

fn user_mute(value: String, mute_sponsorblock: &mut bool) {
    // If muted by the plugin and request unmute then plugin doesn't own mute
    if *mute_sponsorblock && value == "no" {
        *mute_sponsorblock = false;
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

    // Create SponsorBlock worker
    let mut worker: Option<Worker> = None;

    // Boolean to check if we are currently in a mutted segment
    let mut mute_segment: Option<Segment> = None;
    let mut mute_sponsorblock: bool = false;

    // Subscribe to property time-pos and mute
    mpv.observe_property::<f64>(REPL_PROP_TIME, NAME_PROP_TIME).unwrap();
    mpv.observe_property::<String>(REPL_PROP_MUTE, NAME_PROP_MUTE).unwrap();

    loop {
        // Wait for MPV events indefinitely
        match mpv.wait_event(-1.) {
            Event::StartFile(_) => {
                log::trace!("Received start-file event");
                mute_segment = None;
                worker = Worker::new(config.clone(), mpv.get_property::<String>(NAME_PROP_PATH).unwrap());
            }
            Event::PropertyChange(REPL_PROP_TIME, data) if worker.is_some() => {
                log::trace!("Received {} on reply {}", data.name(), REPL_PROP_TIME);
                if let Some(time_pos) = data.data::<f64>() {
                    if let Some(s) = worker.as_ref().and_then(|w| w.get_skip_segment(time_pos)) {
                        skip(&mpv, s); // Skip segments are priority
                    } else if let Some(s) = worker.as_ref().and_then(|w| w.get_mute_segment(time_pos)) {
                        mute(&mpv, s.clone(), &mute_segment, &mut mute_sponsorblock);
                        mute_segment = Some(s);
                    } else {
                        unmute(&mpv, &mute_segment, &mut mute_sponsorblock);
                        mute_segment = None;
                    }
                }
            }
            Event::PropertyChange(REPL_PROP_MUTE, data) if worker.is_some() => {
                log::trace!("Received {} on reply {}", data.name(), REPL_PROP_MUTE);
                if let Some(mute) = data.data::<String>() {
                    user_mute(mute, &mut mute_sponsorblock);
                }
            }
            Event::EndFile if worker.is_some() => {
                log::trace!("Received end-file event");
                unmute(&mpv, &mute_segment, &mut mute_sponsorblock);
                worker = None;
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
