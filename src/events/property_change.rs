use crate::mpv::*;
use crate::sponsorblock::segment::Segments;
use crate::PROPERTY_TIME;
use crate::YT_REPLY_USERDATA;

use std::os::raw::c_double;

fn change_video_time(
    mpv_handle: &MpvHandle,
    mpv_event: MpvEvent,
    segments: &Option<Segments>,
) {
    if let Some(segments) = segments {
        let event_property = mpv_event.get_event_property();
        let old_time_pos: c_double = match event_property.get_data() {
            Some(v) => v,
            None => return,
        };
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
            mpv_handle.set_property(PROPERTY_TIME, MpvFormat::DOUBLE, new_time_pos);
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

pub fn event(mpv_handle: &MpvHandle, mpv_event: MpvEvent, segments: &Option<Segments>) {
    match mpv_event.get_reply_userdata() {
        YT_REPLY_USERDATA => change_video_time(mpv_handle, mpv_event, segments),
        _ => {}
    }
}
