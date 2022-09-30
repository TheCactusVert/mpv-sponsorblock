use crate::mpv::*;
use crate::YT_REPLY_USERDATA;

pub unsafe fn event(handle: *mut Handle) {
    mpv_unobserve_property(handle, YT_REPLY_USERDATA);
}
