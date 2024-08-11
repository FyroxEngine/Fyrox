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

//! Resource event handling.

use crate::core::{
    parking_lot::Mutex,
    pool::{Handle, Pool},
};
use crate::UntypedResource;
use std::{
    path::PathBuf,
    sync::{mpsc::Sender, Arc},
};

/// A resource event.
#[derive(Clone)]
pub enum ResourceEvent {
    /// Occurs when a resource was fully loaded without any errors.
    Loaded(UntypedResource),

    /// Occurs when a resource was already fully loaded, but was reloaded by an explicit request.
    Reloaded(UntypedResource),

    /// Occurs when a resource was just added to a resource container.
    Added(UntypedResource),

    /// Occurs when a resource was removed from a resource container.
    Removed(PathBuf),
}

/// Type alias for event sender.
pub type ResourceEventSender = Sender<ResourceEvent>;

/// Event broadcaster is responsible for delivering resource events to "subscribers".
#[derive(Clone)]
pub struct ResourceEventBroadcaster {
    container: Arc<Mutex<Pool<ResourceEventSender>>>,
}

impl Default for ResourceEventBroadcaster {
    fn default() -> Self {
        Self::new()
    }
}

impl ResourceEventBroadcaster {
    /// Creates new empty event broadcaster.
    pub fn new() -> Self {
        Self {
            container: Arc::new(Default::default()),
        }
    }

    /// Adds an event sender to the broadcaster and returns its handle.
    pub fn add(&self, sender: ResourceEventSender) -> Handle<ResourceEventSender> {
        self.container.lock().spawn(sender)
    }

    /// Removes an event sender by its handle.
    pub fn remove(&self, handle: Handle<ResourceEventSender>) -> ResourceEventSender {
        self.container.lock().free(handle)
    }

    /// Sends an event to all "subscribers" in the broadcaster.
    pub fn broadcast(&self, event: ResourceEvent) {
        let container = self.container.lock();
        for sender in container.iter() {
            let _ = sender.send(event.clone());
        }
    }

    /// Sends a [`ResourceEvent::Loaded`] event to all "subscribers" in the broadcaster.
    pub fn broadcast_loaded(&self, resource: UntypedResource) {
        self.broadcast(ResourceEvent::Loaded(resource))
    }

    /// Sends either a [`ResourceEvent::Loaded`] event or a [`ResourceEvent::Reloaded`] to all
    /// "subscribers" in the broadcaster depending on the `reload` parameter.
    pub fn broadcast_loaded_or_reloaded(&self, resource: UntypedResource, reload: bool) {
        self.broadcast(if reload {
            ResourceEvent::Reloaded(resource)
        } else {
            ResourceEvent::Loaded(resource)
        })
    }
}

#[cfg(test)]
mod test {
    use std::sync::mpsc::channel;

    use super::*;

    #[test]
    fn resource_event_broadcaster_add_and_remove() {
        let broadcaster = ResourceEventBroadcaster::new();
        let (sender, receiver) = channel();

        let h = broadcaster.add(sender);
        assert!(h.is_some());
        assert_eq!(h.index(), 0);
        assert_eq!(h.generation(), 1);

        broadcaster.broadcast(ResourceEvent::Added(UntypedResource::default()));
        assert!(matches!(
            receiver.recv(),
            Ok(ResourceEvent::Added(UntypedResource(_)))
        ));

        broadcaster.remove(h);
        broadcaster.broadcast(ResourceEvent::Added(UntypedResource::default()));
        assert!(receiver.recv().is_err());
    }

    #[test]
    fn resource_event_broadcaster_broadcast_loaded() {
        let broadcaster = ResourceEventBroadcaster::default();
        let (sender, receiver) = channel();
        broadcaster.add(sender);

        broadcaster.broadcast_loaded(UntypedResource::default());
        assert!(matches!(
            receiver.recv(),
            Ok(ResourceEvent::Loaded(UntypedResource(_)))
        ));
    }

    #[test]
    fn resource_event_broadcaster_broadcast_loaded_or_reloaded() {
        let broadcaster = ResourceEventBroadcaster::default();
        let (sender, receiver) = channel();
        broadcaster.add(sender);

        broadcaster.broadcast_loaded_or_reloaded(UntypedResource::default(), false);
        assert!(matches!(
            receiver.recv(),
            Ok(ResourceEvent::Loaded(UntypedResource(_)))
        ));

        broadcaster.broadcast_loaded_or_reloaded(UntypedResource::default(), true);
        assert!(matches!(
            receiver.recv(),
            Ok(ResourceEvent::Reloaded(UntypedResource(_)))
        ));
    }

    #[test]
    fn resource_event_broadcaster_clone() {
        let broadcaster = ResourceEventBroadcaster::new();
        let (sender, receiver) = channel();

        broadcaster.add(sender);
        let broadcaster2 = broadcaster.clone();

        broadcaster2.broadcast(ResourceEvent::Added(UntypedResource::default()));
        assert!(matches!(
            receiver.recv(),
            Ok(ResourceEvent::Added(UntypedResource(_)))
        ));
    }
}
