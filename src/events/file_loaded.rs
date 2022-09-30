use crate::config::Config;
use crate::mpv::*;
use crate::sponsorblock::segment::{Segment, Segments};
use crate::YT_REPLY_USERDATA;

use std::ffi::{CStr, CString};
use std::os::raw::c_void;

use regex::Regex;

fn get_youtube_id(path: &CStr) -> Option<String> {
    let path = path.to_str().ok()?;

    // I don't uderstand this crap but it's working
    let regexes = [
        Regex::new(r"https?://youtu%.be/([A-Za-z0-9-_]+).*").unwrap(),
        Regex::new(r"https?://w?w?w?%.?youtube%.com/v/([A-Za-z0-9-_]+).*").unwrap(),
        Regex::new(r"/watch.*[?&]v=([A-Za-z0-9-_]+).*").unwrap(),
        Regex::new(r"/embed/([A-Za-z0-9-_]+).*").unwrap(),
        Regex::new(r".*\[([A-Za-z0-9-_]+)\]\.webm").unwrap(),
    ];

    regexes
        .into_iter()
        .filter_map(|r| r.captures(&path))
        .filter_map(|c| c.get(1).map(|m| m.as_str().to_string()))
        .next()
}

pub unsafe fn event(handle: *mut Handle, config: &Config) -> Option<Segments> {
    let property_path = CString::new("path").unwrap();
    let property_time = CString::new("time-pos").unwrap();

    let c_path = mpv_get_property_string(handle, property_path.as_ptr());
    let path = CStr::from_ptr(c_path);

    let yt_id = get_youtube_id(path);

    let segments: Option<Segments> = if let Some(id) = yt_id {
        log::debug!("YouTube ID detected: {}.", id);
        mpv_observe_property(
            handle,
            YT_REPLY_USERDATA,
            property_time.as_ptr(),
            FORMAT_DOUBLE,
        );
        if config.privacy_api {
            Segment::get_segments_with_privacy(config, id)
        } else {
            Segment::get_segments(config, id)
        }
    } else {
        mpv_unobserve_property(handle, YT_REPLY_USERDATA);
        None
    };

    mpv_free(c_path as *mut c_void);

    segments
}
