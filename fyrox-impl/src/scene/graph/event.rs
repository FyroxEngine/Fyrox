//! Graph event broadcaster allows you to receive graph events such as node deletion or addition.
//! Check [GraphEventBroadcaster::subscribe] for examples.

use crate::{core::pool::Handle, scene::node::Node};
use std::{
    fmt::{Debug, Formatter},
    sync::mpsc::Sender,
};

/// An event that happened in a graph.
#[derive(Clone, PartialEq, Debug, Eq)]
pub enum GraphEvent {
    /// A node was added.
    Added(Handle<Node>),
    /// A node was removed.
    Removed(Handle<Node>),
}

/// Graph event broadcaster allows you to receive graph events such as node deletion or addition.
/// Check [GraphEventBroadcaster::subscribe] for examples.
#[derive(Default)]
pub struct GraphEventBroadcaster {
    senders: Vec<Sender<GraphEvent>>,
}

impl Debug for GraphEventBroadcaster {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "GraphEventBroadcaster has {} senders.",
            self.senders.len()
        )
    }
}

impl GraphEventBroadcaster {
    /// Adds new subscriber, an instance of [Sender].
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use std::sync::mpsc::channel;
    /// # use fyrox_impl::scene::base::BaseBuilder;
    /// # use fyrox_impl::scene::graph::event::GraphEvent;
    /// # use fyrox_impl::scene::graph::Graph;
    /// # use fyrox_impl::scene::pivot::PivotBuilder;
    /// # use fyrox_impl::graph::BaseSceneGraph;
    ///
    /// let mut graph = Graph::new();
    ///
    /// let (tx, rx) = channel();
    /// graph.event_broadcaster.subscribe(tx);
    ///
    /// // Add a node
    /// let handle = PivotBuilder::new(BaseBuilder::new()).build(&mut graph);
    ///
    /// assert_eq!(rx.recv(), Ok(GraphEvent::Added(handle)));
    ///
    /// graph.remove_node(handle);
    ///
    /// assert_eq!(rx.recv(), Ok(GraphEvent::Removed(handle)));
    ///
    /// ```
    pub fn subscribe(&mut self, sender: Sender<GraphEvent>) {
        self.senders.push(sender);
    }

    pub(crate) fn broadcast(&mut self, event: GraphEvent) {
        self.senders
            .retain_mut(|sender| sender.send(event.clone()).is_ok());
    }
}
