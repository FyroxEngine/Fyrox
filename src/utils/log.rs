//! Simple logger, it writes in file and in console at the same time.

use std::{
    fs::File,
    io::{self, Write},
    sync::Mutex,
};

lazy_static! {
    static ref LOG_FILE: Mutex<File> = Mutex::new(File::create("rg3d.log").unwrap());
}

/// See module docs.
pub struct Log {}

impl Log {
    /// Writes string into console and into file.
    pub fn write(msg: String) {
        let _ = io::stdout().write_all(msg.as_bytes());
        let _ = LOG_FILE.lock().unwrap().write_all(msg.as_bytes());
    }

    /// Writes line into console and into file.
    pub fn writeln(mut msg: String) {
        msg.push('\n');
        Self::write(msg)
    }
}
