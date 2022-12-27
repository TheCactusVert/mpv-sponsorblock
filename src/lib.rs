#![feature(drain_filter)]
#![feature(is_some_and)]

mod actions;
mod config;
mod sponsorblock;
mod utils;

use actions::Actions;
use mpv_client::{mpv_handle, Event, Handle};
use sponsorblock::segment::Segment;

use std::time::Duration;

use env_logger::Env;

static NAME_PROP_PATH: &str = "path";
static NAME_PROP_TIME: &str = "time-pos";
static NAME_PROP_MUTE: &str = "mute";
static NAME_HOOK_END: &str = "on_after_end_file";

const REPL_PROP_TIME: u64 = 1;
const REPL_PROP_MUTE: u64 = 2;
const REPL_HOOK_END: u64 = 3;

const PRIO_HOOK_NONE: i32 = 0;

fn skip(mpv: &Handle, working_segment: Segment) {
    log::info!("Skipping {}", working_segment);
    mpv.set_property(NAME_PROP_TIME, working_segment.segment[1]).unwrap();
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
        log::info!("Mutting {}", working_segment);
        mpv.set_property(NAME_PROP_MUTE, "yes".to_string()).unwrap();
        mpv.osd_message(format!("Mutting {}", working_segment), Duration::from_secs(8))
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
        log::info!("Unmutting");
        mpv.set_property(NAME_PROP_MUTE, "no".to_string()).unwrap();
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

    // Create actions handler
    let mut actions = Actions::default();

    // Boolean to check if we are currently in a mutted segment
    let mut mute_segment: Option<Segment> = None;
    let mut mute_sponsorblock: bool = false;

    // Subscribe to property time-pos
    mpv.observe_property::<f64>(REPL_PROP_TIME, NAME_PROP_TIME).unwrap();

    // Subscribe to property mute
    mpv.observe_property::<String>(REPL_PROP_MUTE, NAME_PROP_MUTE).unwrap();

    // Add hook on file unload
    mpv.hook_add(REPL_HOOK_END, NAME_HOOK_END, PRIO_HOOK_NONE).unwrap();

    loop {
        // Wait for MPV events indefinitely
        match mpv.wait_event(-1.) {
            Event::StartFile(_) => {
                log::trace!("Received start-file event");
                mute_segment = None;
                actions.start(mpv.get_property::<String>(NAME_PROP_PATH).unwrap());
            }
            Event::FileLoaded => {
                log::trace!("Received file-loaded event");
                if let Some(c) = actions.get_video_category() {
                    mpv.osd_message(
                        format!("This entire video is labeled as '{}' and is too tightly integrated to be able to separate", c),
                        Duration::from_secs(10)
                    ).unwrap();
                }
            }
            Event::PropertyChange(REPL_PROP_TIME, data) => {
                log::trace!("Received {} on reply {}", data.name(), REPL_PROP_TIME);
                if let Some(time_pos) = data.data::<f64>() {
                    if let Some(s) = actions.get_skip_segment(time_pos) {
                        skip(&mpv, s); // Skip segments are priority
                    } else if let Some(s) = actions.get_mute_segment(time_pos) {
                        mute(&mpv, s.clone(), &mute_segment, &mut mute_sponsorblock);
                        mute_segment = Some(s);
                    } else {
                        unmute(&mpv, &mute_segment, &mut mute_sponsorblock);
                        mute_segment = None;
                    }
                }
            }
            Event::PropertyChange(REPL_PROP_MUTE, data) => {
                log::trace!("Received {} on reply {}", data.name(), REPL_PROP_MUTE);
                if let Some(mute) = data.data::<String>() {
                    user_mute(mute, &mut mute_sponsorblock);
                }
            }
            Event::EndFile => {
                log::trace!("Received end-file event");
                unmute(&mpv, &mute_segment, &mut mute_sponsorblock);
            }
            Event::Hook(REPL_HOOK_END, data) => {
                log::trace!("Received {}", data.name());
                actions.join(); // Blocking action, so we use a hook
                mpv.hook_continue(data.id()).unwrap();
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
