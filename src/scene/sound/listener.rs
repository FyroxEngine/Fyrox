//! Listener represents directional microphone-like device. It receives sound from surroundings
//! and plays it through output device (headphones, speakers, etc.).
//!
//! See [`Listener`] docs for more info.

use crate::{
    core::{
        inspect::{Inspect, PropertyInfo},
        math::{aabb::AxisAlignedBoundingBox, Matrix4Ext},
        pool::Handle,
        reflect::Reflect,
        uuid::{uuid, Uuid},
        visitor::prelude::*,
    },
    engine::resource_manager::ResourceManager,
    scene::{
        base::{Base, BaseBuilder},
        graph::Graph,
        node::{Node, NodeTrait, SyncContext, TypeUuidProvider},
    },
};
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
#[derive(Visit, Inspect, Reflect, Default, Clone, Debug)]
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

impl TypeUuidProvider for Listener {
    fn type_uuid() -> Uuid {
        uuid!("2c7dabc1-5666-4256-b020-01532701e4c6")
    }
}

impl NodeTrait for Listener {
    crate::impl_query_component!();

    /// Returns local bounding box of the listener, since listener cannot have any bounds -
    /// returned bounding box is collapsed into a point.
    fn local_bounding_box(&self) -> AxisAlignedBoundingBox {
        AxisAlignedBoundingBox {
            min: Default::default(),
            max: Default::default(),
        }
    }

    fn world_bounding_box(&self) -> AxisAlignedBoundingBox {
        self.base.world_bounding_box()
    }

    fn restore_resources(&mut self, resource_manager: ResourceManager) {
        self.base.restore_resources(resource_manager);
    }

    fn id(&self) -> Uuid {
        Self::type_uuid()
    }

    fn sync_native(&self, _self_handle: Handle<Node>, context: &mut SyncContext) {
        let mut state = context.sound_context.native.state();
        let native = state.listener_mut();
        native.set_position(self.global_position());
        native.set_basis(self.global_transform().basis());
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

    /// Creates [`Listener`] node.
    pub fn build_node(self) -> Node {
        Node::new(self.build_listener())
    }

    /// Creates [`Listener`] node and adds it to the scene graph.
    pub fn build(self, graph: &mut Graph) -> Handle<Node> {
        graph.add_node(self.build_node())
    }
}

#[cfg(test)]
mod test {
    use crate::core::reflect::Reflect;
    use crate::core::variable::try_inherit_properties;
    use crate::scene::{
        base::{test::check_inheritable_properties_equality, BaseBuilder},
        sound::listener::{Listener, ListenerBuilder},
    };

    #[test]
    fn test_listener_inheritance() {
        let parent = ListenerBuilder::new(BaseBuilder::new()).build_node();

        let mut child = ListenerBuilder::new(BaseBuilder::new()).build_listener();

        try_inherit_properties(child.as_reflect_mut(), parent.as_reflect()).unwrap();

        let parent = parent.cast::<Listener>().unwrap();

        check_inheritable_properties_equality(&child.base, &parent.base);
    }
}
