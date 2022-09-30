use crate::mpv::*;
use crate::sponsorblock::segment::Segments;
use crate::YT_REPLY_USERDATA;

use std::ffi::CString;
use std::os::raw::{c_double, c_void};

unsafe fn change_video_time(handle: *mut Handle, event: *mut Event, segments: &Option<Segments>) {
    if let Some(segments) = segments {
        let property = (*event).data as *mut EventProperty;
        let time_pos = (*property).data as *mut c_double;

        if time_pos.is_null() {
            return;
        }

        let old_time_pos: c_double = *time_pos;
        let mut new_time_pos: c_double = old_time_pos;

        for segment in segments.iter().filter(|s| s.action.as_str() == "skip") {
            if new_time_pos >= segment.segment[0] && new_time_pos < segment.segment[1] {
                log::info!(
                    "Skipping segment [{}] from {} to {}.",
                    segment.category,
                    segment.segment[0],
                    segment.segment[1]
                );

                new_time_pos = segment.segment[1];
            }
        }

        if old_time_pos != new_time_pos {
            let property_time = CString::new("time-pos").unwrap();
            let data: *mut c_void = &mut new_time_pos as *mut _ as *mut c_void;
            mpv_set_property(handle, property_time.as_ptr(), FORMAT_DOUBLE, data);
        }

        /*for segment in segments {
            let t1 = segment.segment[0];
            let mut t2 = segment.segment[1];
            match segment.action.as_str() {
                "skip" if time_pos >= t1 && time_pos < t2 => {
                    let property_time = CString::new("time-pos").unwrap();
                    let data: *mut c_void = &mut t2 as *mut _ as *mut c_void;
                    log::info!(
                        "Skipping segment [{}] from {} to {}.",
                        segment.category,
                        t1,
                        t2
                    );

                }
                "mute" => { } // Should mute a segment, seems more complicate than anything
                "poi" => { } // Keybinding should send to point of interest but impossible in C plugins
                "full" => { } // Should skip the whole video
                _ => {}
            }
        }*/
    }
}

pub unsafe fn event(handle: *mut Handle, event: *mut Event, segments: &Option<Segments>) {
    match (*event).reply_userdata {
        YT_REPLY_USERDATA => change_video_time(handle, event, segments),
        _ => {}
    }
}
