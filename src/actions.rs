use crate::config::Config;
use crate::mpv::{EventProperty, Format, Handle};
use crate::sponsorblock::segment::{Segment, Segments};
use crate::utils::get_youtube_id;

pub struct Actions {
    segments: Option<Segments>,
}

impl Actions {
    pub fn new() -> Self {
        Self { segments: None }
    }

    pub fn load_segments(&mut self, mpv_handle: &Handle, config: &Config) {
        let path = mpv_handle.get_property_string("path").unwrap();
        let yt_id = get_youtube_id(&path);

        self.segments = match yt_id {
            Some(id) if config.privacy_api => Segment::get_segments_with_privacy(config, id),
            Some(id) => Segment::get_segments(config, id),
            None => None,
        };
    }

    pub fn drop_segments(&mut self) {
        self.segments = None;
    }

    pub fn skip_segments(&self, mpv_handle: &Handle, mpv_event: EventProperty) {
        if let Some(segments) = &self.segments {
            let old_time_pos: f64 = match mpv_event.get_data() {
                Some(v) => v,
                None => return,
            };
            let mut new_time_pos: f64 = old_time_pos;

            for segment in segments.iter().filter(|s| s.action.as_str() == "skip") {
                if new_time_pos >= segment.segment[0] && new_time_pos < segment.segment[1] {
                    log::info!(
                        "Skipping segment [{}] from {} to {}.",
                        segment.category,
                        segment.segment[0],
                        segment.segment[1]
                    );

                    new_time_pos = segment.segment[1];
                }
            }

            if old_time_pos != new_time_pos {
                if let Err(e) = mpv_handle.set_property("time-pos", Format::DOUBLE, new_time_pos) {
                    log::error!("Failed to set time position property: {}", e);
                }
            }
        }
    }
}
