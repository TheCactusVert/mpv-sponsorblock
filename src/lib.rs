mod mpv;
mod config;
mod events;
mod api;

use crate::mpv::*;
use crate::config::Config;
use crate::events::*;
use crate::api::segment::{Segments};

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
    
    let config: Config = Config::get();
    
    let mut segments: Option<Segments> = None;

    loop {
        let event: *mut mpv_event = mpv_wait_event(handle, -1.0);

        let event_id = (*event).event_id;

        if event_id == MPV_EVENT_SHUTDOWN {
            return 0;
        } else if event_id == MPV_EVENT_FILE_LOADED {
            segments = file_loaded::event(handle, &config);
        } else if event_id == MPV_EVENT_PROPERTY_CHANGE {
            property_change::event(handle, event, &segments);
        }
    }
}
