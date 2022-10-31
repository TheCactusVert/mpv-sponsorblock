use std::ffi::{c_char, c_double, c_int, c_ulonglong, c_void};

#[repr(i32)]
#[allow(dead_code)]
#[allow(non_camel_case_types)]
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum mpv_error {
    SUCCESS = 0,
    EVENT_QUEUE_FULL = -1,
    NOMEM = -2,
    UNINITIALIZED = -3,
    INVALID_PARAMETER = -4,
    OPTION_NOT_FOUND = -5,
    OPTION_FORMAT = -6,
    OPTION_ERROR = -7,
    PROPERTY_NOT_FOUND = -8,
    PROPERTY_FORMAT = -9,
    PROPERTY_UNAVAILABLE = -10,
    PROPERTY_ERROR = -11,
    COMMAND = -12,
    LOADING_FAILED = -13,
    AO_INIT_FAILED = -14,
    VO_INIT_FAILED = -15,
    NOTHING_TO_PLAY = -16,
    UNKNOWN_FORMAT = -17,
    UNSUPPORTED = -18,
    NOT_IMPLEMENTED = -19,
    GENERIC = -20,
}

#[repr(i32)]
#[allow(dead_code)]
#[allow(non_camel_case_types)]
#[derive(Copy, Clone, PartialEq)]
pub enum mpv_format {
    NONE = 0,
    STRING = 1,
    OSD_STRING = 2,
    FLAG = 3,
    INT64 = 4,
    DOUBLE = 5,
    NODE_ARRAY = 7,
    NODE_MAP = 8,
    BYTE_ARRAY = 9,
}

#[repr(i32)]
#[allow(dead_code)]
#[allow(non_camel_case_types)]
#[derive(Copy, Clone, PartialEq)]
pub enum mpv_event_id {
    NONE = 0,
    SHUTDOWN = 1,
    LOG_MESSAGE = 2,
    GET_PROPERTY_REPLY = 3,
    SET_PROPERTY_REPLY = 4,
    COMMAND_REPLY = 5,
    START_FILE = 6,
    END_FILE = 7,
    FILE_LOADED = 8,
    CLIENT_MESSAGE = 16,
    VIDEO_RECONFIG = 17,
    AUDIO_RECONFIG = 18,
    SEEK = 20,
    PLAYBACK_RESTART = 21,
    PROPERTY_CHANGE = 22,
    QUEUE_OVERFLOW = 24,
    HOOK = 25,
}

#[allow(non_camel_case_types)]
pub type mpv_handle = c_void;

#[repr(C)]
#[allow(non_camel_case_types)]
pub struct mpv_event_start_file {
    pub playlist_entry_id: c_ulonglong,
}

#[repr(C)]
#[allow(non_camel_case_types)]
pub struct mpv_event_property {
    pub name: *const c_char,
    pub format: mpv_format,
    pub data: *mut c_void,
}

#[repr(C)]
#[allow(non_camel_case_types)]
pub struct mpv_event_hook {
    pub name: *const c_char,
    pub id: c_ulonglong,
}

#[repr(C)]
#[allow(non_camel_case_types)]
pub struct mpv_event {
    pub event_id: mpv_event_id,
    pub error: mpv_error,
    pub reply_userdata: c_ulonglong,
    pub data: *mut c_void,
}

extern "C" {
    pub fn mpv_error_string(error: mpv_error) -> *const c_char;
    pub fn mpv_free(data: *mut c_void);
    pub fn mpv_client_name(ctx: *mut mpv_handle) -> *const c_char;
    pub fn mpv_command(ctx: *mut mpv_handle, args: *const *const c_char) -> mpv_error;
    pub fn mpv_set_property(
        ctx: *mut mpv_handle,
        name: *const c_char,
        format: mpv_format,
        data: *const c_void,
    ) -> mpv_error;
    pub fn mpv_get_property(
        ctx: *mut mpv_handle,
        name: *const c_char,
        format: mpv_format,
        data: *mut c_void,
    ) -> mpv_error;
    pub fn mpv_observe_property(
        ctx: *mut mpv_handle,
        reply_userdata: c_ulonglong,
        name: *const c_char,
        format: mpv_format,
    ) -> mpv_error;
    pub fn mpv_wait_event(ctx: *mut mpv_handle, timeout: c_double) -> *mut mpv_event;
    pub fn mpv_hook_add(
        ctx: *mut mpv_handle,
        reply_userdata: c_ulonglong,
        name: *const c_char,
        priority: c_int,
    ) -> mpv_error;
    pub fn mpv_hook_continue(ctx: *mut mpv_handle, id: c_ulonglong) -> mpv_error;
}
