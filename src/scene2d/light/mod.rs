use crate::{
    core::{
        color::Color,
        inspect::{Inspect, PropertyInfo},
        visitor::prelude::*,
    },
    scene2d::{
        base::{Base, BaseBuilder},
        light::{point::PointLight, spot::SpotLight},
    },
};
use std::ops::{Deref, DerefMut};

pub mod point;
pub mod spot;

#[derive(Visit, Debug)]
pub enum Light {
    Point(PointLight),
    Spot(SpotLight),
}

impl Inspect for Light {
    fn properties(&self) -> Vec<PropertyInfo<'_>> {
        match self {
            Light::Point(v) => v.properties(),
            Light::Spot(v) => v.properties(),
        }
    }
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

impl Light {
    pub fn raw_copy(&self) -> Self {
        match self {
            Light::Point(v) => Light::Point(v.raw_copy()),
            Light::Spot(v) => Light::Spot(v.raw_copy()),
        }
    }
}

impl Default for Light {
    fn default() -> Self {
        Self::Spot(Default::default())
    }
}

#[derive(Default, Visit, Inspect, Debug)]
pub struct BaseLight {
    base: Base,
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

    pub fn raw_copy(&self) -> Self {
        Self {
            base: self.base.raw_copy(),
            color: self.color,
        }
    }
}

pub struct BaseLightBuilder {
    base_builder: BaseBuilder,
    color: Color,
}

impl BaseLightBuilder {
    pub fn new(base_builder: BaseBuilder) -> Self {
        Self {
            base_builder,
            color: Color::WHITE,
        }
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn build(self) -> BaseLight {
        BaseLight {
            base: self.base_builder.build_base(),
            color: self.color,
        }
    }
}
