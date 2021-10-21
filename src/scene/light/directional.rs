//! Directional light is a light source with parallel rays, it has
//! excellent example in real life - Sun. It does not have position,
//! only direction which defined by parent light scene node.
//!
//! # Notes
//!
//! Current directional light does *not* support shadows, it is still
//! on list of features that should be implemented.

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
#[derive(Default, Debug, Inspect)]
pub struct DirectionalLight {
    base_light: BaseLight,
}

impl From<BaseLight> for DirectionalLight {
    fn from(base_light: BaseLight) -> Self {
        Self { base_light }
    }
}

impl Deref for DirectionalLight {
    type Target = BaseLight;

    fn deref(&self) -> &Self::Target {
        &self.base_light
    }
}

impl DerefMut for DirectionalLight {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base_light
    }
}

impl Visit for DirectionalLight {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.base_light.visit("BaseLight", visitor)?;

        visitor.leave_region()
    }
}

impl DirectionalLight {
    /// Creates a raw copy of a directional light node.
    pub fn raw_copy(&self) -> Self {
        Self {
            base_light: self.base_light.raw_copy(),
        }
    }
}

/// Allows you to build directional light in declarative manner.
pub struct DirectionalLightBuilder {
    base_light_builder: BaseLightBuilder,
}

impl DirectionalLightBuilder {
    /// Creates new builder instance.
    pub fn new(base_light_builder: BaseLightBuilder) -> Self {
        Self { base_light_builder }
    }

    /// Creates new instance of directional light.
    pub fn build_directional_light(self) -> DirectionalLight {
        DirectionalLight {
            base_light: self.base_light_builder.build(),
        }
    }

    /// Creates new instance of directional light node.
    pub fn build_node(self) -> Node {
        Node::Light(Light::Directional(self.build_directional_light()))
    }

    /// Creates new instance of directional light and adds it to the graph.
    pub fn build(self, graph: &mut Graph) -> Handle<Node> {
        graph.add_node(self.build_node())
    }
}
