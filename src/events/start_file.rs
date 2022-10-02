use crate::config::Config;
use crate::mpv::MpvHandle;
use crate::sponsorblock::segment::{Segment, Segments};

use regex::Regex;

fn get_youtube_id(path: String) -> Option<String> {
    // I don't uderstand this crap but it's working
    let regexes = [
        Regex::new(r"https?://youtu%.be/([A-Za-z0-9-_]+).*").unwrap(),
        Regex::new(r"https?://w?w?w?%.?youtube%.com/v/([A-Za-z0-9-_]+).*").unwrap(),
        Regex::new(r"/watch.*[?&]v=([A-Za-z0-9-_]+).*").unwrap(),
        Regex::new(r"/embed/([A-Za-z0-9-_]+).*").unwrap(),
        Regex::new(r".*\[([A-Za-z0-9-_]+)\]\.webm").unwrap(),
    ];

    regexes
        .into_iter()
        .filter_map(|r| r.captures(&path))
        .find_map(|c| c.get(1).map(|m| m.as_str().to_string()))
}

pub fn event(mpv_handle: &MpvHandle, config: &Config) -> Option<Segments> {
    log::debug!("File started.");

    let path = mpv_handle.get_property_string("path").ok()?;
    let yt_id = get_youtube_id(path);

    match yt_id {
        Some(id) if config.privacy_api => Segment::get_segments_with_privacy(config, id),
        Some(id) => Segment::get_segments(config, id),
        None => None,
    }
}
