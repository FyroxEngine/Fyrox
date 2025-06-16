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

use crate::scene::node::UpdateContext;
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
use fyrox_texture::{TextureKind, TextureResource, TextureResourceExtension};
use std::cell::Cell;
use std::ops::{Deref, DerefMut};
use strum_macros::{AsRefStr, EnumString, VariantNames};

#[derive(
    Clone,
    Reflect,
    PartialEq,
    Default,
    Debug,
    Visit,
    TypeUuidProvider,
    AsRefStr,
    EnumString,
    VariantNames,
)]
#[type_uuid(id = "66450303-4f6c-4456-bde5-a7309f25b7ce")]
pub enum UpdateMode {
    #[default]
    Once,
    EachFrame,
}

#[derive(Clone, Reflect, Debug, Visit, ComponentProvider, TypeUuidProvider)]
#[type_uuid(id = "7e0c138f-e371-4045-bd2c-ff5b165c7ee6")]
#[reflect(derived_type = "Node")]
#[visit(optional)]
pub struct ReflectionProbe {
    base: Base,
    #[reflect(min_value = 0.0)]
    pub size: InheritableVariable<Vector3<f32>>,
    pub offset: InheritableVariable<Vector3<f32>>,
    #[reflect(min_value = 0.0)]
    pub z_near: InheritableVariable<f32>,
    #[reflect(min_value = 0.0)]
    pub z_far: InheritableVariable<f32>,
    pub update_mode: InheritableVariable<UpdateMode>,
    #[reflect(hidden)]
    #[visit(skip)]
    pub need_update: bool,
    #[reflect(hidden)]
    #[visit(skip)]
    pub(crate) updated: Cell<bool>,
    #[reflect(hidden)]
    render_target: TextureResource,
}

impl Default for ReflectionProbe {
    fn default() -> Self {
        Self {
            base: Default::default(),
            size: Default::default(),
            offset: Default::default(),
            z_near: 0.001.into(),
            z_far: 128.0.into(),
            update_mode: Default::default(),
            need_update: true,
            updated: Cell::new(false),
            render_target: Default::default(),
        }
    }
}

impl ReflectionProbe {
    pub fn set_resolution(&mut self, resolution: u32) {
        self.render_target = TextureResource::new_cube_render_target(resolution);
    }

    pub fn resolution(&self) -> u32 {
        match self.render_target.data_ref().kind() {
            TextureKind::Cube { size } => size,
            _ => unreachable!(),
        }
    }

    pub fn render_target(&self) -> &TextureResource {
        &self.render_target
    }

    pub fn force_update(&mut self) {
        self.need_update = true;
        self.updated.set(false);
    }
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

    fn update(&mut self, _context: &mut UpdateContext) {
        match *self.update_mode {
            UpdateMode::Once => {
                if self.need_update {
                    self.updated.set(false);
                    self.need_update = false;
                }
            }
            UpdateMode::EachFrame => {
                self.updated.set(false);
            }
        }
    }
}

/// Allows you to create a reflection probe node declaratively.
pub struct ReflectionProbeBuilder {
    base_builder: BaseBuilder,
    size: Vector3<f32>,
    offset: Vector3<f32>,
    z_near: f32,
    z_far: f32,
    resolution: u32,
    update_mode: UpdateMode,
}

impl ReflectionProbeBuilder {
    /// Creates a new reflection probe builder.
    pub fn new(base_builder: BaseBuilder) -> Self {
        Self {
            base_builder,
            size: Default::default(),
            offset: Default::default(),
            z_near: 0.1,
            z_far: 32.0,
            resolution: 512,
            update_mode: Default::default(),
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

    pub fn with_z_near(mut self, z_near: f32) -> Self {
        self.z_near = z_near;
        self
    }

    pub fn with_z_far(mut self, z_far: f32) -> Self {
        self.z_far = z_far;
        self
    }

    pub fn with_resolution(mut self, resolution: u32) -> Self {
        self.resolution = resolution;
        self
    }

    pub fn with_update_mode(mut self, mode: UpdateMode) -> Self {
        self.update_mode = mode;
        self
    }

    /// Creates a new reflection probe node.
    pub fn build_node(self) -> Node {
        Node::new(ReflectionProbe {
            base: self.base_builder.build_base(),
            size: self.size.into(),
            offset: self.offset.into(),
            z_near: self.z_near.into(),
            z_far: self.z_far.into(),
            update_mode: self.update_mode.into(),
            need_update: true,
            updated: Cell::new(false),
            render_target: TextureResource::new_cube_render_target(self.resolution),
        })
    }

    /// Creates a new reflection probe node and adds it to the graph.
    pub fn build(self, graph: &mut Graph) -> Handle<Node> {
        graph.add_node(self.build_node())
    }
}
