//! Resource event handling.

use crate::{
    core::{
        parking_lot::Mutex,
        pool::{Handle, Pool},
    },
    utils::log::Log,
};
use std::{
    path::PathBuf,
    sync::{mpsc::Sender, Arc},
};

/// A resource event.
#[derive(Clone)]
pub enum ResourceEvent<T>
where
    T: Clone,
{
    /// Occurs when a resource was fully loaded without any errors.
    Loaded(T),

    /// Occurs when a resource was already fully loaded, but was reloaded by an explicit request.
    Reloaded(T),

    /// Occurs when a resource was just added to a resource container.
    Added(T),

    /// Occurs when a resource was removed from a resource container.
    Removed(PathBuf),
}

/// Type alias for event sender.
pub type ResourceEventSender<T> = Sender<ResourceEvent<T>>;

/// Event broadcaster is responsible for delivering resource events to "subscribers".
#[derive(Clone)]
pub struct ResourceEventBroadcaster<T>
where
    T: Clone,
{
    container: Arc<Mutex<Pool<ResourceEventSender<T>>>>,
}

impl<T> Default for ResourceEventBroadcaster<T>
where
    T: Send + Clone + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T> ResourceEventBroadcaster<T>
where
    T: Send + Clone + 'static,
{
    /// Creates new empty event broadcaster.
    pub fn new() -> Self {
        Self {
            container: Arc::new(Default::default()),
        }
    }

    /// Adds an event sender to the broadcaster and returns its handle.
    pub fn add(&self, sender: ResourceEventSender<T>) -> Handle<ResourceEventSender<T>> {
        self.container.lock().spawn(sender)
    }

    /// Removes an event sender by its handle.
    pub fn remove(&self, handle: Handle<ResourceEventSender<T>>) -> ResourceEventSender<T> {
        self.container.lock().free(handle)
    }

    /// Sends an event to all "subscribers" in the broadcaster.
    pub fn broadcast(&self, event: ResourceEvent<T>) {
        let container = self.container.lock();
        for sender in container.iter() {
            Log::verify(sender.send(event.clone()));
        }
    }

    /// Sends a [`ResourceEvent::Loaded`] event to all "subscribers" in the broadcaster.
    pub fn broadcast_loaded(&self, resource: T) {
        self.broadcast(ResourceEvent::Loaded(resource))
    }

    /// Sends either a [`ResourceEvent::Loaded`] event or a [`ResourceEvent::Reloaded`] to all
    /// "subscribers" in the broadcaster depending on the `reload` parameter.
    pub fn broadcast_loaded_or_reloaded(&self, resource: T, reload: bool) {
        self.broadcast(if reload {
            ResourceEvent::Reloaded(resource)
        } else {
            ResourceEvent::Loaded(resource)
        })
    }
}
