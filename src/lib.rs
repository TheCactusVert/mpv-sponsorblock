mod events;
mod mpv;

use crate::mpv::*;
use crate::events::*;

use std::ffi::CStr;
use std::os::raw::{c_int};

pub const YOUTUBE_REPLY_USERDATA: u64 = 1;

pub type Segments = Vec<sponsor_block::Segment>;

#[no_mangle]
pub unsafe extern "C" fn mpv_open_cplugin(handle: *mut mpv_handle) -> c_int {
    env_logger::init();

    log::info!(
        "Starting plugin SponsorBlock ({:?})!",
        CStr::from_ptr(mpv_client_name(handle))
    );
    
    let mut segments: Option<Segments> = None;

    loop {
        let event: *mut mpv_event = mpv_wait_event(handle, -1.0);

        log::debug!("Event received: {}", (*event).event_id);
        
        let event_id = (*event).event_id;
        
        if event_id == MPV_EVENT_SHUTDOWN {
            return 0;
        } else if event_id == MPV_EVENT_FILE_LOADED {
            segments = file_loaded::event(handle);
        } else if event_id == MPV_EVENT_PROPERTY_CHANGE {
            property_changed::event(handle, (*event).reply_userdata);
        }
    }
}
