use std::ffi::CStr;
use std::os::raw::{c_char, c_int, c_void};

use anyhow::Result;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct mpv_handle {
    _unused: [u8; 0],
}
pub struct MpvHandle(*mut mpv_handle);

#[repr(C)]
#[derive(Debug, Copy, Clone)]
struct mpv_event {
    pub event_id: EventID,
    pub error: c_int,
    pub reply_userdata: u64,
    pub data: *mut c_void,
}
pub struct MpvEvent(*mut mpv_event);

#[repr(C)]
#[derive(Debug, Copy, Clone)]
struct mpv_event_property {
    pub name: *const c_char,
    pub format: Format,
    pub data: *mut c_void,
}
pub struct MpvEventProperty(*mut mpv_event_property);

pub const EVENT_SHUTDOWN: EventID = 1;
pub const EVENT_START_FILE: EventID = 6;
pub const EVENT_END_FILE: EventID = 7;
pub const EVENT_PROPERTY_CHANGE: EventID = 22;
pub type EventID = c_int;

pub const FORMAT_DOUBLE: Format = 5;
pub type Format = c_int;

extern "C" {
    fn mpv_wait_event(ctx: *mut mpv_handle, timeout: f64) -> *mut mpv_event;
    fn mpv_client_name(ctx: *mut mpv_handle) -> *const c_char;
    fn mpv_get_property_string(ctx: *mut mpv_handle, name: *const c_char) -> *mut c_char;
    fn mpv_set_property(
        ctx: *mut mpv_handle,
        name: *const c_char,
        format: Format,
        data: *mut c_void,
    ) -> c_int;
    fn mpv_free(data: *mut c_void);
    fn mpv_observe_property(
        mpv: *mut mpv_handle,
        reply_userdata: u64,
        name: *const c_char,
        format: Format,
    ) -> c_int;
}

impl MpvHandle {
    pub fn new(handle: *mut mpv_handle) -> Self {
        assert!(!handle.is_null());
        Self(handle)
    }

    pub fn wait_event(&self, timeout: f64) -> MpvEvent {
        unsafe { MpvEvent::new(mpv_wait_event(self.0, timeout)) }
    }

    pub fn client_name(&self) -> Result<String> {
        unsafe {
            let c_name = mpv_client_name(self.0);
            let c_str = CStr::from_ptr(c_name);
            let str_slice: &str = c_str.to_str()?;
            Ok(str_slice.to_owned())
        }
    }

    pub fn get_property_string(&self, name: &'static [u8]) -> Result<String> {
        unsafe {
            let c_path = mpv_get_property_string(self.0, name.as_ptr() as *const c_char);
            let c_str = CStr::from_ptr(c_path);
            let str_slice: &str = c_str.to_str()?;
            let str_buf: String = str_slice.to_owned();
            mpv_free(c_path as *mut c_void);
            Ok(str_buf)
        }
    }

    pub fn set_property<T>(&self, name: &'static [u8], format: Format, mut data: T) -> i32 {
        unsafe {
            let data: *mut c_void = &mut data as *mut _ as *mut c_void;
            mpv_set_property(self.0, name.as_ptr() as *const c_char, format, data)
        }
    }

    pub fn observe_property(
        &self,
        reply_userdata: u64,
        name: &'static [u8],
        format: Format,
    ) -> i32 {
        unsafe {
            mpv_observe_property(
                self.0,
                reply_userdata,
                name.as_ptr() as *const c_char,
                format,
            )
        }
    }
}

impl MpvEvent {
    fn new(event: *mut mpv_event) -> Self {
        assert!(!event.is_null());
        Self(event)
    }
    
    pub fn get_event_id(&self) -> EventID {
        unsafe {
            (*self.0).event_id
        }
    }
    
    pub fn get_reply_userdata(&self) -> u64 {
        unsafe {
            (*self.0).reply_userdata
        }
    }
    
    pub fn get_event_property(&self) -> MpvEventProperty {
        unsafe {
            MpvEventProperty::new((*self.0).data as *mut mpv_event_property)
        }
    }
}

impl MpvEventProperty {
    fn new(event_property: *mut mpv_event_property) -> Self {
        assert!(!event_property.is_null());
        Self(event_property)
    }
    
    pub fn get_data<T: Copy>(&self) -> Option<T> {
        unsafe {
            let data = (*self.0).data as *mut T;
            return if data.is_null() {
                 None
            } else {
                Some(*data)
            }
        }
    }
}
