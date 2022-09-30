use crate::mpv::*;
use crate::YT_REPLY_USERDATA;

use std::ffi::CString;

pub unsafe fn event(handle: *mut Handle) {
    log::debug!("File started.");

    let property_time = CString::new("time-pos").unwrap();

    mpv_observe_property(
            handle,
            YT_REPLY_USERDATA,
            property_time.as_ptr(),
            FORMAT_DOUBLE,
        );
}
