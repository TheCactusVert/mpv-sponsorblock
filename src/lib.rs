mod config;
mod events;
mod mpv;
mod sponsorblock;

use crate::config::Config;
use crate::events::*;
use crate::mpv::*;
use crate::sponsorblock::segment::Segments;

use std::ffi::{CStr, CString};
use std::os::raw::c_int;

pub const YT_REPLY_USERDATA: u64 = 1;

unsafe fn observe_time(handle: *mut Handle) -> c_int {
    let property_time = CString::new("time-pos").unwrap();
    mpv_observe_property(
        handle,
        YT_REPLY_USERDATA,
        property_time.as_ptr(),
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
        } else if event_id == EVENT_PROPERTY_CHANGE {
            property_change::event(handle, event, &segments);
        }
    }
}
