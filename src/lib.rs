mod config;
mod events;
mod mpv;
mod sponsorblock;

use crate::config::Config;
use crate::events::*;
use crate::mpv::*;
use crate::sponsorblock::segment::Segments;

use std::ffi::CStr;
use std::os::raw::{c_char, c_int};

pub const YT_REPLY_USERDATA: u64 = 1;

pub const PROPERTY_TIME: &'static [u8] = b"time-pos\0";

unsafe fn observe_time(handle: *mut Handle) -> c_int {
    mpv_observe_property(
        handle,
        YT_REPLY_USERDATA,
        PROPERTY_TIME.as_ptr() as *const c_char,
        FORMAT_DOUBLE,
    )
}

#[no_mangle]
pub unsafe extern "C" fn mpv_open_cplugin(handle: *mut Handle) -> c_int {
    env_logger::init();

    log::info!(
        "Starting plugin SponsorBlock ({:?})!",
        CStr::from_ptr(mpv_client_name(handle))
    );

    let config: Config = Config::get();

    let mut segments: Option<Segments> = None;

    observe_time(handle);

    loop {
        let event: *mut Event = mpv_wait_event(handle, -1.0);

        let event_id = (*event).event_id;

        if event_id == EVENT_SHUTDOWN {
            return 0;
        } else if event_id == EVENT_START_FILE {
            segments = start_file::event(handle, &config);
        } else if event_id == EVENT_END_FILE {
            segments = None;
        } else if event_id == EVENT_PROPERTY_CHANGE {
            property_change::event(handle, event, &segments);
        }
    }
}
