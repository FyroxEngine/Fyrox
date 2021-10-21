use crate::{
    core::{
        inspect::{Inspect, PropertyInfo},
        pool::Handle,
        visitor::prelude::*,
    },
    scene2d::{
        graph::Graph,
        light::{BaseLight, BaseLightBuilder, Light},
        node::Node,
    },
};
use std::ops::{Deref, DerefMut};

#[derive(Visit, Inspect, Debug)]
pub struct PointLight {
    base_light: BaseLight,
    radius: f32,
}

impl PointLight {
    pub fn radius(&self) -> f32 {
        self.radius
    }

    pub fn raw_copy(&self) -> Self {
        Self {
            base_light: self.base_light.raw_copy(),
            radius: self.radius,
        }
    }
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

impl Default for PointLight {
    fn default() -> Self {
        Self {
            base_light: Default::default(),
            radius: 10.0,
        }
    }
}

pub struct PointLightBuilder {
    base_light_builder: BaseLightBuilder,
    radius: f32,
}

impl PointLightBuilder {
    pub fn new(base_light_builder: BaseLightBuilder) -> Self {
        Self {
            base_light_builder,
            radius: 10.0,
        }
    }

    pub fn with_radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    pub fn build(self, graph: &mut Graph) -> Handle<Node> {
        graph.add_node(Node::Light(Light::Point(PointLight {
            base_light: self.base_light_builder.build(),
            radius: self.radius,
        })))
    }
}
