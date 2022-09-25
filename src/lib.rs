mod mpv;
mod events;
mod sponsorblock;

use crate::mpv::*;
use crate::events::*;
use crate::sponsorblock::segment::{SkipSegments};

use std::ffi::CStr;
use std::os::raw::{c_int};

pub const YT_REPLY_USERDATA: u64 = 1;

#[no_mangle]
pub unsafe extern "C" fn mpv_open_cplugin(handle: *mut mpv_handle) -> c_int {
    env_logger::init();

    log::info!(
        "Starting plugin SponsorBlock ({:?})!",
        CStr::from_ptr(mpv_client_name(handle))
    );
    
    let mut segments: Option<SkipSegments> = None;

    loop {
        let event: *mut mpv_event = mpv_wait_event(handle, -1.0);

        let event_id = (*event).event_id;

        if event_id == MPV_EVENT_SHUTDOWN {
            return 0;
        } else if event_id == MPV_EVENT_FILE_LOADED {
            segments = file_loaded::event(handle);
        } else if event_id == MPV_EVENT_PROPERTY_CHANGE {
            property_change::event(handle, event, &segments);
        }
    }
}
