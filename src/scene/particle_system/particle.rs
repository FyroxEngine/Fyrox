//! Particle is a quad with texture and various other parameters, such as
//! position, velocity, size, lifetime, etc.

use crate::core::{algebra::Vector3, color::Color, visitor::prelude::*};
use std::cell::Cell;

/// See module docs.
#[derive(Clone, Debug, Visit)]
pub struct Particle {
    /// Position of particle in local coordinates.
    #[visit(rename = "Pos")]
    pub position: Vector3<f32>,
    /// Velocity of particle in local coordinates.
    #[visit(rename = "Vel")]
    pub velocity: Vector3<f32>,
    /// Size of particle.
    pub size: f32,
    pub(super) alive: bool,
    /// Modifier for size which will be added to size each update tick.
    pub size_modifier: f32,
    /// Particle is alive if lifetime > 0
    #[visit(rename = "LifeTime")]
    pub(super) lifetime: f32,
    /// Lifetime at the moment when particle was created.
    pub initial_lifetime: f32,
    /// Rotation speed of particle in radians per second.
    pub rotation_speed: f32,
    /// Rotation angle in radians.
    pub rotation: f32,
    /// Color of particle.
    pub color: Color,
    pub(super) emitter_index: u32,

    #[visit(skip)]
    pub(super) sqr_distance_to_camera: Cell<f32>,
}

impl Default for Particle {
    fn default() -> Self {
        Self {
            position: Default::default(),
            velocity: Default::default(),
            size: 1.0,
            alive: true,
            size_modifier: 0.0,
            lifetime: 0.0,
            initial_lifetime: 2.0,
            rotation_speed: 0.0,
            rotation: 0.0,
            emitter_index: 0,
            color: Color::WHITE,
            sqr_distance_to_camera: Cell::new(0.0),
        }
    }
}
