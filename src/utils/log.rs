//! Simple logger, it writes in file and in console at the same time.

use crate::lazy_static::lazy_static;
use std::{fmt::Debug, sync::Mutex};

#[cfg(not(target_arch = "wasm32"))]
use std::io::{self, Write};

#[cfg(target_arch = "wasm32")]
use crate::core::wasm_bindgen::{self, prelude::*};

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    // Use `js_namespace` here to bind `console.log(..)` instead of just
    // `log(..)`
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

lazy_static! {
    static ref LOG: Mutex<Log> = Mutex::new(Log {
        #[cfg(not(target_arch = "wasm32"))]
        file: std::fs::File::create("fyrox.log").unwrap(),
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
    #[cfg(not(target_arch = "wasm32"))]
    file: std::fs::File,
    verbosity: MessageKind,
}

impl Log {
    fn write_internal(&mut self, kind: MessageKind, mut msg: String) {
        if kind as u32 >= self.verbosity as u32 {
            msg.insert_str(0, kind.as_str());

            #[cfg(target_arch = "wasm32")]
            {
                log(&msg);
            }

            #[cfg(not(target_arch = "wasm32"))]
            {
                let _ = io::stdout().write_all(msg.as_bytes());
                let _ = self.file.write_all(msg.as_bytes());
            }
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

    /// Allows you to verify that the result of operation is Ok, or print the error in the log.
    ///
    /// # Use cases
    ///
    /// Typical use case for this method is that when you _can_ ignore errors, but want them to
    /// be in the log.
    pub fn verify<E>(result: Result<(), E>)
    where
        E: Debug,
    {
        if let Err(e) = result {
            Self::writeln(
                MessageKind::Error,
                format!("Operation failed! Reason: {:?}", e),
            );
        }
    }
}
