mod ffi;

use ffi::*;

use std::any::TypeId;
use std::ffi::{c_void, CStr, CString, NulError};
use std::fmt;
use std::str::Utf8Error;

pub type Format = mpv_format;
pub type RawHandle = *mut mpv_handle;
pub type ReplyUser = u64;
pub struct Handle(*mut mpv_handle);
pub struct EventStartFile(*mut mpv_event_start_file);
pub struct EventProperty(*mut mpv_event_property);
pub struct EventHook(*mut mpv_event_hook);

#[derive(Debug)]
pub struct Error(mpv_error);
pub type Result<T> = std::result::Result<T, Error>;

macro_rules! convert_result {
    ($f:expr) => {
        match $f {
            mpv_error::SUCCESS => Ok(()),
            e => Err(Error::new(e)),
        }
    };
}

impl Error {
    fn new(error: mpv_error) -> Self {
        Self(error)
    }
}

impl From<NulError> for Error {
    fn from(_: NulError) -> Self {
        Self::new(mpv_error::GENERIC)
    }
}

impl From<Utf8Error> for Error {
    fn from(_: Utf8Error) -> Self {
        Self::new(mpv_error::GENERIC)
    }
}

impl std::error::Error for Error {
    fn description(&self) -> &str {
        unsafe {
            CStr::from_ptr(mpv_error_string(self.0))
                .to_str()
                .unwrap_or("unknow error")
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let e_str = unsafe {
            CStr::from_ptr(mpv_error_string(self.0))
                .to_str()
                .unwrap_or("unknow error")
        };
        write!(f, "[{}] {}", self.0 as i32, e_str)
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
    Hook(EventHook),
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
            Self::Hook(_) => write!(f, "hook"),
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
            let mpv_event = mpv_wait_event(self.0, timeout);

            if mpv_event.is_null() {
                return (0, Ok(Event::None));
            }

            let mpv_reply: ReplyUser = (*mpv_event).reply_userdata;

            if (*mpv_event).error != mpv_error::SUCCESS {
                return (mpv_reply, Err(Error::new((*mpv_event).error)));
            }

            (
                mpv_reply,
                Ok(match (*mpv_event).event_id {
                    mpv_event_id::SHUTDOWN => Event::Shutdown,
                    mpv_event_id::LOG_MESSAGE => Event::LogMessage,
                    mpv_event_id::GET_PROPERTY_REPLY => {
                        Event::GetPropertyReply(EventProperty::from_ptr((*mpv_event).data as *mut mpv_event_property))
                    }
                    mpv_event_id::SET_PROPERTY_REPLY => Event::SetPropertyReply,
                    mpv_event_id::COMMAND_REPLY => Event::CommandReply,
                    mpv_event_id::START_FILE => {
                        Event::StartFile(EventStartFile::from_ptr((*mpv_event).data as *mut mpv_event_start_file))
                    }
                    mpv_event_id::END_FILE => Event::EndFile,
                    mpv_event_id::FILE_LOADED => Event::FileLoaded,
                    mpv_event_id::CLIENT_MESSAGE => Event::ClientMessage,
                    mpv_event_id::VIDEO_RECONFIG => Event::VideoReconfig,
                    mpv_event_id::AUDIO_RECONFIG => Event::AudioReconfig,
                    mpv_event_id::SEEK => Event::Seek,
                    mpv_event_id::PLAYBACK_RESTART => Event::PlaybackRestart,
                    mpv_event_id::PROPERTY_CHANGE => {
                        Event::PropertyChange(EventProperty::from_ptr((*mpv_event).data as *mut mpv_event_property))
                    }
                    mpv_event_id::QUEUE_OVERFLOW => Event::QueueOverflow,
                    mpv_event_id::HOOK => Event::Hook(EventHook::from_ptr((*mpv_event).data as *mut mpv_event_hook)),
                    _ => Event::None,
                }),
            )
        }
    }

    pub fn client_name(&self) -> String {
        unsafe {
            let c_name = mpv_client_name(self.0);
            let c_str = CStr::from_ptr(c_name);
            let str_slice: &str = c_str.to_str().unwrap_or("unknown");
            str_slice.to_owned()
        }
    }

    pub fn get_property_string<S: Into<String>>(&self, name: S) -> Result<String> {
        let c_name = CString::new(name.into())?;

        unsafe {
            let c_path = mpv_get_property_string(self.0, c_name.as_ptr());
            if c_path.is_null() {
                return Err(Error::new(mpv_error::PROPERTY_NOT_FOUND));
            }
            let c_str = CStr::from_ptr(c_path);
            let str_buf = c_str.to_str().map(|s| s.to_owned());
            mpv_free(c_path as *mut c_void);
            Ok(str_buf?) // Meh it shouldn't fail
        }
    }

    pub fn set_property<S: Into<String>, T>(&self, name: S, format: Format, mut data: T) -> Result<()> {
        let c_name = CString::new(name.into())?;
        let data: *mut c_void = &mut data as *mut _ as *mut c_void;
        unsafe { convert_result!(mpv_set_property(self.0, c_name.as_ptr(), format, data)) }
    }

    pub fn observe_property<S: Into<String>>(&self, reply_userdata: ReplyUser, name: S, format: Format) -> Result<()> {
        let c_name = CString::new(name.into())?;
        unsafe { convert_result!(mpv_observe_property(self.0, reply_userdata, c_name.as_ptr(), format)) }
    }

    pub fn hook_add<S: Into<String>>(&self, reply_userdata: ReplyUser, name: S, priority: i32) -> Result<()> {
        let c_name = CString::new(name.into())?;
        unsafe { convert_result!(mpv_hook_add(self.0, reply_userdata, c_name.as_ptr(), priority)) }
    }

    pub fn hook_continue(&self, id: u64) -> Result<()> {
        unsafe { convert_result!(mpv_hook_continue(self.0, id)) }
    }
}

impl EventStartFile {
    fn from_ptr(event: *mut mpv_event_start_file) -> Self {
        assert!(!event.is_null());
        Self(event)
    }

    pub fn get_playlist_entry_id(&self) -> u64 {
        unsafe { (*self.0).playlist_entry_id }
    }
}

impl EventProperty {
    fn from_ptr(event: *mut mpv_event_property) -> Self {
        assert!(!event.is_null());
        Self(event)
    }

    pub fn get_data<T: Copy + 'static>(&self) -> Option<T> {
        unsafe {
            let format = (*self.0).format;
            if format == mpv_format::NONE {
                return None;
            }

            let type_id = TypeId::of::<T>();
            if type_id == TypeId::of::<i64>() {
                assert!(format == mpv_format::INT64, "The format is not of type i64!");
                Some(*((*self.0).data as *mut T))
            } else if type_id == TypeId::of::<f64>() {
                assert!(format == mpv_format::DOUBLE, "The format is not of type f64!");
                Some(*((*self.0).data as *mut T))
            } else {
                panic!("Unsupported format!");
            }
        }
    }
}

impl EventHook {
    fn from_ptr(event: *mut mpv_event_hook) -> Self {
        assert!(!event.is_null());
        Self(event)
    }

    pub fn get_id(&self) -> u64 {
        unsafe { (*self.0).id }
    }
}
