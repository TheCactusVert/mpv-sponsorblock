use super::ffi::mpv_format;

use std::ffi::{c_void, CStr};

pub trait Format: Sized {
    fn get_format() -> mpv_format;
    fn from_raw(raw: *const c_void) -> Option<Self>;
}

impl Format for f64 {
    fn get_format() -> mpv_format {
        mpv_format::DOUBLE
    }

    fn from_raw(raw: *const c_void) -> Option<Self> {
        Some(unsafe { *(raw as *mut Self) })
    }
}

impl Format for i64 {
    fn get_format() -> mpv_format {
        mpv_format::INT64
    }

    fn from_raw(raw: *const c_void) -> Option<Self> {
        Some(unsafe { *(raw as *mut Self) })
    }
}

impl Format for String {
    fn get_format() -> mpv_format {
        mpv_format::STRING
    }

    fn from_raw(raw: *const c_void) -> Option<Self> {
        let c_str = unsafe { CStr::from_ptr(*(raw as *const *const i8)) };
        let str_slice = c_str.to_str().ok()?;
        Some(str_slice.to_owned())
    }
}
