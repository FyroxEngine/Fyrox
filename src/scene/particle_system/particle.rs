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
    /// Modifier for size which will be added to size each update tick.
    pub size_modifier: f32,
    /// Lifetime at the moment when particle was created.
    pub initial_lifetime: f32,
    /// Rotation speed of particle in radians per second.
    pub rotation_speed: f32,
    /// Rotation angle in radians.
    pub rotation: f32,
    /// Color of particle.
    pub color: Color,

    pub(super) alive: bool,
    pub(super) emitter_index: u32,
    /// Particle is alive if lifetime > 0
    #[visit(rename = "LifeTime")]
    pub(super) lifetime: f32,
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

impl Particle {
    /// Sets new position in builder manner.
    pub fn with_position(mut self, position: Vector3<f32>) -> Self {
        self.position = position;
        self
    }

    /// Sets new velocity in builder manner.
    pub fn with_velocity(mut self, velocity: Vector3<f32>) -> Self {
        self.velocity = velocity;
        self
    }

    /// Sets new size in builder manner.
    pub fn with_size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    /// Sets new size modifier in builder manner.
    pub fn with_size_modifier(mut self, size_modifier: f32) -> Self {
        self.size_modifier = size_modifier;
        self
    }

    /// Sets new initial lifetime in builder manner.
    pub fn with_initial_lifetime(mut self, initial_lifetime: f32) -> Self {
        self.initial_lifetime = initial_lifetime;
        self
    }

    /// Sets new rotation in builder manner.
    pub fn with_rotation(mut self, rotation: f32) -> Self {
        self.rotation = rotation;
        self
    }

    /// Sets new rotation speed in builder manner.
    pub fn with_rotation_speed(mut self, rotation_speed: f32) -> Self {
        self.rotation_speed = rotation_speed;
        self
    }

    /// Sets new color in builder manner.
    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }
}
