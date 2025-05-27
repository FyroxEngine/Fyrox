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

#![allow(missing_docs)] // TODO

use crate::{
    core::{
        algebra::Vector3,
        math::aabb::AxisAlignedBoundingBox,
        pool::Handle,
        reflect::prelude::*,
        type_traits::prelude::*,
        uuid::{uuid, Uuid},
        variable::InheritableVariable,
        visitor::prelude::*,
    },
    graph::{constructor::ConstructorProvider, BaseSceneGraph},
    scene::{
        base::{Base, BaseBuilder},
        graph::Graph,
        node::{constructor::NodeConstructor, Node, NodeTrait},
    },
};
use std::ops::{Deref, DerefMut};

#[derive(Clone, Reflect, Default, Debug, Visit, ComponentProvider, TypeUuidProvider)]
#[type_uuid(id = "7e0c138f-e371-4045-bd2c-ff5b165c7ee6")]
#[reflect(derived_type = "Node")]
pub struct ReflectionProbe {
    base: Base,
    #[reflect(min_value = 0.0)]
    pub size: InheritableVariable<Vector3<f32>>,
    pub offset: InheritableVariable<Vector3<f32>>,
}

impl Deref for ReflectionProbe {
    type Target = Base;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for ReflectionProbe {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl ConstructorProvider<Node, Graph> for ReflectionProbe {
    fn constructor() -> NodeConstructor {
        NodeConstructor::new::<Self>().with_variant("Reflection Probe", |_| {
            ReflectionProbeBuilder::new(BaseBuilder::new().with_name("Reflection Probe"))
                .build_node()
                .into()
        })
    }
}

impl NodeTrait for ReflectionProbe {
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

/// Allows you to create a reflection probe node declaratively.
pub struct ReflectionProbeBuilder {
    base_builder: BaseBuilder,
    size: Vector3<f32>,
    offset: Vector3<f32>,
}

impl ReflectionProbeBuilder {
    /// Creates a new reflection probe builder.
    pub fn new(base_builder: BaseBuilder) -> Self {
        Self {
            base_builder,
            size: Default::default(),
            offset: Default::default(),
        }
    }

    /// Sets the desired size of the reflection probe.
    pub fn with_size(mut self, size: Vector3<f32>) -> Self {
        self.size = size;
        self
    }

    /// Sets the desired offset of the reflection probe.
    pub fn with_offset(mut self, offset: Vector3<f32>) -> Self {
        self.size = offset;
        self
    }

    /// Creates a new reflection probe node.
    pub fn build_node(self) -> Node {
        Node::new(ReflectionProbe {
            base: self.base_builder.build_base(),
            size: self.size.into(),
            offset: self.offset.into(),
        })
    }

    /// Creates a new reflection probe node and adds it to the graph.
    pub fn build(self, graph: &mut Graph) -> Handle<Node> {
        graph.add_node(self.build_node())
    }
}
