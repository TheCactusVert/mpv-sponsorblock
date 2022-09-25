use crate::{YT_REPLY_USERDATA, Segments};
use crate::mpv::*;

use std::ffi::CString;
use std::os::raw::{c_void, c_double};

use sponsor_block::Action::*;

unsafe fn change_video_time(handle: *mut mpv_handle, event: *mut mpv_event, segments: &Option<Segments>) {
    if let Some(segments) = segments {
        let property = (*event).data as *mut mpv_event_property;
        let time_pos = (*property).data as *mut c_double;
        
        if time_pos.is_null() {
            return;
        }
        
        let time_pos: c_double = *time_pos;
        
        for segment in segments {
            match segment.action {
                Skip(t1, mut t2) =>  if time_pos >= t1 && time_pos < t2 {
                    let property_time = CString::new("time-pos").unwrap();
                    let data: *mut c_void = &mut t2 as *mut _ as *mut c_void;
                    log::info!("Skipping segment from {} to {}.", t1, t2);
                    mpv_set_property(handle, property_time.as_ptr(), MPV_FORMAT_DOUBLE, data);
                },
                _ => {},
            }
        }
    }
}

pub unsafe fn event(handle: *mut mpv_handle, event: *mut mpv_event, segments: &Option<Segments>) {
    match (*event).reply_userdata {
        YT_REPLY_USERDATA => change_video_time(handle, event, segments),
        _ => {}
    }
}
