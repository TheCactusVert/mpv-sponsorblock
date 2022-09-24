use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};

use regex::Regex;
use sponsor_block::{AcceptedActions, AcceptedCategories, Client, Segment};

// This should be random, treated like a password, and stored across sessions
const USER_ID: &str = include_str!("../user.in");

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

pub type Segments = Vec<Segment>;

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

unsafe fn event_file_loaded(handle: *mut mpv_handle) -> Option<Segments> {
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
    
    let mut segments: Option<Segments> = None;

    loop {
        let event: *mut mpv_event = mpv_wait_event(handle, -1.0);

        log::debug!("Event received: {}", (*event).event_id);
        
        let event_id = (*event).event_id;
        
        if event_id == MPV_EVENT_SHUTDOWN {
            return 0;
        } else if event_id == MPV_EVENT_FILE_LOADED {
            segments = event_file_loaded(handle);
        } else if event_id == MPV_EVENT_PROPERTY_CHANGE {
            event_property_changed(handle, (*event).reply_userdata);
        }
    }
}
