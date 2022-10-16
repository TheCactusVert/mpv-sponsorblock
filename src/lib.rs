mod actions;
mod config;
mod mpv;
mod sponsorblock;
mod utils;

use crate::actions::Actions;
use crate::config::Config;
use crate::mpv::{Event, Format, Handle, RawHandle, ReplyUser};

const REPLY_TIME: ReplyUser = 1;

// MPV entry point
#[no_mangle]
extern "C" fn mpv_open_cplugin(handle: RawHandle) -> std::os::raw::c_int {
    // TODO Maybe use MPV logger ?
    env_logger::init();

    // Wrap handle
    let mpv_handle = Handle::from_ptr(handle);

    // Show that the plugin has started
    log::debug!("Starting plugin SponsorBlock [{}]!", mpv_handle.client_name());

    // Load config file
    let config: Config = Config::get();

    // Create actions handler
    let mut actions: Actions = Actions::new();

    // Subscribe to property time-pos
    if let Err(e) = mpv_handle.observe_property(REPLY_TIME, "time-pos", Format::DOUBLE) {
        log::error!("Failed to observe time position property: {}.", e);
        return -1;
    }

    loop {
        // Wait for MPV events indefinitely
        match mpv_handle.wait_event(-1.0) {
            (_, Ok(Event::Shutdown)) => return 0,
            (_, Ok(Event::StartFile(_event))) => actions.load_segments(&mpv_handle, &config),
            (_, Ok(Event::EndFile)) => actions.drop_segments(),
            (REPLY_TIME, Ok(Event::PropertyChange(event))) => actions.skip_segments(&mpv_handle, event),
            (reply, Ok(event)) => log::trace!("Ignoring {} event on reply {}.", event, reply),
            (reply, Err(e)) => log::error!("Asynchronous call failed: {} on reply {}.", e, reply),
        }
    }
}
