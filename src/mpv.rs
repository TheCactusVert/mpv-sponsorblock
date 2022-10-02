use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};

use anyhow::Result;

#[repr(i32)]
#[derive(Copy, Clone, PartialEq)]
pub enum MpvEventID {
    Shutdown = 1,
    StartFile = 6,
    EndFile = 7,
    PropertyChange = 22,
}

#[repr(i32)]
pub enum MpvFormat {
    Double = 5,
}

#[repr(C)]
pub struct mpv_handle {
    _unused: [u8; 0],
}
pub struct MpvHandle(*mut mpv_handle);

#[repr(C)]
struct mpv_event {
    pub event_id: MpvEventID,
    pub error: c_int,
    pub reply_userdata: u64,
    pub data: *mut c_void,
}
pub struct MpvEvent(*mut mpv_event);

#[repr(C)]
struct mpv_event_property {
    pub name: *const c_char,
    pub format: MpvFormat,
    pub data: *mut c_void,
}
pub struct MpvEventProperty(*mut mpv_event_property);

extern "C" {
    fn mpv_wait_event(ctx: *mut mpv_handle, timeout: f64) -> *mut mpv_event;
    fn mpv_client_name(ctx: *mut mpv_handle) -> *const c_char;
    fn mpv_get_property_string(ctx: *mut mpv_handle, name: *const c_char) -> *mut c_char;
    fn mpv_set_property(
        ctx: *mut mpv_handle,
        name: *const c_char,
        format: MpvFormat,
        data: *mut c_void,
    ) -> c_int;
    fn mpv_free(data: *mut c_void);
    fn mpv_observe_property(
        mpv: *mut mpv_handle,
        reply_userdata: u64,
        name: *const c_char,
        format: MpvFormat,
    ) -> c_int;
}

impl MpvHandle {
    pub fn from_ptr(handle: *mut mpv_handle) -> Self {
        assert!(!handle.is_null());
        Self(handle)
    }

    pub fn wait_event(&self, timeout: f64) -> MpvEvent {
        MpvEvent::from_ptr(unsafe { mpv_wait_event(self.0, timeout) })
    }

    pub fn client_name(&self) -> Result<String> {
        Ok(unsafe {
            let c_name = mpv_client_name(self.0);
            let c_str = CStr::from_ptr(c_name);
            let str_slice: &str = c_str.to_str()?;
            str_slice.to_owned()
        })
    }

    pub fn get_property_string<S: Into<String>>(&self, name: S) -> Result<String> {
        let c_name = CString::new(name.into())?;

        Ok(unsafe {
            let c_path = mpv_get_property_string(self.0, c_name.as_ptr());
            let c_str = CStr::from_ptr(c_path);
            let str_slice: &str = c_str.to_str()?;
            let str_buf: String = str_slice.to_owned();
            mpv_free(c_path as *mut c_void);
            str_buf
        })
    }

    pub fn set_property<S: Into<String>, T>(
        &self,
        name: S,
        format: MpvFormat,
        mut data: T,
    ) -> Result<()> {
        let c_name = CString::new(name.into())?;

        if unsafe {
            let data: *mut c_void = &mut data as *mut _ as *mut c_void;
            mpv_set_property(self.0, c_name.as_ptr(), format, data) == 0
        } {
            Ok(())
        } else {
            Err(anyhow::anyhow!("Failed to set property"))
        }
    }

    pub fn observe_property<S: Into<String>>(
        &self,
        reply_userdata: u64,
        name: S,
        format: MpvFormat,
    ) -> Result<()> {
        let c_name = CString::new(name.into())?;

        if unsafe { mpv_observe_property(self.0, reply_userdata, c_name.as_ptr(), format) == 0 } {
            Ok(())
        } else {
            Err(anyhow::anyhow!("Failed to observe property"))
        }
    }
}

impl MpvEvent {
    fn from_ptr(event: *mut mpv_event) -> Self {
        assert!(!event.is_null());
        Self(event)
    }

    pub fn get_event_id(&self) -> MpvEventID {
        unsafe { (*self.0).event_id }
    }

    pub fn get_reply_userdata(&self) -> u64 {
        unsafe { (*self.0).reply_userdata }
    }

    pub fn get_event_property(&self) -> MpvEventProperty {
        MpvEventProperty::from_ptr(unsafe { (*self.0).data as *mut mpv_event_property })
    }
}

impl MpvEventProperty {
    fn from_ptr(event_property: *mut mpv_event_property) -> Self {
        assert!(!event_property.is_null());
        Self(event_property)
    }

    pub fn get_data<T: Copy>(&self) -> Option<T> {
        unsafe {
            let data = (*self.0).data as *mut T;
            if data.is_null() {
                None
            } else {
                Some(*data)
            }
        }
    }
}
