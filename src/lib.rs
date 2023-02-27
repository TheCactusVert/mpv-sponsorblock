mod client;

use client::Client;
use mpv_client::mpv_handle;
use simple_logger::SimpleLogger;

// MPV entry point
#[no_mangle]
extern "C" fn mpv_open_cplugin(handle: *mut mpv_handle) -> std::os::raw::c_int {
    // MPV handle
    let mut client = Client::from_ptr(handle);

    // Init logger
    if let Err(e) = SimpleLogger::new().with_level(log::LevelFilter::Warn).env().init() {
        log::warn!("Logger error: {}", e);
    }

    // Show that the plugin has started
    log::debug!("Starting plugin SponsorBlock [{}]!", client.name());

    // MPV loop
    match client.exec() {
        Ok(()) => {
            log::debug!("Closing plugin SponsorBlock [{}]!", client.name());
            0
        }
        Err(e) => {
            log::error!("Unhandled error on plugin SponsorBlock [{}]: {}", client.name(), e);
            -1
        }
    }
}
