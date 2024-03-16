//! Box emitter emits particles uniformly in its volume. Can be used to create simple fog
//! layer.

use crate::core::numeric_range::RangeExt;
use crate::scene::particle_system::ParticleSystemRng;
use crate::{
    core::{algebra::Vector3, reflect::prelude::*, visitor::prelude::*},
    scene::particle_system::{
        emitter::{
            base::{BaseEmitter, BaseEmitterBuilder},
            Emit, Emitter,
        },
        particle::Particle,
    },
};
use std::ops::{Deref, DerefMut};

/// See module docs.
#[derive(Debug, Clone, Visit, PartialEq, Reflect)]
pub struct CuboidEmitter {
    emitter: BaseEmitter,
    #[reflect(min_value = 0.0, step = 0.1)]
    half_width: f32,
    #[reflect(min_value = 0.0, step = 0.1)]
    half_height: f32,
    #[reflect(min_value = 0.0, step = 0.1)]
    half_depth: f32,
}

impl Deref for CuboidEmitter {
    type Target = BaseEmitter;

    fn deref(&self) -> &Self::Target {
        &self.emitter
    }
}

impl DerefMut for CuboidEmitter {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.emitter
    }
}

impl CuboidEmitter {
    /// Creates new box emitter of given width, height and depth.
    pub fn new(emitter: BaseEmitter, width: f32, height: f32, depth: f32) -> Self {
        Self {
            emitter,
            half_width: width * 0.5,
            half_height: height * 0.5,
            half_depth: depth * 0.5,
        }
    }

    /// Returns half width of the box emitter.
    pub fn half_width(&self) -> f32 {
        self.half_width
    }

    /// Sets half width of the box emitter.
    pub fn set_half_width(&mut self, half_width: f32) {
        self.half_width = half_width.max(0.0);
    }

    /// Returns half height of the box emitter.
    pub fn half_height(&self) -> f32 {
        self.half_height
    }

    /// Sets half height of the box emitter.
    pub fn set_half_height(&mut self, half_height: f32) {
        self.half_height = half_height.max(0.0);
    }

    /// Returns half depth of the box emitter.
    pub fn half_depth(&self) -> f32 {
        self.half_depth
    }

    /// Sets half depth of the box emitter.
    pub fn set_half_depth(&mut self, half_depth: f32) {
        self.half_depth = half_depth.max(0.0);
    }
}

impl Default for CuboidEmitter {
    fn default() -> Self {
        Self {
            emitter: Default::default(),
            half_width: 0.5,
            half_height: 0.5,
            half_depth: 0.5,
        }
    }
}

impl Emit for CuboidEmitter {
    fn emit(&self, particle: &mut Particle, rng: &mut ParticleSystemRng) {
        self.emitter.emit(particle, rng);
        let position = self.position();
        particle.position = Vector3::new(
            position.x + (-self.half_width..self.half_width).random(rng),
            position.y + (-self.half_height..self.half_height).random(rng),
            position.z + (-self.half_depth..self.half_depth).random(rng),
        )
    }
}

/// Box emitter builder allows you to construct box emitter in declarative manner.
/// This is typical implementation of Builder pattern.
pub struct CuboidEmitterBuilder {
    base: BaseEmitterBuilder,
    width: f32,
    height: f32,
    depth: f32,
}

impl CuboidEmitterBuilder {
    /// Creates new box emitter builder with
    pub fn new(base: BaseEmitterBuilder) -> Self {
        Self {
            base,
            width: 1.0,
            height: 1.0,
            depth: 1.0,
        }
    }

    /// Sets desired width of the emitter.
    pub fn with_width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// Sets desired height of the emitter.
    pub fn with_height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    /// Sets desired depth of the emitter.
    pub fn with_depth(mut self, depth: f32) -> Self {
        self.depth = depth;
        self
    }

    /// Creates new box emitter with given parameters.
    pub fn build(self) -> Emitter {
        Emitter::Cuboid(CuboidEmitter {
            emitter: self.base.build(),
            half_width: self.width * 0.5,
            half_height: self.height * 0.5,
            half_depth: self.depth * 0.5,
        })
    }
}
