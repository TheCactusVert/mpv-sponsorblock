mod ffi;

use std::ffi::{c_void, CStr, CString};

use anyhow::{anyhow, Result};

pub type MpvEventID = ffi::mpv_event_id;
pub type MpvFormat = ffi::mpv_format;
pub type MpvRawHandle = *mut ffi::mpv_handle;
pub struct MpvHandle(*mut ffi::mpv_handle);
pub struct MpvEvent(*mut ffi::mpv_event);
pub struct MpvEventProperty(*mut ffi::mpv_event_property);

impl MpvHandle {
    pub fn from_ptr(handle: MpvRawHandle) -> Self {
        assert!(!handle.is_null());
        Self(handle)
    }

    pub fn wait_event(&self, timeout: f64) -> MpvEvent {
        MpvEvent::from_ptr(unsafe { ffi::mpv_wait_event(self.0, timeout) })
    }

    pub fn client_name(&self) -> Result<String> {
        Ok(unsafe {
            let c_name = ffi::mpv_client_name(self.0);
            let c_str = CStr::from_ptr(c_name);
            let str_slice: &str = c_str.to_str()?;
            str_slice.to_owned()
        })
    }

    pub fn get_property_string<S: Into<String>>(&self, name: S) -> Result<String> {
        let c_name = CString::new(name.into())?;

        unsafe {
            let c_path = ffi::mpv_get_property_string(self.0, c_name.as_ptr());
            if c_path.is_null() {
                return Err(anyhow!("Failed to get property"));
            }
            let c_str = CStr::from_ptr(c_path);
            let str_buf = c_str.to_str().map(|s| s.to_owned()).map_err(|e| anyhow!(e));
            ffi::mpv_free(c_path as *mut c_void);
            str_buf
        }
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
            ffi::mpv_set_property(self.0, c_name.as_ptr(), format, data) == 0
        } {
            Ok(())
        } else {
            Err(anyhow!("Failed to set property"))
        }
    }

    pub fn observe_property<S: Into<String>>(
        &self,
        reply_userdata: u64,
        name: S,
        format: MpvFormat,
    ) -> Result<()> {
        let c_name = CString::new(name.into())?;

        if unsafe {
            ffi::mpv_observe_property(self.0, reply_userdata, c_name.as_ptr(), format) == 0
        } {
            Ok(())
        } else {
            Err(anyhow!("Failed to observe property"))
        }
    }
}

impl MpvEvent {
    fn from_ptr(event: *mut ffi::mpv_event) -> Self {
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
        MpvEventProperty::from_ptr(unsafe { (*self.0).data as *mut ffi::mpv_event_property })
    }
}

impl MpvEventProperty {
    fn from_ptr(event_property: *mut ffi::mpv_event_property) -> Self {
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
