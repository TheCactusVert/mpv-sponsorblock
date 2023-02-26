#![feature(drain_filter)]

mod client;
mod config;

use client::Client;
use mpv_client::mpv_handle;
use simple_logger::SimpleLogger;

// MPV entry point
#[no_mangle]
extern "C" fn mpv_open_cplugin(handle: *mut mpv_handle) -> std::os::raw::c_int {
    // MPV handle
    let mut mpv = Client::from_ptr(handle);

    // Init logger
    if let Err(e) = SimpleLogger::new().with_level(log::LevelFilter::Warn).env().init() {
        log::warn!("Logger error: {}", e);
    }

    // Show that the plugin has started
    log::debug!("Starting plugin SponsorBlock [{}]!", mpv.client_name());

    // MPV loop
    match mpv.exec() {
        Ok(()) => 0,
        Err(e) => {
            log::error!("Unhandled error on plugin SponsorBlock: {}", e);
            -1
        }
    }
}
