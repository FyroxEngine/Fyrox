#![allow(missing_docs)] // TODO

use crate::scene::base::BaseBuilder;
use crate::scene::graph::Graph;
use crate::scene::node::Node;
use crate::{
    core::{
        math::aabb::AxisAlignedBoundingBox,
        reflect::prelude::*,
        uuid::{uuid, Uuid},
        visitor::prelude::*,
    },
    scene::{
        base::Base,
        node::{NodeTrait, TypeUuidProvider},
    },
    utils::navmesh::Navmesh,
};
use fyrox_core::pool::Handle;
use std::ops::{Deref, DerefMut};

#[derive(Debug, Clone, Visit, Reflect, Default)]
pub struct NavigationalMesh {
    base: Base,
    #[reflect(hidden)]
    navmesh: Navmesh,
}

impl TypeUuidProvider for NavigationalMesh {
    fn type_uuid() -> Uuid {
        uuid!("d0ce963c-b50a-4707-bd21-af6dc0d1c668")
    }
}

impl Deref for NavigationalMesh {
    type Target = Base;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for NavigationalMesh {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl NodeTrait for NavigationalMesh {
    crate::impl_query_component!();

    fn local_bounding_box(&self) -> AxisAlignedBoundingBox {
        self.base.local_bounding_box()
    }

    fn world_bounding_box(&self) -> AxisAlignedBoundingBox {
        self.base.world_bounding_box()
    }

    fn id(&self) -> Uuid {
        Self::type_uuid()
    }
}

impl NavigationalMesh {
    pub fn navmesh_ref(&self) -> &Navmesh {
        &self.navmesh
    }

    pub fn navmesh_mut(&mut self) -> &mut Navmesh {
        &mut self.navmesh
    }
}

pub struct NavigationalMeshBuilder {
    base_builder: BaseBuilder,
    navmesh: Navmesh,
}

impl NavigationalMeshBuilder {
    pub fn new(base_builder: BaseBuilder) -> Self {
        Self {
            base_builder,
            navmesh: Default::default(),
        }
    }

    pub fn with_navmesh(mut self, navmesh: Navmesh) -> Self {
        self.navmesh = navmesh;
        self
    }

    fn build_navigational_mesh(self) -> NavigationalMesh {
        NavigationalMesh {
            base: self.base_builder.build_base(),
            navmesh: self.navmesh,
        }
    }

    /// Creates new navigational mesh instance.
    pub fn build_node(self) -> Node {
        Node::new(self.build_navigational_mesh())
    }

    /// Creates new navigational mesh instance and adds it to the graph.
    pub fn build(self, graph: &mut Graph) -> Handle<Node> {
        graph.add_node(self.build_node())
    }
}
