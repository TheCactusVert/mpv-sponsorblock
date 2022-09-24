use crate::{YOUTUBE_REPLY_USERDATA, Segments};
use crate::mpv::*;

use std::ffi::{CStr, CString};
use std::os::raw::{c_void};

use regex::Regex;
use sponsor_block::{AcceptedActions, AcceptedCategories, Client};

// This should be random, treated like a password, and stored across sessions
const USER_ID: &str = include_str!("../../user.in");

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

pub unsafe fn event(handle: *mut mpv_handle) -> Option<Segments> {
    let property_path = CString::new("path").unwrap();
    let property_time = CString::new("time-pos").unwrap();

    let c_path = mpv_get_property_string(handle, property_path.as_ptr());
    let path = CStr::from_ptr(c_path);

    let youtube_id = get_youtube_id(path);
    log::info!("YouTube ID: {:?}!", youtube_id);

    let segments: Option<Segments> = if let Some(id) = youtube_id {
        mpv_observe_property(
            handle,
            YOUTUBE_REPLY_USERDATA,
            property_time.as_ptr(),
            MPV_FORMAT_DOUBLE,
        );

        let client = Client::new(USER_ID);
        client.fetch_segments(&id, AcceptedCategories::all(), AcceptedActions::all()).ok()
    } else {
        mpv_unobserve_property(handle, YOUTUBE_REPLY_USERDATA);
        None
    };

    mpv_free(c_path as *mut c_void);
    
    segments
}
