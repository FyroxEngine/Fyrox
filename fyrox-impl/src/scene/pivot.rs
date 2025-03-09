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

//! A simplest possible node which represents point in space.
use crate::scene::node::constructor::NodeConstructor;
use crate::{
    core::{
        math::aabb::AxisAlignedBoundingBox,
        pool::Handle,
        reflect::prelude::*,
        type_traits::prelude::*,
        uuid::{uuid, Uuid},
        visitor::prelude::*,
    },
    scene::{
        base::{Base, BaseBuilder},
        graph::Graph,
        node::{Node, NodeTrait},
    },
};
use fyrox_graph::constructor::ConstructorProvider;
use fyrox_graph::BaseSceneGraph;
use std::ops::{Deref, DerefMut};

/// A simplest possible node which represents point in space.
#[derive(Clone, Reflect, Default, Debug, ComponentProvider)]
#[reflect(derived_type = "Node")]
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
        uuid!("dd2ecb96-b1f4-4ee0-943b-2a4d1844e3bb")
    }
}

impl DerefMut for Pivot {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl ConstructorProvider<Node, Graph> for Pivot {
    fn constructor() -> NodeConstructor {
        NodeConstructor::new::<Self>().with_variant("Pivot", |_| {
            PivotBuilder::new(BaseBuilder::new().with_name("Pivot"))
                .build_node()
                .into()
        })
    }
}

impl NodeTrait for Pivot {
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

/// Allows you to create pivot node in declarative manner.
pub struct PivotBuilder {
    base_builder: BaseBuilder,
}

impl PivotBuilder {
    /// Creates new pivot builder.
    pub fn new(base_builder: BaseBuilder) -> Self {
        Self { base_builder }
    }

    /// Creates new Pivot node.
    pub fn build_node(self) -> Node {
        Node::new(Pivot {
            base: self.base_builder.build_base(),
        })
    }

    /// Creates new Pivot node and adds it to the graph.
    pub fn build(self, graph: &mut Graph) -> Handle<Node> {
        graph.add_node(self.build_node())
    }
}
