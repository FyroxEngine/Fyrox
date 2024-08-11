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

//! Resource watcher allows you to track changed resources and "tell" resource manager to reload
//! them.

use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::{
    path::Path,
    sync::mpsc::{channel, Receiver},
    time::Duration,
};

/// Resource watcher allows you to track changed resources and "tell" resource manager to reload
/// them.
pub struct FileSystemWatcher {
    #[allow(dead_code)] // We must keep watcher alive, but compiler isn't smart enough.
    watcher: RecommendedWatcher,
    receiver: Receiver<notify::Result<Event>>,
}

impl FileSystemWatcher {
    /// Creates new resource watcher with a path to watch and notification delay.
    pub fn new<P: AsRef<Path>>(path: P, delay: Duration) -> Result<Self, notify::Error> {
        let (tx, rx) = channel();

        let mut watcher = RecommendedWatcher::new(tx, Config::default().with_poll_interval(delay))?;

        watcher.watch(path.as_ref(), RecursiveMode::Recursive)?;

        Ok(Self {
            receiver: rx,
            watcher,
        })
    }

    pub fn try_get_event(&self) -> Option<Event> {
        if let Ok(Ok(evt)) = self.receiver.try_recv() {
            return Some(evt);
        }
        None
    }
}
