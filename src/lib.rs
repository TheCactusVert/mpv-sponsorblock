mod actions;
mod config;
mod mpv;
mod sponsorblock;
mod utils;

use crate::actions::Actions;
use crate::config::Config;
use crate::mpv::{Event, Format, Handle, RawHandle};

use env_logger::Env;

static NAME_PROP_PATH: &str = "path";
static NAME_PROP_TIME: &str = "time-pos";
static NAME_HOOK_LOAD: &str = "on_load";

const REPLY_NONE_NONE: u64 = 0;
const REPLY_PROP_TIME: u64 = 1;
const REPLY_HOOK_LOAD: u64 = 2;

const PRIO_HOOK_DEFAULT: i32 = 0;

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
        .observe_property(REPLY_PROP_TIME, NAME_PROP_TIME, f64::get_format())
        .unwrap();

    // Add hook on file load
    mpv_handle
        .hook_add(REPLY_HOOK_LOAD, NAME_HOOK_LOAD, PRIO_HOOK_DEFAULT)
        .unwrap();

    loop {
        // Wait for MPV events indefinitely
        match mpv_handle.wait_event(-1.) {
            (REPLY_NONE_NONE, Ok(Event::Shutdown)) => {
                log::trace!("Received shutdown event on reply {}.", REPLY_NONE_NONE);
                // End plugin
                return 0;
            }
            (REPLY_NONE_NONE, Ok(Event::EndFile)) => {
                log::trace!("Received end file event on reply {}.", REPLY_NONE_NONE);
                // Clean segments
                actions.drop_segments();
            }
            (REPLY_NONE_NONE, Ok(Event::FileLoaded)) => {
                log::trace!("File loaded event on reply {}.", REPLY_NONE_NONE);
            }
            (REPLY_PROP_TIME, Ok(Event::PropertyChange(event))) => {
                log::trace!("Received {} on reply {}.", event.get_name(), REPLY_PROP_TIME);
                // Get new time posistion
                if let Some(ref s) = actions.skip_segments(event) {
                    log::info!("Skipping segment [{}] to {}.", s.category, s.segment[1]);
                    // Skip segments
                    mpv_handle.set_property(NAME_PROP_TIME, s.segment[1]).unwrap();
                }
            }
            (REPLY_HOOK_LOAD, Ok(Event::Hook(event))) => {
                log::trace!("Received {} on reply {}.", event.get_name(), REPLY_HOOK_LOAD);
                // Get video path
                let path = mpv_handle.get_property_string(NAME_PROP_PATH).unwrap();
                // Blocking operation
                // Non blocking operation might be better, but risky on short videos ?!
                actions.load_segments(&path, &config);
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
