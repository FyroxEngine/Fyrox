//! Emitter is an enum over all possible emitter types, they all must
//! use BaseEmitter which contains base functionality.

use crate::{
    core::{reflect::prelude::*, visitor::prelude::*},
    scene::particle_system::{
        emitter::{
            base::BaseEmitter, cuboid::CuboidEmitter, cylinder::CylinderEmitter,
            sphere::SphereEmitter,
        },
        Particle, ParticleSystemRng,
    },
};
use fyrox_core::uuid_provider;
use std::ops::{Deref, DerefMut};
use strum_macros::{AsRefStr, EnumString, VariantNames};

pub mod base;
pub mod cuboid;
pub mod cylinder;
pub mod sphere;

/// Emit trait must be implemented for any particle system emitter.
pub trait Emit {
    /// Initializes state of particle using given emitter and particle system.
    fn emit(&self, particle: &mut Particle, rng: &mut ParticleSystemRng);
}

/// See module docs.
#[derive(PartialEq, Debug, Reflect, AsRefStr, EnumString, VariantNames)]
pub enum Emitter {
    /// See BoxEmitter docs.
    Cuboid(CuboidEmitter),
    /// See SphereEmitter docs.
    Sphere(SphereEmitter),
    /// Cylinder emitter.
    Cylinder(CylinderEmitter),
}

uuid_provider!(Emitter = "4cad87ed-6b2c-411d-8c05-86dc26e463b2");

impl Emitter {
    /// Creates new emitter from given id.
    pub fn new(id: i32) -> Result<Self, String> {
        match id {
            1 => Ok(Self::Cuboid(Default::default())),
            2 => Ok(Self::Sphere(Default::default())),
            3 => Ok(Self::Cylinder(Default::default())),
            _ => Err(format!("Invalid emitter id {}!", id)),
        }
    }

    /// Returns id of current emitter kind.
    pub fn id(&self) -> i32 {
        match self {
            Self::Cuboid(_) => 1,
            Self::Sphere(_) => 2,
            Self::Cylinder(_) => 3,
        }
    }
}

macro_rules! static_dispatch {
    ($self:ident, $func:ident, $($args:expr),*) => {
        match $self {
            Emitter::Cuboid(v) => v.$func($($args),*),
            Emitter::Sphere(v) => v.$func($($args),*),
            Emitter::Cylinder(v) => v.$func($($args),*),
        }
    };
}

impl Visit for Emitter {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut kind_id: i32 = self.id();
        kind_id.visit("KindId", visitor)?;
        if visitor.is_reading() {
            *self = Emitter::new(kind_id)?;
        }

        static_dispatch!(self, visit, name, visitor)
    }
}

impl Emit for Emitter {
    fn emit(&self, particle: &mut Particle, rng: &mut ParticleSystemRng) {
        static_dispatch!(self, emit, particle, rng)
    }
}

impl Clone for Emitter {
    fn clone(&self) -> Self {
        match self {
            Self::Cuboid(box_emitter) => Self::Cuboid(box_emitter.clone()),
            Self::Sphere(sphere_emitter) => Self::Sphere(sphere_emitter.clone()),
            Self::Cylinder(cylinder) => Self::Cylinder(cylinder.clone()),
        }
    }
}

impl Deref for Emitter {
    type Target = BaseEmitter;

    fn deref(&self) -> &Self::Target {
        static_dispatch!(self, deref,)
    }
}

impl DerefMut for Emitter {
    fn deref_mut(&mut self) -> &mut Self::Target {
        static_dispatch!(self, deref_mut,)
    }
}

impl Default for Emitter {
    fn default() -> Self {
        Self::Cuboid(Default::default())
    }
}
