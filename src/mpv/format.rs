use super::ffi::{mpv_format, mpv_free};
use super::Result;

use std::ffi::{c_char, c_void, CStr, CString};

pub trait Format: Sized + Default {
    const FORMAT: mpv_format;
    fn from_raw(raw: *mut c_void) -> Result<Self>;
    fn to_mpv<F: Fn(*const c_void) -> Result<()>>(self, fun: F) -> Result<()>;
    fn from_mpv<F: Fn(*mut c_void) -> Result<()>>(fun: F) -> Result<Self>;
}

impl Format for f64 {
    const FORMAT: mpv_format = mpv_format::DOUBLE;

    fn from_raw(raw: *mut c_void) -> Result<Self> {
        Ok(unsafe { *(raw as *mut Self) })
    }

    fn to_mpv<F: Fn(*const c_void) -> Result<()>>(self, fun: F) -> Result<()> {
        fun(&self as *const _ as *const c_void)
    }

    fn from_mpv<F: Fn(*mut c_void) -> Result<()>>(fun: F) -> Result<Self> {
        let mut data = Self::default();
        fun(&mut data as *mut _ as *mut c_void)?;
        Ok(data)
    }
}

impl Format for i64 {
    const FORMAT: mpv_format = mpv_format::INT64;

    fn from_raw(raw: *mut c_void) -> Result<Self> {
        Ok(unsafe { *(raw as *mut Self) })
    }

    fn to_mpv<F: Fn(*const c_void) -> Result<()>>(self, fun: F) -> Result<()> {
        fun(&self as *const _ as *const c_void)
    }

    fn from_mpv<F: Fn(*mut c_void) -> Result<()>>(fun: F) -> Result<Self> {
        let mut data = Self::default();
        fun(&mut data as *mut _ as *mut c_void)?;
        Ok(data)
    }
}

impl Format for String {
    const FORMAT: mpv_format = mpv_format::STRING;

    fn from_raw(raw: *mut c_void) -> Result<Self> {
        Ok(unsafe { CString::from_raw(raw as *mut i8) }.to_str()?.to_string())
    }

    fn to_mpv<F: Fn(*const c_void) -> Result<()>>(self, fun: F) -> Result<()> {
        let str = CString::new::<String>(self.into())?;
        fun(str.as_ptr() as *const c_void)
    }

    fn from_mpv<F: Fn(*mut c_void) -> Result<()>>(fun: F) -> Result<Self> {
        let mut ptr: *mut c_char = std::ptr::null_mut();
        fun(&mut ptr as *mut _ as *mut c_void)?;
        unsafe {
            let str = CStr::from_ptr(ptr as *mut i8);
            let str = str.to_str().map(|s| s.to_owned());
            mpv_free(ptr as *mut c_void);
            Ok(str?)
        }
    }
}
