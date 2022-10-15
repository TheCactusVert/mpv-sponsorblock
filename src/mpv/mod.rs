mod ffi;

use std::any::TypeId;
use std::ffi::{c_void, CStr, CString};
use std::fmt;

use anyhow::{anyhow, Result};

pub type MpvFormat = ffi::mpv_format;
pub type MpvRawHandle = *mut ffi::mpv_handle;
pub type MpvReplyUser = u64;
pub struct MpvHandle(*mut ffi::mpv_handle);
pub struct MpvEventProperty(*mut ffi::mpv_event_property);

pub enum MpvEventID {
    None,
    Shutdown,
    LogMessage, // TODO mpv_event_log_message
    GetPropertyReply(MpvReplyUser, MpvEventProperty),
    SetPropertyReply,
    CommandReply, // TODO mpv_event_command
    StartFile,    // TODO mpv_event_start_file
    EndFile,      // TODO mpv_event_end_file
    FileLoaded,
    ClientMessage, // TODO mpv_event_client_message
    VideoReconfig,
    AudioReconfig,
    Seek,
    PlaybackRestart,
    PropertyChange(MpvReplyUser, MpvEventProperty),
    QueueOverflow,
    Hook, // TODO mpv_event_hook
}

impl fmt::Display for MpvEventID {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Self::None => write!(f, "none"),
            Self::Shutdown => write!(f, "shutdown"),
            Self::LogMessage => write!(f, "log message"),
            Self::GetPropertyReply(_, _) => write!(f, "get property reply"),
            Self::SetPropertyReply => write!(f, "set property reply"),
            Self::CommandReply => write!(f, "command reply"),
            Self::StartFile => write!(f, "start file"),
            Self::EndFile => write!(f, "end file"),
            Self::FileLoaded => write!(f, "file loaded"),
            Self::ClientMessage => write!(f, "client message"),
            Self::VideoReconfig => write!(f, "video reconfig"),
            Self::AudioReconfig => write!(f, "audio reconfig"),
            Self::Seek => write!(f, "seek"),
            Self::PlaybackRestart => write!(f, "playback restart"),
            Self::PropertyChange(_, _) => write!(f, "property change"),
            Self::QueueOverflow => write!(f, "queue overflow"),
            Self::Hook => write!(f, "hook"),
        }
    }
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
                ffi::mpv_event_id::LOG_MESSAGE => MpvEventID::LogMessage,
                ffi::mpv_event_id::GET_PROPERTY_REPLY => MpvEventID::GetPropertyReply(
                    (*mpv_event).reply_userdata,
                    MpvEventProperty::from_ptr((*mpv_event).data as *mut ffi::mpv_event_property),
                ),
                ffi::mpv_event_id::SET_PROPERTY_REPLY => MpvEventID::SetPropertyReply,
                ffi::mpv_event_id::COMMAND_REPLY => MpvEventID::CommandReply,
                ffi::mpv_event_id::START_FILE => MpvEventID::StartFile,
                ffi::mpv_event_id::END_FILE => MpvEventID::EndFile,
                ffi::mpv_event_id::FILE_LOADED => MpvEventID::FileLoaded,
                ffi::mpv_event_id::CLIENT_MESSAGE => MpvEventID::ClientMessage,
                ffi::mpv_event_id::VIDEO_RECONFIG => MpvEventID::VideoReconfig,
                ffi::mpv_event_id::AUDIO_RECONFIG => MpvEventID::AudioReconfig,
                ffi::mpv_event_id::SEEK => MpvEventID::Seek,
                ffi::mpv_event_id::PLAYBACK_RESTART => MpvEventID::PlaybackRestart,
                ffi::mpv_event_id::PROPERTY_CHANGE => MpvEventID::PropertyChange(
                    (*mpv_event).reply_userdata,
                    MpvEventProperty::from_ptr((*mpv_event).data as *mut ffi::mpv_event_property),
                ),
                ffi::mpv_event_id::QUEUE_OVERFLOW => MpvEventID::QueueOverflow,
                ffi::mpv_event_id::HOOK => MpvEventID::Hook,
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
        assert!(!event_property.is_null()); // TODO dangerous
        Self(event_property)
    }

    pub fn get_data<T: Copy + 'static>(&self) -> Option<T> {
        unsafe {
            let format = (*self.0).format;
            if format == ffi::mpv_format::NONE {
                return None;
            }

            let type_id = TypeId::of::<T>();
            if type_id == TypeId::of::<i64>() {
                assert!(
                    format == ffi::mpv_format::INT64,
                    "The format is not of type i64!"
                );
                Some(*((*self.0).data as *mut T))
            } else if type_id == TypeId::of::<f64>() {
                assert!(
                    format == ffi::mpv_format::DOUBLE,
                    "The format is not of type f64!"
                );
                Some(*((*self.0).data as *mut T))
            } else {
                panic!("Unsupported format!");
            }
        }
    }
}
