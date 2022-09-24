use crate::{YOUTUBE_REPLY_USERDATA, Segments};
use crate::mpv::*;

pub unsafe fn event(handle: *mut mpv_handle, reply_userdata: u64) {
    match reply_userdata {
        YOUTUBE_REPLY_USERDATA => println!("Thing happened!"),
        _ => {}
    }
}
