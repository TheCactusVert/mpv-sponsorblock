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
static NAME_HOOK_LOAD: &str = "on_load";

const REPL_PROP_TIME: u64 = 1;
const REPL_PROP_MUTE: u64 = 2;
const REPL_HOOK_LOAD: u64 = 3;

const PRIO_HOOK_NONE: i32 = 0;

fn skip(mpv: &Handle, working_segment: &Segment) {
    log::info!("Skipping {}.", working_segment);
    mpv.set_property(NAME_PROP_TIME, working_segment.segment[1]).unwrap();
}

fn mute(mpv: &Handle, working_segment: &Segment, current_segment: Option<&Segment>, mute_sponsorblock: &mut bool) {
    // Working only if entering a new segment
    if current_segment.is_some_and(|segment| segment.uuid == working_segment.uuid) {
        return;
    }

    // If already muted by the plugin do it again just for the log
    if *mute_sponsorblock || mpv.get_property::<String>(NAME_PROP_MUTE).unwrap() != "yes" {
        log::info!("Mutting {}.", working_segment);
        mpv.set_property(NAME_PROP_MUTE, "yes".to_string()).unwrap();
        mpv.osd_message(format!("Mutting {}.", working_segment), Duration::from_secs(8))
            .unwrap();
        *mute_sponsorblock = true;
    } else {
        log::trace!("Muttable segment found but MPV was mutted by user before. Ignoring...");
        *mute_sponsorblock = false;
    }
}

fn unmute(mpv: &Handle, mute_sponsorblock: &mut bool) {
    if *mute_sponsorblock {
        log::info!("Unmutting.");
        mpv.set_property(NAME_PROP_MUTE, "no".to_string()).unwrap();
        *mute_sponsorblock = false;
    } else {
        log::trace!("Ignoring unmute...");
        *mute_sponsorblock = false;
    }
}

fn user_mute(value: String, mute_sponsorblock: &mut bool) {
    match (value.as_str(), *mute_sponsorblock) {
        ("no", true) => *mute_sponsorblock = false,
        _ => {}
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
    let mut actions = Actions::new();

    // Boolean to check if we are currently in a mutted segment
    let mut mute_segment: Option<&Segment> = None;
    let mut mute_sponsorblock: bool = false;

    // Subscribe to property time-pos
    mpv.observe_property::<f64>(REPL_PROP_TIME, NAME_PROP_TIME).unwrap();

    // Subscribe to property mute
    mpv.observe_property::<String>(REPL_PROP_MUTE, NAME_PROP_MUTE).unwrap();

    // Add hook on file load
    mpv.hook_add(REPL_HOOK_LOAD, NAME_HOOK_LOAD, PRIO_HOOK_NONE).unwrap();

    loop {
        // Wait for MPV events indefinitely
        match mpv.wait_event(-1.) {
            Event::Hook(REPL_HOOK_LOAD, data) => {
                log::trace!("Received {}.", data.name());
                mute_segment = None;
                actions.load_chapters(mpv.get_property::<String>(NAME_PROP_PATH).unwrap());
                mpv.hook_continue(data.id()).unwrap();
            }
            Event::FileLoaded => {
                log::trace!("Received file-loaded event.");
                if let Some(c) = actions.get_video_category() {
                    mpv.osd_message(
                        format!("This entire video is labeled as '{}' and is too tightly integrated to be able to separate.", c),
                        Duration::from_secs(10)
                    ).unwrap();
                }
            }
            Event::PropertyChange(REPL_PROP_TIME, data) => {
                log::trace!("Received {} on reply {}.", data.name(), REPL_PROP_TIME);
                if let Some(time_pos) = data.data::<f64>() {
                    if let Some(ref s) = actions.get_skip_segment(time_pos) {
                        skip(&mpv, s); // Skip segments are priority
                    } else if let Some(ref s) = actions.get_mute_segment(time_pos) {
                        mute(&mpv, s, mute_segment, &mut mute_sponsorblock);
                        mute_segment = Some(s);
                    } else {
                        unmute(&mpv, &mut mute_sponsorblock);
                        mute_segment = None;
                    }
                }
            }
            Event::PropertyChange(REPL_PROP_MUTE, data) => {
                log::trace!("Received {} on reply {}.", data.name(), REPL_PROP_MUTE);
                if let Some(mute) = data.data::<String>() {
                    user_mute(mute, &mut mute_sponsorblock);
                }
            }
            Event::EndFile => {
                log::trace!("Received end-file event.");
                unmute(&mpv, &mut mute_sponsorblock);
            }
            Event::Shutdown => {
                log::trace!("Received shutdown event.");
                return 0;
            }
            event => {
                log::trace!("Ignoring {} event.", event);
            }
        }
    }
}
