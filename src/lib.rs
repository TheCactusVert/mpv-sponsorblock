#![feature(drain_filter)]

mod client;
mod config;

use client::{Client, REPL_PROP_MUTE, REPL_PROP_TIME};
use mpv_client::{mpv_handle, Event};
use simple_logger::SimpleLogger;

// MPV entry point
#[no_mangle]
extern "C" fn mpv_open_cplugin(handle: *mut mpv_handle) -> std::os::raw::c_int {
    let mut mpv = Client::from_ptr(handle); // Wrap handle

    // Init logger
    if let Err(e) = SimpleLogger::new().with_level(log::LevelFilter::Warn).env().init() {
        log::warn!("Logger error: {}", e);
    }

    // Show that the plugin has started
    log::debug!("Starting plugin SponsorBlock [{}]!", mpv.client_name());

    loop {
        // Wait for MPV events indefinitely
        match mpv.wait_event(-1.) {
            Event::StartFile(_data) => {
                log::trace!("Received start-file event");
                mpv.start_file();
            }
            Event::PropertyChange(REPL_PROP_TIME, data) => {
                if let Some(time_pos) = data.data() {
                    log::trace!("Received {} on reply {}", data.name(), REPL_PROP_TIME);
                    mpv.time_change(time_pos);
                }
            }
            Event::PropertyChange(REPL_PROP_MUTE, data) => {
                if let Some(mute) = data.data() {
                    log::trace!("Received {} on reply {}", data.name(), REPL_PROP_MUTE);
                    mpv.mute_change(mute);
                }
            }
            Event::ClientMessage(data) => {
                log::trace!("Received client-message event");
                mpv.client_message(data.args().iter().map(|v| v.as_str()).collect::<Vec<&str>>().as_slice());
            }
            Event::EndFile => {
                log::trace!("Received end-file event");
                mpv.end_file();
            }
            Event::Shutdown => {
                log::trace!("Received shutdown event");
                return 0;
            }
            _ => {}
        }
    }
}
