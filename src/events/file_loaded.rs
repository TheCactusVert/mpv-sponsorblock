use crate::YT_REPLY_USERDATA;
use crate::mpv::*;
use crate::sponsorblock::segment::{SkipSegments};

use std::ffi::{CStr, CString};
use std::os::raw::{c_void};

use curl::easy::Easy;
use regex::Regex;

fn get_youtube_id(path: &CStr) -> Option<String> {
    let regexes = [
        Regex::new(r"https?://youtu%.be/([A-Za-z0-9-_]+).*").unwrap(),
        Regex::new(r"https?://w?w?w?%.?youtube%.com/v/([A-Za-z0-9-_]+).*").unwrap(),
        Regex::new(r"/watch.*[?&]v=([A-Za-z0-9-_]+).*").unwrap(),
        Regex::new(r"/embed/([A-Za-z0-9-_]+).*").unwrap(),
    ];

    let path = path.to_str().ok()?;

    for regex in regexes.iter() {
        if let Some(c) = regex.captures(path) {
            return c.get(1).map(|m| m.as_str().to_string());
        }
    }

    None
}

fn get_sponsorblock_skip_segments(id: String) -> Option<SkipSegments> {
    let mut buf = Vec::new();
    let mut handle = Easy::new();
    handle.url(&format!("https://sponsor.ajay.app/api/skipSegments?videoID={}&category=sponsor&category=selfpromo", id)).ok()?;
    {
        let mut transfer = handle.transfer();
        transfer.write_function(|data| {
            buf.extend_from_slice(data);
            Ok(data.len())
        }).ok()?;
        transfer.perform().ok()?;
    }
    
    // Parse the string of data into SkipSegments.
    serde_json::from_slice(&buf).ok()
}

pub unsafe fn event(handle: *mut mpv_handle) -> Option<SkipSegments> {
    let property_path = CString::new("path").unwrap();
    let property_time = CString::new("time-pos").unwrap();

    let c_path = mpv_get_property_string(handle, property_path.as_ptr());
    let path = CStr::from_ptr(c_path);

    let yt_id = get_youtube_id(path);

    let skip_segments: Option<SkipSegments> = if let Some(id) = yt_id {
        log::info!("YouTube ID detected: {}.", id);
        mpv_observe_property(handle, YT_REPLY_USERDATA, property_time.as_ptr(), MPV_FORMAT_DOUBLE);
        get_sponsorblock_skip_segments(id)
    } else {
        mpv_unobserve_property(handle, YT_REPLY_USERDATA);
        None
    };

    mpv_free(c_path as *mut c_void);
    
    skip_segments
}
