use log::{Level, Log, Metadata, Record};
use mpv_client::Client;

use std::ops::Deref;

pub struct Logger(Client);

impl Logger {
    fn new() -> Logger {
        let client = Client::new();

        Logger(client.initialize().unwrap())
    }
}

unsafe impl Send for Logger {}

unsafe impl Sync for Logger {}

impl Deref for Logger {
    type Target = Client;

    #[inline]
    fn deref(&self) -> &Client {
        &self.0
    }
}

impl Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            println!("{} - {}", record.level(), record.args());
        }
    }

    fn flush(&self) {}
}
