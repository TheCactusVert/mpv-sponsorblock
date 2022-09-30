use std::os::raw::{c_char, c_int, c_void};

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Handle {
    _unused: [u8; 0],
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Event {
    pub event_id: EventID,
    pub error: c_int,
    pub reply_userdata: u64,
    pub data: *mut c_void,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct EventProperty {
    pub name: *const c_char,
    pub format: Format,
    pub data: *mut c_void,
}

pub const EVENT_SHUTDOWN: EventID = 1;
pub const EVENT_START_FILE: EventID = 6;
pub const EVENT_END_FILE: EventID = 7;
pub const EVENT_FILE_LOADED: EventID = 8;
pub const EVENT_PROPERTY_CHANGE: EventID = 22;
pub type EventID = c_int;

pub const FORMAT_DOUBLE: Format = 5;
pub type Format = c_int;

extern "C" {
    pub fn mpv_wait_event(ctx: *mut Handle, timeout: f64) -> *mut Event;
    pub fn mpv_client_name(ctx: *mut Handle) -> *const c_char;
    pub fn mpv_get_property_string(ctx: *mut Handle, name: *const c_char) -> *mut c_char;
    pub fn mpv_free(data: *mut c_void);
    pub fn mpv_observe_property(
        mpv: *mut Handle,
        reply_userdata: u64,
        name: *const c_char,
        format: Format,
    ) -> c_int;
    pub fn mpv_unobserve_property(mpv: *mut Handle, registered_reply_userdata: u64) -> c_int;
    pub fn mpv_set_property(
        ctx: *mut Handle,
        name: *const c_char,
        format: Format,
        data: *mut c_void,
    ) -> c_int;
}
