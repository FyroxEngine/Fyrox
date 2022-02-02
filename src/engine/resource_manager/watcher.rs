//! Resource watcher allows you to track changed resources and "tell" resource manager to reload
//! them.

use notify::{watcher, DebouncedEvent, RecommendedWatcher, RecursiveMode, Watcher};
use std::{
    path::Path,
    sync::mpsc::{channel, Receiver},
    time::Duration,
};

/// Resource watcher allows you to track changed resources and "tell" resource manager to reload
/// them.
pub struct ResourceWatcher {
    #[allow(dead_code)] // We must keep watcher alive, but compiler isn't smart enough.
    watcher: RecommendedWatcher,
    receiver: Receiver<DebouncedEvent>,
}

impl ResourceWatcher {
    /// Creates new resource watcher with a path to watch and notification delay.
    pub fn new<P: AsRef<Path>>(path: P, delay: Duration) -> Result<Self, notify::Error> {
        let (tx, rx) = channel();

        let mut watcher = watcher(tx, delay)?;

        watcher.watch(path, RecursiveMode::Recursive)?;

        Ok(Self {
            receiver: rx,
            watcher,
        })
    }

    pub(crate) fn try_get_event(&self) -> Option<DebouncedEvent> {
        self.receiver.try_recv().ok()
    }
}
