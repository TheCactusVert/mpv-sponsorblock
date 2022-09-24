use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};

use regex::Regex;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct mpv_handle {
    _unused: [u8; 0],
}

#[allow(non_camel_case_types)]
type mpv_event_id = c_int;

pub const MPV_EVENT_SHUTDOWN: mpv_event_id = 1;
pub const MPV_EVENT_FILE_LOADED: mpv_event_id = 8;
pub const MPV_EVENT_PROPERTY_CHANGE: mpv_event_id = 22;

#[allow(non_camel_case_types)]
pub type mpv_format = c_int;

pub const MPV_FORMAT_DOUBLE: mpv_format = 5;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct mpv_event {
    pub event_id: mpv_event_id,
    pub error: c_int,
    pub reply_userdata: u64,
    pub data: *mut c_void,
}

extern "C" {
    pub fn mpv_wait_event(ctx: *mut mpv_handle, timeout: f64) -> *mut mpv_event;
    pub fn mpv_client_name(ctx: *mut mpv_handle) -> *const c_char;
    pub fn mpv_get_property_string(ctx: *mut mpv_handle, name: *const c_char) -> *mut c_char;
    pub fn mpv_free(data: *mut c_void);
    pub fn mpv_observe_property(
        mpv: *mut mpv_handle,
        reply_userdata: u64,
        name: *const c_char,
        format: mpv_format,
    ) -> c_int;
    pub fn mpv_unobserve_property(mpv: *mut mpv_handle, registered_reply_userdata: u64) -> c_int;
}

pub const YOUTUBE_REPLY_USERDATA: u64 = 1;

fn get_youtube_id(path: &CStr) -> Option<String> {
    let regexes = [
        Regex::new(r"https?://youtu%.be/([A-Za-z0-9-_]+).*").ok()?,
        Regex::new(r"https?://w?w?w?%.?youtube%.com/v/([A-Za-z0-9-_]+).*").ok()?,
        Regex::new(r"/watch.*[?&]v=([A-Za-z0-9-_]+).*").ok()?,
        Regex::new(r"/embed/([A-Za-z0-9-_]+).*").ok()?,
    ];

    let path = path.to_str().ok()?;

    for regex in regexes.iter() {
        if let Some(c) = regex.captures(path) {
            return c.get(1).map(|m| m.as_str().to_string());
        }
    }

    None
}

unsafe fn event_file_loaded(handle: *mut mpv_handle) {
    let property_path = CString::new("path").expect("CString::new failed");
    let property_time = CString::new("time-pos").expect("CString::new failed");

    let c_path = mpv_get_property_string(handle, property_path.as_ptr());
    let path = CStr::from_ptr(c_path);

    let youtube_id = get_youtube_id(path);
    log::info!("YouTube ID: {:?}!", youtube_id);

    match youtube_id {
        Some(_id) => mpv_observe_property(
            handle,
            YOUTUBE_REPLY_USERDATA,
            property_time.as_ptr(),
            MPV_FORMAT_DOUBLE,
        ),
        None => mpv_unobserve_property(handle, YOUTUBE_REPLY_USERDATA),
    };

    mpv_free(c_path as *mut c_void);
}

unsafe fn event_property_changed(handle: *mut mpv_handle, reply_userdata: u64) {
    match reply_userdata {
        YOUTUBE_REPLY_USERDATA => println!("Thing happened!"),
        _ => {}
    }
}

#[no_mangle]
pub unsafe extern "C" fn mpv_open_cplugin(handle: *mut mpv_handle) -> c_int {
    env_logger::init();

    log::info!(
        "Starting plugin SponsorBlock ({:?})!",
        CStr::from_ptr(mpv_client_name(handle))
    );

    loop {
        let event: *mut mpv_event = mpv_wait_event(handle, -1.0);

        log::debug!("Event received: {}", (*event).event_id);

        match (*event).event_id {
            MPV_EVENT_SHUTDOWN => return 0,
            MPV_EVENT_FILE_LOADED => event_file_loaded(handle),
            MPV_EVENT_PROPERTY_CHANGE => event_property_changed(handle, (*event).reply_userdata),
            _ => {}
        }
    }
}
