use crate::{
    core::{pool::Handle, visitor::prelude::*},
    scene2d::{
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
    hotspot_angle: f32,
    falloff_angle_delta: f32,
}

impl SpotLight {
    pub fn radius(&self) -> f32 {
        self.radius
    }

    pub fn hot_spot_angle(&self) -> f32 {
        self.hotspot_angle
    }

    pub fn falloff_angle_delta(&self) -> f32 {
        self.falloff_angle_delta
    }

    pub fn half_full_cone_angle_cos(&self) -> f32 {
        ((self.falloff_angle_delta + self.hotspot_angle) * 0.5).cos()
    }

    pub fn half_hotspot_cone_angle(&self) -> f32 {
        (self.hotspot_angle * 0.5).cos()
    }

    pub fn raw_copy(&self) -> Self {
        Self {
            base_light: self.base_light.raw_copy(),
            radius: self.radius,
            hotspot_angle: self.hotspot_angle,
            falloff_angle_delta: self.falloff_angle_delta,
        }
    }
}

impl Deref for SpotLight {
    type Target = BaseLight;

    fn deref(&self) -> &Self::Target {
        &self.base_light
    }
}

impl DerefMut for SpotLight {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base_light
    }
}

impl Default for SpotLight {
    fn default() -> Self {
        Self {
            base_light: Default::default(),
            radius: 10.0,
            hotspot_angle: 90.0f32.to_radians(),
            falloff_angle_delta: 10.0f32.to_radians(),
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

    pub fn with_hotspot_angle(mut self, angle: f32) -> Self {
        self.hotspot = angle;
        self
    }

    pub fn with_angle_delta(mut self, delta: f32) -> Self {
        self.delta = delta.abs();
        self
    }

    pub fn build(self, graph: &mut Graph) -> Handle<Node> {
        graph.add_node(Node::Light(Light::Spot(SpotLight {
            base_light: self.base_light_builder.build(),
            radius: self.radius,
            hotspot_angle: self.hotspot,
            falloff_angle_delta: self.delta,
        })))
    }
}
