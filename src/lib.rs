#![feature(drain_filter)]
#![feature(if_let_guard)]

mod config;
mod event_handler;

use config::Config;
use event_handler::{EventHandler, REPL_PROP_MUTE, REPL_PROP_TIME};
use mpv_client::{mpv_handle, Event, Handle};
use simple_logger::SimpleLogger;

// MPV entry point
#[no_mangle]
extern "C" fn mpv_open_cplugin(handle: *mut mpv_handle) -> std::os::raw::c_int {
    let mpv = Handle::from_ptr(handle); // Wrap handle

    // Init logger
    if let Err(e) = SimpleLogger::new().with_level(log::LevelFilter::Warn).env().init() {
        log::warn!("Logger error: {}", e);
    }

    // Show that the plugin has started
    log::debug!("Starting plugin SponsorBlock [{}]!", mpv.client_name());

    let config = Config::get(); // Read config
    let mut event_handler: Option<EventHandler> = None; // Event handler of MPV

    loop {
        // Wait for MPV events indefinitely
        match mpv.wait_event(-1.) {
            Event::StartFile(_data) => {
                log::trace!("Received start-file event");
                event_handler = EventHandler::new(&mpv, config.clone());
            }
            Event::PropertyChange(REPL_PROP_TIME, data) if let Some(event_handler) = event_handler.as_mut() => {
                log::trace!("Received {} on reply {}", data.name(), REPL_PROP_TIME);
                if let Some(time_pos) = data.data() {
                    event_handler.time_change(&mpv, &config, time_pos);
                }
            }
            Event::PropertyChange(REPL_PROP_MUTE, data) if let Some(event_handler) = event_handler.as_mut() => {
                log::trace!("Received {} on reply {}", data.name(), REPL_PROP_MUTE);
                if let Some(mute) = data.data() {
                    event_handler.mute_change(mute);
                }
            }
            Event::ClientMessage(data) => if let Some(event_handler) = event_handler.as_mut()  {
                log::trace!("Received client-message event");
                event_handler.client_message(&mpv, &config, data.args().iter().map(|v| v.as_str()).collect::<Vec<&str>>().as_slice());
            }
            Event::EndFile if let Some(mut event_handler) = event_handler.take() => {
                log::trace!("Received end-file event");
                event_handler.end_file(&mpv);
            }
            Event::Shutdown => {
                log::trace!("Received shutdown event");
                return 0;
            }
            Event::QueueOverflow => {
                log::warn!("Received queue-overflow event");
                // TODO Might be good to handle ??
            }
            event => {
                log::trace!("Ignoring {} event", event);
            }
        }
    }
}
