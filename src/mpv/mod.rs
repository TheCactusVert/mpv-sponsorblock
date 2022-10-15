mod ffi;

use std::ffi::{c_void, CStr, CString};

use anyhow::{anyhow, Result};

pub type MpvFormat = ffi::mpv_format;
pub type MpvRawHandle = *mut ffi::mpv_handle;
pub type MpvReplyUser = u64;
pub struct MpvHandle(*mut ffi::mpv_handle);
pub struct MpvEventProperty(*mut ffi::mpv_event_property);

pub enum MpvEventID {
    None,
    Shutdown,
    StartFile,
    EndFile,
    PropertyChange(MpvReplyUser, MpvEventProperty),
}

impl MpvHandle {
    pub fn from_ptr(handle: MpvRawHandle) -> Self {
        assert!(!handle.is_null());
        Self(handle)
    }

    pub fn wait_event(&self, timeout: f64) -> MpvEventID {
        unsafe {
            let mpv_event = ffi::mpv_wait_event(self.0, timeout);

            if mpv_event.is_null() {
                return MpvEventID::None;
            }

            match (*mpv_event).event_id {
                ffi::mpv_event_id::SHUTDOWN => MpvEventID::Shutdown,
                ffi::mpv_event_id::START_FILE => MpvEventID::StartFile,
                ffi::mpv_event_id::END_FILE => MpvEventID::EndFile,
                ffi::mpv_event_id::PROPERTY_CHANGE => MpvEventID::PropertyChange(
                    (*mpv_event).reply_userdata,
                    MpvEventProperty::from_ptr((*mpv_event).data as *mut ffi::mpv_event_property),
                ),
                _ => MpvEventID::None,
            }
        }
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

        unsafe {
            let data: *mut c_void = &mut data as *mut _ as *mut c_void;
            match ffi::mpv_set_property(self.0, c_name.as_ptr(), format, data) {
                ffi::mpv_error::SUCCESS => Ok(()),
                e => Err(anyhow!(CStr::from_ptr(ffi::mpv_error_string(e))
                    .to_str()
                    .unwrap())),
            }
        }
    }

    pub fn observe_property<S: Into<String>>(
        &self,
        reply_userdata: u64,
        name: S,
        format: MpvFormat,
    ) -> Result<()> {
        let c_name = CString::new(name.into())?;

        unsafe {
            match ffi::mpv_observe_property(self.0, reply_userdata, c_name.as_ptr(), format) {
                ffi::mpv_error::SUCCESS => Ok(()),
                e => Err(anyhow!(CStr::from_ptr(ffi::mpv_error_string(e))
                    .to_str()
                    .unwrap())),
            }
        }
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
