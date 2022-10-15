mod config;
mod events;
mod mpv;
mod sponsorblock;
mod utils;

use crate::config::Config;
use crate::events::*;
use crate::mpv::{EventID, Format, Handle, RawHandle};
use crate::sponsorblock::segment::Segments;

pub const WATCHER_TIME: u64 = 1;

#[no_mangle]
pub extern "C" fn mpv_open_cplugin(handle: RawHandle) -> std::os::raw::c_int {
    env_logger::init();

    let mpv_handle = Handle::from_ptr(handle);

    log::debug!(
        "Starting plugin SponsorBlock ({})!",
        mpv_handle.client_name().unwrap_or("Unknown".to_string())
    );

    let config: Config = Config::get();
    let mut segments: Option<Segments> = None;

    if let Err(e) = mpv_handle.observe_property(WATCHER_TIME, "time-pos", Format::DOUBLE) {
        log::error!("Failed to observe time position property: {}", e);
        return -1;
    }

    loop {
        match mpv_handle.wait_event(-1.0) {
            EventID::Shutdown => {
                return 0;
            }
            EventID::StartFile => {
                segments = start_file::event(&mpv_handle, &config);
            }
            EventID::EndFile => {
                segments = None;
            }
            EventID::PropertyChange(mpv_reply, mpv_event) => {
                property_change::event(&mpv_handle, mpv_reply, mpv_event, &segments);
            }
            EventID::None => {
                // Do nothing
            }
            event => {
                log::trace!("Ignoring event named: {}", event)
            }
        }
    }
}
