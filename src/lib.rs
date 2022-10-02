mod config;
mod events;
mod mpv;
mod sponsorblock;

use crate::config::Config;
use crate::events::*;
use crate::mpv::{MpvEventID, MpvFormat, MpvHandle, MpvRawHandle};
use crate::sponsorblock::segment::Segments;

pub const WATCHER_TIME: u64 = 1;

#[no_mangle]
pub extern "C" fn mpv_open_cplugin(handle: MpvRawHandle) -> std::os::raw::c_int {
    env_logger::init();

    let mpv_handle = MpvHandle::from_ptr(handle);

    log::info!(
        "Starting plugin SponsorBlock ({})!",
        mpv_handle.client_name().unwrap_or("Unknown".to_string())
    );

    let config: Config = Config::get();
    let mut segments: Option<Segments> = None;

    if let Err(e) = mpv_handle.observe_property(WATCHER_TIME, "time-pos", MpvFormat::Double) {
        log::error!("{}", e);
        return -1;
    }

    loop {
        let mpv_event = mpv_handle.wait_event(-1.0);
        let mpv_event_id = mpv_event.get_event_id();

        if mpv_event_id == MpvEventID::Shutdown {
            return 0;
        } else if mpv_event_id == MpvEventID::StartFile {
            segments = start_file::event(&mpv_handle, &config);
        } else if mpv_event_id == MpvEventID::EndFile {
            segments = None;
        } else if mpv_event_id == MpvEventID::PropertyChange {
            property_change::event(&mpv_handle, mpv_event, &segments);
        }
    }
}
