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
