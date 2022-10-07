use crate::config::Config;
use crate::mpv::MpvHandle;
use crate::sponsorblock::segment::{Segment, Segments};
use crate::utils::get_youtube_id;

pub fn event(mpv_handle: &MpvHandle, config: &Config) -> Option<Segments> {
    log::debug!("File started.");

    let path = mpv_handle.get_property_string("path").ok()?;
    let yt_id = get_youtube_id(&path);

    match yt_id {
        Some(id) if config.privacy_api => Segment::get_segments_with_privacy(config, id),
        Some(id) => Segment::get_segments(config, id),
        None => None,
    }
}
