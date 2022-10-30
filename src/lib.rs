#![feature(drain_filter)]

mod actions;
mod config;
mod mpv;
mod sponsorblock;
mod utils;

use actions::Actions;
use config::Config;
use mpv::{Event, Format, Handle, RawHandle};

use env_logger::Env;

static NAME_PROP_PATH: &str = "path";
static NAME_PROP_TIME: &str = "time-pos";
static NAME_PROP_MUTE: &str = "mute";
static NAME_HOOK_LOAD: &str = "on_load";

const REPL_NONE_NONE: u64 = 0;
const REPL_PROP_TIME: u64 = 1;
const REPL_PROP_MUTE: u64 = 2;
const REPL_HOOK_LOAD: u64 = 3;

const PRIO_HOOK_NONE: i32 = 0;

// MPV entry point
#[no_mangle]
extern "C" fn mpv_open_cplugin(handle: RawHandle) -> std::os::raw::c_int {
    // TODO Maybe use MPV logger ?
    let env = Env::new().filter("MPV_SB_LOG").write_style("MPV_SB_LOG_STYLE");
    env_logger::init_from_env(env);

    // Wrap handle
    let mpv_handle = Handle::from_ptr(handle);

    // Show that the plugin has started
    log::debug!("Starting plugin SponsorBlock [{}]!", mpv_handle.client_name());

    // Load config file
    let config = Config::get();

    // Create actions handler
    let mut actions = Actions::default();

    // Subscribe to property time-pos
    mpv_handle
        .observe_property(REPL_PROP_TIME, NAME_PROP_TIME, f64::get_format())
        .unwrap();

    // Subscribe to property volume
    mpv_handle
        .observe_property(REPL_PROP_MUTE, NAME_PROP_MUTE, String::get_format())
        .unwrap();

    // Add hook on file load
    mpv_handle
        .hook_add(REPL_HOOK_LOAD, NAME_HOOK_LOAD, PRIO_HOOK_NONE)
        .unwrap();

    loop {
        // Wait for MPV events indefinitely
        match mpv_handle.wait_event(-1.) {
            (REPL_NONE_NONE, Ok(Event::Shutdown)) => {
                log::trace!("Received shutdown event on reply {}.", REPL_NONE_NONE);
                // End plugin
                return 0;
            }
            (REPL_NONE_NONE, Ok(Event::FileLoaded)) => {
                log::trace!("Received file loaded event on reply {}.", REPL_NONE_NONE);
                // Display On Screen
                if let Some(c) = actions.get_video_category() {
                    log::info!("Video category: {}", c);
                }
                // Display On Screen
                if let Some(p) = actions.get_video_poi() {
                    log::info!("Video POI at: {} s", p);
                }
            }
            (REPL_PROP_TIME, Ok(Event::PropertyChange(event))) => {
                log::trace!("Received {} on reply {}.", event.get_name(), REPL_PROP_TIME);
                // Get new time position
                if let Some(time_pos) = event.get_data::<f64>() {
                    if let Some(ref s) = actions.get_skip_segment(time_pos) {
                        log::info!("Skipping segment [{}] to {}.", s.category, s.segment[1]);
                        mpv_handle.set_property(NAME_PROP_TIME, s.segment[1]).unwrap();
                    } else if let Some(ref s) = actions.get_mute_segment(time_pos) {
                        log::info!("Muting segment [{}] until {}.", s.category, s.segment[1]);
                        //mpv_handle.set_property().unwrap();
                    } else {
                        //log::info!("Unmuting video.");
                        //mpv_handle.set_property().unwrap();
                    }
                } else {
                    log::warn!("Received {} without data. Ignoring...", event.get_name());
                }
            }
            (REPL_PROP_MUTE, Ok(Event::PropertyChange(event))) => {
                log::trace!("Received {} on reply {}.", event.get_name(), REPL_PROP_MUTE);
            }
            (REPL_HOOK_LOAD, Ok(Event::Hook(event))) => {
                log::trace!("Received {} on reply {}.", event.get_name(), REPL_HOOK_LOAD);
                // Get video path
                let path = mpv_handle.get_property_string(NAME_PROP_PATH).unwrap();
                // Blocking operation
                // Non blocking operation might be better, but risky on short videos ?!
                actions.load_chapters(&path, &config);
                // Unblock MPV and continue
                mpv_handle.hook_continue(event.get_id()).unwrap();
            }
            (reply, Ok(event)) => {
                log::trace!("Ignoring {} event on reply {}.", event, reply);
            }
            (reply, Err(e)) => {
                log::error!("Asynchronous call failed: {} on reply {}.", e, reply);
            }
        }
    }
}
