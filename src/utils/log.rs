//! Simple logger, it writes in file and in console at the same time.

use crate::lazy_static::lazy_static;
use std::{
    fs::File,
    io::{self, Write},
    sync::Mutex,
};

lazy_static! {
    static ref LOG: Mutex<Log> = Mutex::new(Log {
        file: File::create("rg3d.log").unwrap(),
        verbosity: MessageKind::Information
    });
}

/// A kind of message.
#[derive(Copy, Clone, PartialOrd, PartialEq, Eq, Ord, Hash)]
#[repr(u32)]
pub enum MessageKind {
    /// Some useful information.
    Information = 0,
    /// A warning.
    Warning = 1,
    /// An error of some kind.
    Error = 2,
}

impl MessageKind {
    fn as_str(self) -> &'static str {
        match self {
            MessageKind::Information => "[INFO]: ",
            MessageKind::Warning => "[WARNING]: ",
            MessageKind::Error => "[ERROR]: ",
        }
    }
}

/// See module docs.
pub struct Log {
    file: File,
    verbosity: MessageKind,
}

impl Log {
    fn write_internal(&mut self, kind: MessageKind, mut msg: String) {
        if kind as u32 >= self.verbosity as u32 {
            msg.insert_str(0, kind.as_str());
            let _ = io::stdout().write_all(msg.as_bytes());
            let _ = self.file.write_all(msg.as_bytes());
        }
    }

    fn writeln_internal(&mut self, kind: MessageKind, mut msg: String) {
        msg.push('\n');
        self.write_internal(kind, msg)
    }

    /// Writes string into console and into file.
    pub fn write(kind: MessageKind, msg: String) {
        LOG.lock().unwrap().write_internal(kind, msg);
    }

    /// Writes line into console and into file.
    pub fn writeln(kind: MessageKind, msg: String) {
        LOG.lock().unwrap().writeln_internal(kind, msg);
    }

    /// Sets verbosity level.
    pub fn set_verbosity(kind: MessageKind) {
        LOG.lock().unwrap().verbosity = kind;
    }
}
