use crate::mpv::{EventProperty, Format, Handle};
use crate::sponsorblock::segment::Segments;

pub fn event_time_change(
    mpv_handle: &Handle,
    mpv_event: EventProperty,
    segments: &Option<Segments>,
) {
    if let Some(segments) = segments {
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
