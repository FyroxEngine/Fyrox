use crate::{core::visitor::prelude::*, scene2d::base::Base};
use std::ops::{Deref, DerefMut};

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
}

#[derive(Visit)]
pub struct PointLight {
    base_light: BaseLight,
    radius: f32,
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

impl Default for PointLight {
    fn default() -> Self {
        Self {
            base_light: Default::default(),
            radius: 10.0,
        }
    }
}

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
