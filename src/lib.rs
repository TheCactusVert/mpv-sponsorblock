#![feature(drain_filter)]

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
const REPL_HOOK_LOAD: u64 = 3;

const PRIO_HOOK_NONE: i32 = 0;

fn skip(mpv_handle: &Handle, s: &Segment) {
    log::info!("Skipping {}.", s);
    mpv_handle.set_property(NAME_PROP_TIME, s.segment[1]).unwrap();
}

fn mute(mpv_handle: &Handle, s: &Segment, entering_segment: bool, mute_sponsorblock: &mut bool) {
    // Working only if entering a new segment and not already mutted by plugin
    if !entering_segment || *mute_sponsorblock {
        return;
    }

    if mpv_handle.get_property::<String>(NAME_PROP_MUTE).unwrap() != "yes" {
        log::info!("Mutting {}.", s);
        mpv_handle.set_property(NAME_PROP_MUTE, "yes".to_string()).unwrap();
        *mute_sponsorblock = true;
    } else {
        log::trace!("Muttable segment found but MPV was mutted by user before. Ignoring...");
    }
}

fn unmute(mpv_handle: &Handle, mute_sponsorblock: &mut bool) {
    if *mute_sponsorblock {
        log::info!("Unmutting.");
        mpv_handle.set_property(NAME_PROP_MUTE, "no".to_string()).unwrap();
        *mute_sponsorblock = false;
    } else {
        log::trace!("Ignoring unmute...");
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
    let mpv_handle = Handle::from_ptr(handle);

    // Show that the plugin has started
    log::debug!("Starting plugin SponsorBlock [{}]!", mpv_handle.client_name());

    // Create actions handler
    let mut actions = Actions::new();

    // Boolean to check if we are currently in a mutted segment
    let mut mute_segment: Option<String> = None;
    let mut mute_sponsorblock: bool = false;

    // Subscribe to property time-pos
    mpv_handle
        .observe_property::<f64>(REPL_PROP_TIME, NAME_PROP_TIME)
        .unwrap();

    // Add hook on file load
    mpv_handle
        .hook_add(REPL_HOOK_LOAD, NAME_HOOK_LOAD, PRIO_HOOK_NONE)
        .unwrap();

    loop {
        // Wait for MPV events indefinitely
        match mpv_handle.wait_event(-1.) {
            Event::Hook(REPL_HOOK_LOAD, data) => {
                log::trace!("Received {}.", data.name());
                mute_segment = None;
                // Blocking operation
                actions.load_chapters(mpv_handle.get_property::<String>(NAME_PROP_PATH).unwrap());
                // Unblock MPV and continue
                mpv_handle.hook_continue(data.id()).unwrap();
            }
            Event::FileLoaded => {
                log::trace!("Received file-loaded event.");
                // Display the category of the video at start
                if let Some(c) = actions.get_video_category() {
                    mpv_handle.osd_message(
                        format!("This entire video is labeled as '{}' and is too tightly integrated to be able to separate.", c),
                        Duration::from_secs(10)
                    ).unwrap();
                }
            }
            Event::PropertyChange(REPL_PROP_TIME, data) => {
                log::trace!("Received {} on reply {}.", data.name(), REPL_PROP_TIME);
                // Get new time position
                if let Some(time_pos) = data.data::<f64>() {
                    if let Some(ref s) = actions.get_skip_segment(time_pos) {
                        skip(&mpv_handle, s); // Skip segments are priority
                    } else if let Some(ref s) = actions.get_mute_segment(time_pos) {
                        let uuid = Some(s.uuid.clone());
                        mute(&mpv_handle, s, mute_segment != uuid, &mut mute_sponsorblock);
                        mute_segment = uuid;
                    } else {
                        unmute(&mpv_handle, &mut mute_sponsorblock);
                        mute_segment = None;
                    }
                }
            }
            Event::EndFile => {
                log::trace!("Received end-file event.");
                unmute(&mpv_handle, &mut mute_sponsorblock);
                mute_segment = None;
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
