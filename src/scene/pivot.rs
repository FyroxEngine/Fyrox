use crate::scene::node::TypeUuidProvider;
use crate::{
    core::{
        inspect::{Inspect, PropertyInfo},
        math::aabb::AxisAlignedBoundingBox,
        pool::Handle,
        uuid::Uuid,
        visitor::prelude::*,
    },
    engine::resource_manager::ResourceManager,
    scene::{
        base::{Base, BaseBuilder},
        graph::Graph,
        node::{Node, NodeTrait},
        variable::InheritError,
    },
};
use fxhash::FxHashMap;
use std::{
    ops::{Deref, DerefMut},
    str::FromStr,
};

#[derive(Clone, Inspect, Default, Debug)]
pub struct Pivot {
    base: Base,
}

impl Visit for Pivot {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        self.base.visit(name, visitor)
    }
}

impl Deref for Pivot {
    type Target = Base;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl TypeUuidProvider for Pivot {
    fn type_uuid() -> Uuid {
        Uuid::from_str("dd2ecb96-b1f4-4ee0-943b-2a4d1844e3bb").unwrap()
    }
}

impl DerefMut for Pivot {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl NodeTrait for Pivot {
    crate::impl_query_component!();

    fn local_bounding_box(&self) -> AxisAlignedBoundingBox {
        self.base.local_bounding_box()
    }

    fn world_bounding_box(&self) -> AxisAlignedBoundingBox {
        self.base.world_bounding_box()
    }

    fn inherit(&mut self, parent: &Node) -> Result<(), InheritError> {
        self.base.inherit_properties(parent)
    }

    fn reset_inheritable_properties(&mut self) {
        self.base.reset_inheritable_properties()
    }

    fn restore_resources(&mut self, resource_manager: ResourceManager) {
        self.base.restore_resources(resource_manager)
    }

    fn remap_handles(&mut self, old_new_mapping: &FxHashMap<Handle<Node>, Handle<Node>>) {
        self.base.remap_handles(old_new_mapping)
    }

    fn id(&self) -> Uuid {
        Self::type_uuid()
    }
}

pub struct PivotBuilder {
    base_builder: BaseBuilder,
}

impl PivotBuilder {
    pub fn new(base_builder: BaseBuilder) -> Self {
        Self { base_builder }
    }

    pub fn build_node(self) -> Node {
        Node::new(Pivot {
            base: self.base_builder.build_base(),
        })
    }

    pub fn build(self, graph: &mut Graph) -> Handle<Node> {
        graph.add_node(self.build_node())
    }
}
