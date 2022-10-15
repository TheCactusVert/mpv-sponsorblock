mod actions;
mod config;
mod mpv;
mod sponsorblock;
mod utils;

use crate::config::Config;
use crate::mpv::{Event, Format, Handle, RawHandle};
use crate::sponsorblock::segment::Segments;

pub const REPLY_TIME_CHANGE: u64 = 1;

#[no_mangle]
pub extern "C" fn mpv_open_cplugin(handle: RawHandle) -> std::os::raw::c_int {
    env_logger::init();

    let mpv_handle = Handle::from_ptr(handle);

    log::debug!(
        "Starting plugin SponsorBlock ({})!",
        mpv_handle.client_name()
    );

    let config: Config = Config::get();
    let mut segments: Option<Segments> = None;

    if let Err(e) = mpv_handle.observe_property(REPLY_TIME_CHANGE, "time-pos", Format::DOUBLE) {
        log::error!("Failed to observe time position property: {}", e);
        return -1;
    }

    loop {
        match mpv_handle.wait_event(-1.0) {
            Ok((_, Event::Shutdown)) => {
                return 0;
            }
            Ok((_, Event::StartFile(_mpv_event))) => {
                segments = actions::load_segments(&mpv_handle, &config);
            }
            Ok((_, Event::EndFile)) => {
                segments = None;
            }
            Ok((REPLY_TIME_CHANGE, Event::PropertyChange(mpv_event))) => {
                actions::change_time(&mpv_handle, mpv_event, &segments);
            }
            Ok((_, Event::None)) => {
                // Do nothing
            }
            Ok((reply, event)) => {
                log::trace!("Ignoring {} event for reply {}", event, reply)
            }
            Err(e) => {
                log::error!("Asynchronous call failed: {}", e)
            }
        }
    }
}
