use crate::{
    core::{color::Color, visitor::prelude::*},
    scene2d::{
        base::{Base, BaseBuilder},
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
    type Target = BaseLight;

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
    color: Color,
}

impl Deref for BaseLight {
    type Target = Base;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for BaseLight {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl BaseLight {
    pub fn color(&self) -> Color {
        self.color
    }

    pub fn set_color(&mut self, color: Color) {
        self.color = color;
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
}

pub struct BaseLightBuilder {
    base_builder: BaseBuilder,
    enabled: bool,
    color: Color,
}

impl BaseLightBuilder {
    pub fn new(base_builder: BaseBuilder) -> Self {
        Self {
            base_builder,
            enabled: true,
            color: Color::WHITE,
        }
    }

    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn build(self) -> BaseLight {
        BaseLight {
            base: self.base_builder.build_base(),
            enabled: self.enabled,
            color: self.color,
        }
    }
}
