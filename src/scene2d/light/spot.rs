use crate::{
    core::{pool::Handle, visitor::prelude::*},
    scene2d::{
        base::Base,
        graph::Graph,
        light::{BaseLight, BaseLightBuilder, Light},
        node::Node,
    },
};
use std::ops::{Deref, DerefMut};

#[derive(Visit)]
pub struct SpotLight {
    base_light: BaseLight,
    radius: f32,
    hotspot: f32,
    delta: f32,
}

impl Deref for SpotLight {
    type Target = Base;

    fn deref(&self) -> &Self::Target {
        &self.base_light.base
    }
}

impl DerefMut for SpotLight {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base_light.base
    }
}

impl Default for SpotLight {
    fn default() -> Self {
        Self {
            base_light: Default::default(),
            radius: 10.0,
            hotspot: 90.0f32.to_radians(),
            delta: 5.0f32.to_radians(),
        }
    }
}

pub struct SpotLightBuilder {
    base_light_builder: BaseLightBuilder,
    radius: f32,
    hotspot: f32,
    delta: f32,
}

impl SpotLightBuilder {
    pub fn new(base_light_builder: BaseLightBuilder) -> Self {
        Self {
            base_light_builder,
            radius: 10.0,
            hotspot: 90.0f32.to_radians(),
            delta: 5.0f32.to_radians(),
        }
    }

    pub fn with_radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    pub fn build(self, graph: &mut Graph) -> Handle<Node> {
        graph.add_node(Node::Light(Light::Spot(SpotLight {
            base_light: self.base_light_builder.build(),
            radius: self.radius,
            hotspot: self.hotspot,
            delta: self.delta,
        })))
    }
}
