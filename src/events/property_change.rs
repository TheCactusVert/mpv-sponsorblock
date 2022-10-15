use crate::mpv::{MpvEventProperty, MpvFormat, MpvHandle, MpvReplyUser};
use crate::sponsorblock::segment::Segments;
use crate::WATCHER_TIME;

fn event_time_change(
    mpv_handle: &MpvHandle,
    mpv_event: MpvEventProperty,
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
            if let Err(e) = mpv_handle.set_property("time-pos", MpvFormat::DOUBLE, new_time_pos) {
                log::error!("Failed to set time position property: {}", e);
            }
        }
    }
}

pub fn event(
    mpv_handle: &MpvHandle,
    mpv_reply: MpvReplyUser,
    mpv_event: MpvEventProperty,
    segments: &Option<Segments>,
) {
    match mpv_reply {
        WATCHER_TIME => event_time_change(mpv_handle, mpv_event, segments),
        _ => {}
    }
}
