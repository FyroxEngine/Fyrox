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

//! Point light can be represented as light bulb which hangs on wire - it is
//! spherical light source which emits light in all directions. It has single
//! parameter - radius at which intensity will be zero. Intensity of light will
//! be calculated using inverse square root law.
//!
//! # Light scattering
//!
//! Point light support light scattering feature - it means that you'll see light
//! volume as well as lighted surfaces. Simple example from real life: light bulb
//! in the fog. This effect significantly improves perception of light, but should
//! be used carefully with sane values of light scattering, otherwise you'll get
//! bright glowing sphere instead of slightly visible light volume.
//!
//! # Performance notes
//!
//! Point lights supports shadows, but keep in mind - they're very expensive and
//! can easily ruin performance of your game, especially on low-end hardware. Light
//! scattering is relatively heavy too.

use crate::scene::base::BaseBuilder;
use crate::scene::node::constructor::NodeConstructor;
use crate::{
    core::{
        color::Color,
        math::aabb::AxisAlignedBoundingBox,
        pool::Handle,
        reflect::prelude::*,
        type_traits::prelude::*,
        uuid::{uuid, Uuid},
        variable::InheritableVariable,
        visitor::{Visit, VisitResult, Visitor},
    },
    scene::{
        base::Base,
        debug::SceneDrawingContext,
        graph::Graph,
        light::{BaseLight, BaseLightBuilder},
        node::{Node, NodeTrait},
    },
};
use fyrox_graph::constructor::ConstructorProvider;
use fyrox_graph::BaseSceneGraph;
use std::ops::{Deref, DerefMut};

/// See module docs.
#[derive(Debug, Reflect, Clone, Visit, ComponentProvider)]
#[reflect(derived_type = "Node")]
pub struct PointLight {
    #[component(include)]
    base_light: BaseLight,

    #[reflect(min_value = 0.0, step = 0.001)]
    #[reflect(setter = "set_shadow_bias")]
    shadow_bias: InheritableVariable<f32>,

    #[reflect(min_value = 0.0, step = 0.1)]
    #[reflect(setter = "set_radius")]
    radius: InheritableVariable<f32>,
}

impl Deref for PointLight {
    type Target = Base;

    fn deref(&self) -> &Self::Target {
        &self.base_light.base
    }
}

impl DerefMut for PointLight {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base_light.base
    }
}

impl TypeUuidProvider for PointLight {
    fn type_uuid() -> Uuid {
        uuid!("c81dcc31-7cb9-465f-abd9-b385ac6f4d37")
    }
}

impl PointLight {
    /// Returns a reference to base light.    
    pub fn base_light_ref(&self) -> &BaseLight {
        &self.base_light
    }

    /// Returns a reference to base light.
    pub fn base_light_mut(&mut self) -> &mut BaseLight {
        &mut self.base_light
    }

    /// Sets radius of point light. This parameter also affects radius of spherical
    /// light volume that is used in light scattering.
    #[inline]
    pub fn set_radius(&mut self, radius: f32) -> f32 {
        self.radius.set_value_and_mark_modified(radius.abs())
    }

    /// Returns radius of point light.
    #[inline]
    pub fn radius(&self) -> f32 {
        *self.radius
    }

    /// Sets new shadow bias value. Bias will be used to offset fragment's depth before
    /// compare it with shadow map value, it is used to remove "shadow acne".
    pub fn set_shadow_bias(&mut self, bias: f32) -> f32 {
        self.shadow_bias.set_value_and_mark_modified(bias)
    }

    /// Returns current value of shadow bias.
    pub fn shadow_bias(&self) -> f32 {
        *self.shadow_bias
    }
}

impl ConstructorProvider<Node, Graph> for PointLight {
    fn constructor() -> NodeConstructor {
        NodeConstructor::new::<Self>()
            .with_variant("Point Light", |_| {
                PointLightBuilder::new(BaseLightBuilder::new(
                    BaseBuilder::new().with_name("PointLight"),
                ))
                .with_radius(10.0)
                .build_node()
                .into()
            })
            .with_group("Light")
    }
}

impl NodeTrait for PointLight {
    fn local_bounding_box(&self) -> AxisAlignedBoundingBox {
        AxisAlignedBoundingBox::from_radius(*self.radius)
    }

    fn world_bounding_box(&self) -> AxisAlignedBoundingBox {
        // Discard scaling part, light emission distance does not affected by scaling.
        self.local_bounding_box()
            .transform(&self.global_transform_without_scaling())
    }

    fn id(&self) -> Uuid {
        Self::type_uuid()
    }

    fn debug_draw(&self, ctx: &mut SceneDrawingContext) {
        ctx.draw_wire_sphere(self.global_position(), self.radius(), 30, Color::GREEN);
    }
}

impl Default for PointLight {
    fn default() -> Self {
        Self {
            base_light: Default::default(),
            shadow_bias: InheritableVariable::new_modified(0.025),
            radius: InheritableVariable::new_modified(10.0),
        }
    }
}

/// Allows you to build point light in declarative manner.
pub struct PointLightBuilder {
    base_light_builder: BaseLightBuilder,
    shadow_bias: f32,
    radius: f32,
}

impl PointLightBuilder {
    /// Creates new builder instance.
    pub fn new(base_light_builder: BaseLightBuilder) -> Self {
        Self {
            base_light_builder,
            shadow_bias: 0.025,
            radius: 10.0,
        }
    }

    /// Sets desired radius.
    pub fn with_radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    /// Sets desired shadow bias.
    pub fn with_shadow_bias(mut self, bias: f32) -> Self {
        self.shadow_bias = bias;
        self
    }

    /// Builds new instance of point light.
    pub fn build_point_light(self) -> PointLight {
        PointLight {
            base_light: self.base_light_builder.build(),
            radius: self.radius.into(),
            shadow_bias: self.shadow_bias.into(),
        }
    }

    /// Builds new instance of point light node.
    pub fn build_node(self) -> Node {
        Node::new(self.build_point_light())
    }

    /// Builds new instance of point light and adds it to the graph.
    pub fn build(self, graph: &mut Graph) -> Handle<Node> {
        graph.add_node(self.build_node())
    }
}
