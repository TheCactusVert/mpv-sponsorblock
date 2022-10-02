mod config;
mod events;
mod mpv;
mod sponsorblock;

use crate::config::Config;
use crate::events::*;
use crate::mpv::*;
use crate::sponsorblock::segment::Segments;

use std::os::raw::c_int;

pub const YT_REPLY_USERDATA: u64 = 1;

pub const PROPERTY_TIME: &'static [u8] = b"time-pos\0";

#[no_mangle]
pub unsafe extern "C" fn mpv_open_cplugin(handle: *mut mpv_handle) -> c_int {
    env_logger::init();
    
    let mpv_handle = MpvHandle::new(handle);

    log::info!(
        "Starting plugin SponsorBlock ({})!",
        mpv_handle.client_name().unwrap()
    );

    let config: Config = Config::get();
    let mut segments: Option<Segments> = None;

    mpv_handle.observe_property(YT_REPLY_USERDATA, PROPERTY_TIME, FORMAT_DOUBLE);

    loop {
        let event: *mut Event = mpv_handle.wait_event(-1.0);

        let event_id = (*event).event_id;

        if event_id == EVENT_SHUTDOWN {
            return 0;
        } else if event_id == EVENT_START_FILE {
            segments = start_file::event(&mpv_handle, &config);
        } else if event_id == EVENT_END_FILE {
            segments = None;
        } else if event_id == EVENT_PROPERTY_CHANGE {
            property_change::event(&mpv_handle, event, &segments);
        }
    }
}
