use crate::core::visitor::{Visit, VisitResult, Visitor};
use crate::scene2d::base::Base;
use std::ops::{Deref, DerefMut};

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

impl Light {
    fn id(&self) -> u32 {
        match self {
            Light::Point(_) => 0,
            Light::Spot(_) => 1,
        }
    }

    fn from_id(id: u32) -> Result<Self, String> {
        match id {
            0 => Ok(Self::Point(Default::default())),
            1 => Ok(Self::Spot(Default::default())),
            _ => Err(format!("Invalid light id {}!", id)),
        }
    }
}

impl Visit for Light {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        let mut id = self.id();
        id.visit("Id", visitor)?;
        if visitor.is_reading() {
            *self = Self::from_id(id)?;
        }

        visitor.leave_region()
    }
}

impl Default for Light {
    fn default() -> Self {
        Self::Spot(Default::default())
    }
}

#[derive(Default)]
pub struct BaseLight {
    base: Base,
}

impl Visit for BaseLight {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.base.visit("Base", visitor)?;

        visitor.leave_region()
    }
}

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

impl Visit for PointLight {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.base_light.visit("BaseLight", visitor)?;
        self.radius.visit("Radius", visitor)?;

        visitor.leave_region()
    }
}

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

impl Visit for SpotLight {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.base_light.visit("BaseLight", visitor)?;
        self.radius.visit("Radius", visitor)?;
        self.hotspot.visit("Hotspot", visitor)?;
        self.delta.visit("Delta", visitor)?;

        visitor.leave_region()
    }
}
