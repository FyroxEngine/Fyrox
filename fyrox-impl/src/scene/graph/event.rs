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
