use crate::mpv::*;
use crate::sponsorblock::segment::{Segments};
use crate::YT_REPLY_USERDATA;

pub unsafe fn event(handle: *mut Handle) -> Option<Segments> {
    log::debug!("File ended.");

    mpv_unobserve_property(handle, YT_REPLY_USERDATA);
    
    None
}
