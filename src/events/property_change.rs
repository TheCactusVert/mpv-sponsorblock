use crate::YT_REPLY_USERDATA;
use crate::mpv::*;
use crate::sponsorblock::segment::{SkipSegments};

use std::ffi::CString;
use std::os::raw::{c_void, c_double};

unsafe fn change_video_time(handle: *mut mpv_handle, event: *mut mpv_event, skip_segments: &Option<SkipSegments>) {
    if let Some(skip_segments) = skip_segments {
        let property = (*event).data as *mut mpv_event_property;
        let time_pos = (*property).data as *mut c_double;
        
        if time_pos.is_null() {
            return;
        }
        
        let time_pos: c_double = *time_pos;
        
        for skip_segment in skip_segments {
            let t1 = skip_segment.segment[0];
            let mut t2 = skip_segment.segment[1];
            match skip_segment.action.as_str() {
                "skip" if time_pos >= t1 && time_pos < t2 => {
                    let property_time = CString::new("time-pos").unwrap();
                    let data: *mut c_void = &mut t2 as *mut _ as *mut c_void;
                    log::info!("Skipping segment [{}] from {} to {}.", skip_segment.category, t1, t2);
                    mpv_set_property(handle, property_time.as_ptr(), MPV_FORMAT_DOUBLE, data);
                },
                _ => {},
            }
        }
    }
}

pub unsafe fn event(handle: *mut mpv_handle, event: *mut mpv_event, skip_segments: &Option<SkipSegments>) {
    match (*event).reply_userdata {
        YT_REPLY_USERDATA => change_video_time(handle, event, skip_segments),
        _ => {}
    }
}
