#![feature(drain_filter)]

mod actions;
mod config;
mod sponsorblock;
mod utils;

use actions::{Actions, Volume, MUTE_VOLUME};
use config::Config;
use mpv_client::{Event, Handle, RawHandle};

use std::time::Duration;

use env_logger::Env;

static NAME_PROP_PATH: &str = "path";
static NAME_PROP_TIME: &str = "time-pos";
static NAME_PROP_VOLU: &str = "volume";
static NAME_HOOK_LOAD: &str = "on_load";

const REPL_NONE_NONE: u64 = 0;
const REPL_PROP_TIME: u64 = 1;
const REPL_PROP_VOLU: u64 = 2;
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
        .observe_property::<f64>(REPL_PROP_TIME, NAME_PROP_TIME)
        .unwrap();

    // Subscribe to property volume (is mute deprecated ?)
    mpv_handle
        .observe_property::<f64>(REPL_PROP_VOLU, NAME_PROP_VOLU)
        .unwrap();

    // Add hook on file load
    mpv_handle
        .hook_add(REPL_HOOK_LOAD, NAME_HOOK_LOAD, PRIO_HOOK_NONE)
        .unwrap();

    loop {
        // Wait for MPV events indefinitely
        match mpv_handle.wait_event(-1.) {
            (REPL_HOOK_LOAD, Ok(Event::Hook(data))) => {
                log::trace!("Received {} on reply {}.", data.name(), REPL_HOOK_LOAD);
                // Get video path
                let path: String = mpv_handle.get_property(NAME_PROP_PATH).unwrap();
                // Blocking operation
                // Non blocking operation might be better, but risky on short videos ?!
                actions.load_chapters(&path, &config);
                actions.reset_muted();
                // Unblock MPV and continue
                mpv_handle.hook_continue(data.id()).unwrap();
            }
            (REPL_NONE_NONE, Ok(Event::FileLoaded)) => {
                log::trace!("Received file loaded event on reply {}.", REPL_NONE_NONE);
                // Display the category of the video at start
                if let Some(c) = actions.get_video_category() {
                    let message = format!(
                        "This entire video is labeled as {} and is too tightly integrated to be able to separate.",
                        c
                    );
                    mpv_handle.osd_message(message, Duration::from_secs(10)).unwrap();
                }
            }
            (REPL_PROP_TIME, Ok(Event::PropertyChange(data))) => {
                log::trace!("Received {} on reply {}.", data.name(), REPL_PROP_TIME);
                // Get new time position
                if let Some(time_pos) = data.data::<f64>() {
                    if let Some(ref s) = actions.get_skip_segment(time_pos) {
                        // Skip segments are priority
                        log::info!("Skipping {}.", s);
                        mpv_handle.set_property(NAME_PROP_TIME, s.segment[1]).unwrap();
                    } else if let Some(ref s) = actions.get_mute_segment(time_pos) {
                        // Mute when no skip segments
                        if Volume::Default == actions.get_volume_source() {
                            log::info!("Muting {}.", s);
                            actions.force_muted();
                            mpv_handle.set_property(NAME_PROP_VOLU, MUTE_VOLUME).unwrap();
                        }
                    } else {
                        // Reset volume when not in mute segment
                        if Volume::Default != actions.get_volume_source() {
                            log::info!("Unmuting video.");
                            actions.reset_muted();
                            mpv_handle.set_property(NAME_PROP_VOLU, actions.get_volume()).unwrap();
                        }
                    }
                } else {
                    log::warn!("Received {} without data. Ignoring...", data.name());
                }
            }
            (REPL_PROP_VOLU, Ok(Event::PropertyChange(data))) => {
                log::trace!("Received {} on reply {}.", data.name(), REPL_PROP_VOLU);
                // Get the new volume
                if let Some(volume) = data.data::<f64>() {
                    // Save the volume
                    actions.set_volume(volume);
                } else {
                    // Should be impossible
                    log::warn!("Received {} without data. Ignoring...", data.name());
                }
            }
            (REPL_NONE_NONE, Ok(Event::EndFile)) => {
                log::trace!("Received end file event on reply {}.", REPL_NONE_NONE);
                // Reset volume when file end to avoid starting next file with volume at 0
                if Volume::Default != actions.get_volume_source() {
                    log::info!("Unmuting video.");
                    actions.reset_muted();
                    mpv_handle.set_property(NAME_PROP_VOLU, actions.get_volume()).unwrap();
                }
            }
            (REPL_NONE_NONE, Ok(Event::Shutdown)) => {
                log::trace!("Received shutdown event on reply {}.", REPL_NONE_NONE);
                // End plugin
                return 0;
            }
            (reply, Ok(data)) => {
                log::trace!("Ignoring {} event on reply {}.", data, reply);
            }
            (reply, Err(e)) => {
                log::error!("Asynchronous call failed: {} on reply {}.", e, reply);
            }
        }
    }
}
