use crate::scene2d::base::BaseBuilder;
use crate::{
    core::visitor::prelude::*,
    scene2d::{
        base::Base,
        light::{point::PointLight, spot::SpotLight},
    },
};
use std::ops::{Deref, DerefMut};

pub mod point;
pub mod spot;

#[derive(Visit)]
pub enum Light {
    Point(PointLight),
    Spot(SpotLight),
}

impl Deref for Light {
    type Target = Base;

    fn deref(&self) -> &Self::Target {
        match self {
            Light::Point(v) => v.deref(),
            Light::Spot(v) => v.deref(),
        }
    }
}

impl DerefMut for Light {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Light::Point(v) => v.deref_mut(),
            Light::Spot(v) => v.deref_mut(),
        }
    }
}

impl Default for Light {
    fn default() -> Self {
        Self::Spot(Default::default())
    }
}

#[derive(Default, Visit)]
pub struct BaseLight {
    base: Base,
    enabled: bool,
}

pub struct BaseLightBuilder {
    base_builder: BaseBuilder,
    enabled: bool,
}

impl BaseLightBuilder {
    pub fn new(base_builder: BaseBuilder) -> Self {
        Self {
            base_builder,
            enabled: true,
        }
    }

    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn build(self) -> BaseLight {
        BaseLight {
            base: self.base_builder.build_base(),
            enabled: self.enabled,
        }
    }
}
