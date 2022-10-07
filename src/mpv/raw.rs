use std::os::raw::{c_char, c_double, c_int, c_ulonglong, c_void};

#[repr(i32)]
#[allow(dead_code)]
#[allow(non_camel_case_types)]
#[derive(Copy, Clone, PartialEq)]
pub enum mpv_event_id {
    Shutdown = 1,
    StartFile = 6,
    EndFile = 7,
    PropertyChange = 22,
}

#[repr(i32)]
#[allow(dead_code)]
#[allow(non_camel_case_types)]
#[derive(Copy, Clone, PartialEq)]
pub enum mpv_format {
    None = 0,
    String = 1,
    OSDString = 2,
    Flag = 3,
    Int64 = 4,
    Double = 5,
    NodeArray = 7,
    NodeMap = 8,
    ByteArray = 9,
}

#[repr(C)]
#[allow(non_camel_case_types)]
pub struct mpv_handle {
    _unused: [u8; 0],
}

#[repr(C)]
#[allow(non_camel_case_types)]
pub struct mpv_event {
    pub event_id: mpv_event_id,
    pub error: c_int,
    pub reply_userdata: c_ulonglong,
    pub data: *mut c_void,
}

#[repr(C)]
#[allow(non_camel_case_types)]
pub struct mpv_event_property {
    pub name: *const c_char,
    pub format: mpv_format,
    pub data: *mut c_void,
}

extern "C" {
    pub fn mpv_wait_event(ctx: *mut mpv_handle, timeout: c_double) -> *mut mpv_event;
    pub fn mpv_client_name(ctx: *mut mpv_handle) -> *const c_char;
    pub fn mpv_get_property_string(ctx: *mut mpv_handle, name: *const c_char) -> *mut c_char;
    pub fn mpv_set_property(
        ctx: *mut mpv_handle,
        name: *const c_char,
        format: mpv_format,
        data: *mut c_void,
    ) -> c_int;
    pub fn mpv_free(data: *mut c_void);
    pub fn mpv_observe_property(
        mpv: *mut mpv_handle,
        reply_userdata: c_ulonglong,
        name: *const c_char,
        format: mpv_format,
    ) -> c_int;
}
