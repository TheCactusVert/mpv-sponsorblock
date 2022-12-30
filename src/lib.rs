#![feature(drain_filter)]
#![feature(if_let_guard)]

mod config;
mod sponsorblock;
mod state;
mod utils;
mod worker;

use config::Config;
use mpv_client::{mpv_handle, Event, Handle};
use state::{State, REPL_PROP_MUTE, REPL_PROP_TIME};

use env_logger::Env;

static LOG_ENV: &str = "MPV_SPONSORBLOCK_LOG";
static LOG_ENV_STYLE: &str = "MPV_SPONSORBLOCK_LOG_STYLE";

// MPV entry point
#[no_mangle]
extern "C" fn mpv_open_cplugin(handle: *mut mpv_handle) -> std::os::raw::c_int {
    let mpv = Handle::from_ptr(handle); // Wrap handle

    // Init logger with custom env
    let env = Env::new().filter(LOG_ENV).write_style(LOG_ENV_STYLE);
    env_logger::init_from_env(env);

    // Show that the plugin has started
    log::debug!("Starting plugin SponsorBlock [{}]!", mpv.client_name());

    let config = Config::default(); // Read config
    let mut state: Option<State> = None; // State handler of MPV

    loop {
        // Wait for MPV events indefinitely
        match mpv.wait_event(-1.) {
            Event::StartFile(_) => {
                log::trace!("Received start-file event");
                state = State::new(&mpv, config.clone());
            }
            Event::PropertyChange(REPL_PROP_TIME, data) if let Some(state) = state.as_mut() => {
                log::trace!("Received {} on reply {}", data.name(), REPL_PROP_TIME);
                if let Some(time_pos) = data.data::<f64>() {
                    state.time_change(&mpv, time_pos);
                }
            }
            Event::PropertyChange(REPL_PROP_MUTE, data) if let Some(state) = state.as_mut() => {
                log::trace!("Received {} on reply {}", data.name(), REPL_PROP_MUTE);
                if let Some(mute) = data.data::<String>() {
                    state.mute_change(mute);
                }
            }
            Event::EndFile if let Some(mut state) = state.take() => {
                log::trace!("Received end-file event");
                state.end_file(&mpv);
            }
            Event::Shutdown => {
                log::trace!("Received shutdown event");
                return 0;
            }
            Event::QueueOverflow => {
                log::trace!("Received queue-overflow event");
                // TODO Might be good to handle ??
            }
            event => {
                log::trace!("Ignoring {} event", event);
            }
        }
    }
}
