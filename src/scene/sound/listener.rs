//! Listener represents directional microphone-like device. It receives sound from surroundings
//! and plays it through output device (headphones, speakers, etc.).
//!
//! See [`Listener`] docs for more info.

use crate::{
    core::{
        inspect::{Inspect, PropertyInfo},
        pool::Handle,
        visitor::prelude::*,
    },
    scene::{
        base::{Base, BaseBuilder},
        graph::Graph,
        node::Node,
        variable::InheritError,
    },
};
use fyrox_core::math::aabb::AxisAlignedBoundingBox;
use std::ops::{Deref, DerefMut};

/// Listener represents directional microphone-like device. It receives sound from surroundings
/// and plays it through output device (headphones, speakers, etc.). Orientation of the listener
/// influences overall perception of sound sources as if listener would be human head. Rotation
/// basis's side-vector defines ear axis where -X is for left ear and +X for right. Look vector (Z+)
/// defines "face" of the listener.
///
/// There can be only one listener at a time, if you create multiple listeners, the last one will
/// have priority.
///
/// Usually listener is attached to the main camera, however there might be some other rare cases
/// and you can attach listener to any node you like.
///
/// 2D sound sources (with spatial blend == 0.0) are not influenced by listener's position and
/// orientation.
#[derive(Visit, Inspect, Default, Debug)]
pub struct Listener {
    base: Base,
}

impl Deref for Listener {
    type Target = Base;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for Listener {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl Listener {
    /// Creates raw copy of the listener.
    pub fn raw_copy(&self) -> Self {
        Self {
            base: self.base.raw_copy(),
        }
    }

    // Prefab inheritance resolving.
    pub(crate) fn inherit(&mut self, parent: &Node) -> Result<(), InheritError> {
        self.base.inherit_properties(parent)
    }

    pub(crate) fn reset_inheritable_properties(&mut self) {
        self.base.reset_inheritable_properties();
    }

    /// Returns local bounding box of the listener, since listener cannot have any bounds -
    /// returned bounding box is collapsed into a point.
    pub fn local_bounding_box(&self) -> AxisAlignedBoundingBox {
        AxisAlignedBoundingBox {
            min: Default::default(),
            max: Default::default(),
        }
    }
}

/// Allows you to create listener in declarative manner.
pub struct ListenerBuilder {
    base_builder: BaseBuilder,
}

impl ListenerBuilder {
    /// Creates new listner builder.
    pub fn new(base_builder: BaseBuilder) -> Self {
        Self { base_builder }
    }

    /// Creates listener instance.
    pub fn build_listener(self) -> Listener {
        Listener {
            base: self.base_builder.build_base(),
        }
    }

    /// Creates [`Node::Listener`] node.
    pub fn build_node(self) -> Node {
        Node::Listener(self.build_listener())
    }

    /// Creates [`Node::Listener`] node and adds it to the scene graph.
    pub fn build(self, graph: &mut Graph) -> Handle<Node> {
        graph.add_node(self.build_node())
    }
}

#[cfg(test)]
mod test {
    use crate::scene::{
        base::{test::check_inheritable_properties_equality, BaseBuilder},
        node::Node,
        sound::listener::ListenerBuilder,
    };

    #[test]
    fn test_listener_inheritance() {
        let parent = ListenerBuilder::new(BaseBuilder::new()).build_node();

        let mut child = ListenerBuilder::new(BaseBuilder::new()).build_listener();

        child.inherit(&parent).unwrap();

        if let Node::Listener(parent) = parent {
            check_inheritable_properties_equality(&child.base, &parent.base);
        } else {
            unreachable!()
        }
    }
}
