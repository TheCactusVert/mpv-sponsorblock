mod ffi;

use std::any::TypeId;
use std::ffi::{c_void, CStr, CString, NulError};
use std::fmt;
use std::str::Utf8Error;

pub type Format = ffi::mpv_format;
pub type RawHandle = *mut ffi::mpv_handle;
pub type ReplyUser = u64;
pub struct Handle(*mut ffi::mpv_handle);
pub struct EventStartFile(*mut ffi::mpv_event_start_file);
pub struct EventProperty(*mut ffi::mpv_event_property);

#[derive(Debug)]
pub struct Error(ffi::mpv_error);
pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    fn new(error: ffi::mpv_error) -> Self {
        Self(error)
    }
}

impl From<NulError> for Error {
    fn from(_: NulError) -> Self {
        Self::new(ffi::mpv_error::GENERIC)
    }
}

impl From<Utf8Error> for Error {
    fn from(_: Utf8Error) -> Self {
        Self::new(ffi::mpv_error::GENERIC)
    }
}

impl std::error::Error for Error {
    fn description(&self) -> &str {
        unsafe {
            CStr::from_ptr(ffi::mpv_error_string(self.0))
                .to_str()
                .unwrap_or("unknow error")
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let e_str = unsafe {
            CStr::from_ptr(ffi::mpv_error_string(self.0))
                .to_str()
                .unwrap_or("unknow error")
        };
        write!(f, "{}", e_str)
    }
}

pub enum Event {
    None,
    Shutdown,
    LogMessage, // TODO mpv_event_log_message
    GetPropertyReply(EventProperty),
    SetPropertyReply,
    CommandReply, // TODO mpv_event_command
    StartFile(EventStartFile),
    EndFile, // TODO mpv_event_end_file
    FileLoaded,
    ClientMessage, // TODO mpv_event_client_message
    VideoReconfig,
    AudioReconfig,
    Seek,
    PlaybackRestart,
    PropertyChange(EventProperty),
    QueueOverflow,
    Hook, // TODO mpv_event_hook
}

impl fmt::Display for Event {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Self::None => write!(f, "none"),
            Self::Shutdown => write!(f, "shutdown"),
            Self::LogMessage => write!(f, "log message"),
            Self::GetPropertyReply(_) => write!(f, "get property reply"),
            Self::SetPropertyReply => write!(f, "set property reply"),
            Self::CommandReply => write!(f, "command reply"),
            Self::StartFile(_) => write!(f, "start file"),
            Self::EndFile => write!(f, "end file"),
            Self::FileLoaded => write!(f, "file loaded"),
            Self::ClientMessage => write!(f, "client message"),
            Self::VideoReconfig => write!(f, "video reconfig"),
            Self::AudioReconfig => write!(f, "audio reconfig"),
            Self::Seek => write!(f, "seek"),
            Self::PlaybackRestart => write!(f, "playback restart"),
            Self::PropertyChange(_) => write!(f, "property change"),
            Self::QueueOverflow => write!(f, "queue overflow"),
            Self::Hook => write!(f, "hook"),
        }
    }
}

impl Handle {
    pub fn from_ptr(handle: RawHandle) -> Self {
        assert!(!handle.is_null());
        Self(handle)
    }

    pub fn wait_event(&self, timeout: f64) -> (ReplyUser, Result<Event>) {
        unsafe {
            let mpv_event = ffi::mpv_wait_event(self.0, timeout);

            if mpv_event.is_null() {
                return (0, Ok(Event::None));
            }

            let mpv_reply: ReplyUser = (*mpv_event).reply_userdata;

            if (*mpv_event).error != ffi::mpv_error::SUCCESS {
                return (mpv_reply, Err(Error::new((*mpv_event).error)));
            }

            (
                mpv_reply,
                Ok(match (*mpv_event).event_id {
                    ffi::mpv_event_id::SHUTDOWN => Event::Shutdown,
                    ffi::mpv_event_id::LOG_MESSAGE => Event::LogMessage,
                    ffi::mpv_event_id::GET_PROPERTY_REPLY => Event::GetPropertyReply(EventProperty::from_ptr(
                        (*mpv_event).data as *mut ffi::mpv_event_property,
                    )),
                    ffi::mpv_event_id::SET_PROPERTY_REPLY => Event::SetPropertyReply,
                    ffi::mpv_event_id::COMMAND_REPLY => Event::CommandReply,
                    ffi::mpv_event_id::START_FILE => Event::StartFile(EventStartFile::from_ptr(
                        (*mpv_event).data as *mut ffi::mpv_event_start_file,
                    )),
                    ffi::mpv_event_id::END_FILE => Event::EndFile,
                    ffi::mpv_event_id::FILE_LOADED => Event::FileLoaded,
                    ffi::mpv_event_id::CLIENT_MESSAGE => Event::ClientMessage,
                    ffi::mpv_event_id::VIDEO_RECONFIG => Event::VideoReconfig,
                    ffi::mpv_event_id::AUDIO_RECONFIG => Event::AudioReconfig,
                    ffi::mpv_event_id::SEEK => Event::Seek,
                    ffi::mpv_event_id::PLAYBACK_RESTART => Event::PlaybackRestart,
                    ffi::mpv_event_id::PROPERTY_CHANGE => Event::PropertyChange(EventProperty::from_ptr(
                        (*mpv_event).data as *mut ffi::mpv_event_property,
                    )),
                    ffi::mpv_event_id::QUEUE_OVERFLOW => Event::QueueOverflow,
                    ffi::mpv_event_id::HOOK => Event::Hook,
                    _ => Event::None,
                }),
            )
        }
    }

    pub fn client_name(&self) -> String {
        unsafe {
            let c_name = ffi::mpv_client_name(self.0);
            let c_str = CStr::from_ptr(c_name);
            let str_slice: &str = c_str.to_str().unwrap_or("unknown");
            str_slice.to_owned()
        }
    }

    pub fn get_property_string<S: Into<String>>(&self, name: S) -> Result<String> {
        let c_name = CString::new(name.into())?;

        unsafe {
            let c_path = ffi::mpv_get_property_string(self.0, c_name.as_ptr());
            if c_path.is_null() {
                return Err(Error::new(ffi::mpv_error::PROPERTY_NOT_FOUND));
            }
            let c_str = CStr::from_ptr(c_path);
            let str_buf = c_str.to_str().map(|s| s.to_owned());
            ffi::mpv_free(c_path as *mut c_void);
            Ok(str_buf?) // Meh it shouldn't fail
        }
    }

    pub fn set_property<S: Into<String>, T>(&self, name: S, format: Format, mut data: T) -> Result<()> {
        let c_name = CString::new(name.into())?;

        unsafe {
            let data: *mut c_void = &mut data as *mut _ as *mut c_void;
            match ffi::mpv_set_property(self.0, c_name.as_ptr(), format, data) {
                ffi::mpv_error::SUCCESS => Ok(()),
                e => Err(Error::new(e)),
            }
        }
    }

    pub fn observe_property<S: Into<String>>(&self, reply_userdata: ReplyUser, name: S, format: Format) -> Result<()> {
        let c_name = CString::new(name.into())?;

        unsafe {
            match ffi::mpv_observe_property(self.0, reply_userdata, c_name.as_ptr(), format) {
                ffi::mpv_error::SUCCESS => Ok(()),
                e => Err(Error::new(e)),
            }
        }
    }
}

impl EventStartFile {
    fn from_ptr(event: *mut ffi::mpv_event_start_file) -> Self {
        assert!(!event.is_null());
        Self(event)
    }

    pub fn get_playlist_entry_id(&self) -> u64 {
        unsafe { (*self.0).playlist_entry_id }
    }
}

impl EventProperty {
    fn from_ptr(event: *mut ffi::mpv_event_property) -> Self {
        assert!(!event.is_null());
        Self(event)
    }

    pub fn get_data<T: Copy + 'static>(&self) -> Option<T> {
        unsafe {
            let format = (*self.0).format;
            if format == ffi::mpv_format::NONE {
                return None;
            }

            let type_id = TypeId::of::<T>();
            if type_id == TypeId::of::<i64>() {
                assert!(format == ffi::mpv_format::INT64, "The format is not of type i64!");
                Some(*((*self.0).data as *mut T))
            } else if type_id == TypeId::of::<f64>() {
                assert!(format == ffi::mpv_format::DOUBLE, "The format is not of type f64!");
                Some(*((*self.0).data as *mut T))
            } else {
                panic!("Unsupported format!");
            }
        }
    }
}
