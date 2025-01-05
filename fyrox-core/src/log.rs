// Copyright (c) 2019-present Dmitry Stepanov and Fyrox Engine contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

//! Simple logger. By default, it writes in the console only. To enable logging into a file, call
//! [`Log::set_file_name`] somewhere in your `main` function.

use crate::parking_lot::Mutex;
use std::fmt::{Debug, Display};

use crate::instant::Instant;
#[cfg(not(target_arch = "wasm32"))]
use std::io::{self, Write};
use std::path::Path;
use std::sync::mpsc::Sender;
use std::sync::LazyLock;
use std::time::Duration;

#[cfg(target_arch = "wasm32")]
use crate::wasm_bindgen::{self, prelude::*};

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    // Use `js_namespace` here to bind `console.log(..)` instead of just
    // `log(..)`
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

/// A message that could be sent by the logger to all listeners.
pub struct LogMessage {
    /// Kind of the message: information, warning or error.
    pub kind: MessageKind,
    /// The source message without logger prefixes.
    pub content: String,
    /// Time point at which the message was recorded. It is relative to the moment when the
    /// logger was initialized.
    pub time: Duration,
}

static LOG: LazyLock<Mutex<Log>> = LazyLock::new(|| {
    Mutex::new(Log {
        #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
        file: None,
        verbosity: MessageKind::Information,
        listeners: Default::default(),
        time_origin: Instant::now(),
    })
});

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
    #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
    file: Option<std::fs::File>,
    verbosity: MessageKind,
    listeners: Vec<Sender<LogMessage>>,
    time_origin: Instant,
}

impl Log {
    /// Creates a new log file at the specified path.
    pub fn set_file_name<P: AsRef<Path>>(#[allow(unused_variables)] path: P) {
        #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
        {
            let mut guard = LOG.lock();
            guard.file = std::fs::File::create(path).ok();
        }
    }

    /// Sets new file to write the log to.
    pub fn set_file(#[allow(unused_variables)] file: Option<std::fs::File>) {
        #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
        {
            let mut guard = LOG.lock();
            guard.file = file;
        }
    }

    fn write_internal<S>(&mut self, kind: MessageKind, message: S)
    where
        S: AsRef<str>,
    {
        let mut msg = message.as_ref().to_owned();
        if kind as u32 >= self.verbosity as u32 {
            // Notify listeners about the message and remove all disconnected listeners.
            self.listeners.retain(|listener| {
                listener
                    .send(LogMessage {
                        kind,
                        content: msg.clone(),
                        time: Instant::now() - self.time_origin,
                    })
                    .is_ok()
            });

            msg.insert_str(0, kind.as_str());

            #[cfg(target_arch = "wasm32")]
            {
                log(&msg);
            }

            #[cfg(all(not(target_os = "android"), not(target_arch = "wasm32")))]
            {
                let _ = io::stdout().write_all(msg.as_bytes());

                if let Some(log_file) = self.file.as_mut() {
                    let _ = log_file.write_all(msg.as_bytes());
                    let _ = log_file.flush();
                }
            }

            #[cfg(target_os = "android")]
            {
                let _ = io::stdout().write_all(msg.as_bytes());
            }
        }
    }

    fn writeln_internal<S>(&mut self, kind: MessageKind, message: S)
    where
        S: AsRef<str>,
    {
        let mut msg = message.as_ref().to_owned();
        msg.push('\n');
        self.write_internal(kind, msg)
    }

    /// Writes string into console and into file.
    pub fn write<S>(kind: MessageKind, msg: S)
    where
        S: AsRef<str>,
    {
        LOG.lock().write_internal(kind, msg);
    }

    /// Writes line into console and into file.
    pub fn writeln<S>(kind: MessageKind, msg: S)
    where
        S: AsRef<str>,
    {
        LOG.lock().writeln_internal(kind, msg);
    }

    /// Writes information message.
    pub fn info<S>(msg: S)
    where
        S: AsRef<str>,
    {
        Self::writeln(MessageKind::Information, msg)
    }

    /// Writes warning message.
    pub fn warn<S>(msg: S)
    where
        S: AsRef<str>,
    {
        Self::writeln(MessageKind::Warning, msg)
    }

    /// Writes error message.
    pub fn err<S>(msg: S)
    where
        S: AsRef<str>,
    {
        Self::writeln(MessageKind::Error, msg)
    }

    /// Sets verbosity level.
    pub fn set_verbosity(kind: MessageKind) {
        LOG.lock().verbosity = kind;
    }

    /// Adds a listener that will receive a copy of every message passed into the log.
    pub fn add_listener(listener: Sender<LogMessage>) {
        LOG.lock().listeners.push(listener)
    }

    /// Allows you to verify that the result of operation is Ok, or print the error in the log.
    ///
    /// # Use cases
    ///
    /// Typical use case for this method is that when you _can_ ignore errors, but want them to
    /// be in the log.
    pub fn verify<T, E>(result: Result<T, E>)
    where
        E: Debug,
    {
        if let Err(e) = result {
            Self::writeln(
                MessageKind::Error,
                format!("Operation failed! Reason: {e:?}"),
            );
        }
    }

    /// Allows you to verify that the result of operation is Ok, or print the error in the log.
    ///
    /// # Use cases
    ///
    /// Typical use case for this method is that when you _can_ ignore errors, but want them to
    /// be in the log.
    pub fn verify_message<S, T, E>(result: Result<T, E>, msg: S)
    where
        E: Debug,
        S: Display,
    {
        if let Err(e) = result {
            Self::writeln(MessageKind::Error, format!("{msg}. Reason: {e:?}"));
        }
    }
}

#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {
        $crate::log::Log::info(format!($($arg)*))
    };
}

#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => {
        $crate::log::Log::warn(format!($($arg)*))
    };
}

#[macro_export]
macro_rules! err {
    ($($arg:tt)*) => {
        $crate::log::Log::err(format!($($arg)*))
    };
}
