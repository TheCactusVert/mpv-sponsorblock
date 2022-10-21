mod actions;
mod config;
mod mpv;
mod sponsorblock;
mod utils;

use crate::actions::Actions;
use crate::config::Config;
use crate::mpv::{Event, Handle, RawFormat, RawHandle, ReplyUser};

use env_logger::Env;

const REPLY_NONE_NONE: ReplyUser = 0;
const REPLY_PROP_TIME: ReplyUser = 1;
const REPLY_HOOK_LOAD: ReplyUser = 2;

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
    let mut actions = Actions::new();

    // Subscribe to property time-pos
    if let Err(e) = mpv_handle.observe_property(REPLY_PROP_TIME, "time-pos", RawFormat::DOUBLE) {
        log::error!("Failed to observe time position property: {}.", e);
        return -1;
    }

    // Add hook on file load
    if let Err(e) = mpv_handle.hook_add(REPLY_HOOK_LOAD, "on_load", 1) {
        log::error!("Failed to add on load hook: {}.", e);
        return -1;
    }

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
                log::trace!(
                    "Received property change [{}] event on reply {}.",
                    event.get_name(),
                    REPLY_PROP_TIME
                );
                // Try to skip segments
                actions.skip_segments(&mpv_handle, event);
            }
            (REPLY_HOOK_LOAD, Ok(Event::Hook(event))) => {
                log::trace!(
                    "Received hook [{}] event on reply {}.",
                    event.get_name(),
                    REPLY_HOOK_LOAD
                );
                // Blocking operation
                // Non blocking operation might be better, but risky on short videos ?!
                actions.load_segments(&mpv_handle, &config);
                // Unblock MPV and continue
                if let Err(e) = mpv_handle.hook_continue(event.get_id()) {
                    log::error!("Failed to continue hook: {}.", e);
                    return -1;
                }
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
