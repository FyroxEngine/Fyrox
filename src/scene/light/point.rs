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

use crate::{
    core::{
        inspect::{Inspect, PropertyInfo},
        pool::Handle,
        visitor::{Visit, VisitResult, Visitor},
    },
    scene::{
        graph::Graph,
        light::{BaseLight, BaseLightBuilder, Light},
        node::Node,
    },
};
use std::ops::{Deref, DerefMut};

/// See module docs.
#[derive(Debug, Inspect)]
pub struct PointLight {
    #[inspect(expand)]
    base_light: BaseLight,

    #[inspect(group = "Point Light")]
    shadow_bias: f32,

    #[inspect(group = "Point Light")]
    radius: f32,
}

impl Deref for PointLight {
    type Target = BaseLight;

    fn deref(&self) -> &Self::Target {
        &self.base_light
    }
}

impl DerefMut for PointLight {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base_light
    }
}

impl PointLight {
    /// Sets radius of point light. This parameter also affects radius of spherical
    /// light volume that is used in light scattering.
    #[inline]
    pub fn set_radius(&mut self, radius: f32) {
        self.radius = radius.abs();
    }

    /// Returns radius of point light.
    #[inline]
    pub fn radius(&self) -> f32 {
        self.radius
    }

    /// Sets new shadow bias value. Bias will be used to offset fragment's depth before
    /// compare it with shadow map value, it is used to remove "shadow acne".
    pub fn set_shadow_bias(&mut self, bias: f32) {
        self.shadow_bias = bias;
    }

    /// Returns current value of shadow bias.
    pub fn shadow_bias(&self) -> f32 {
        self.shadow_bias
    }

    /// Creates a raw copy of a point light node.
    pub fn raw_copy(&self) -> Self {
        Self {
            base_light: self.base_light.raw_copy(),
            radius: self.radius,
            shadow_bias: self.shadow_bias,
        }
    }
}

impl Visit for PointLight {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.base_light.visit("BaseLight", visitor)?;
        self.radius.visit("Radius", visitor)?;
        self.shadow_bias.visit("ShadowBias", visitor)?;

        visitor.leave_region()
    }
}

impl Default for PointLight {
    fn default() -> Self {
        Self {
            base_light: Default::default(),
            shadow_bias: 0.025,
            radius: 10.0,
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
            radius: self.radius,
            shadow_bias: self.shadow_bias,
        }
    }

    /// Builds new instance of point light node.
    pub fn build_node(self) -> Node {
        Node::Light(Light::Point(self.build_point_light()))
    }

    /// Builds new instance of point light and adds it to the graph.
    pub fn build(self, graph: &mut Graph) -> Handle<Node> {
        graph.add_node(self.build_node())
    }
}
