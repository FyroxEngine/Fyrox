//! Base emitter contains properties for all other "derived" emitters.

use crate::{
    core::{
        algebra::Vector3, color::Color, numeric_range::RangeExt, reflect::prelude::*,
        visitor::prelude::*,
    },
    scene::particle_system::{Particle, ParticleSystemRng},
};
use std::ops::Range;

/// See module docs.
#[derive(Debug, Visit, PartialEq, Reflect)]
pub struct BaseEmitter {
    /// Offset from center of particle system.
    position: Vector3<f32>,
    /// Particle spawn rate in unit-per-second. If < 0, spawns `max_particles`,
    /// spawns nothing if `max_particles` < 0
    #[visit(rename = "SpawnRate")]
    particle_spawn_rate: u32,
    /// Maximum amount of particles emitter can emit. Unlimited if < 0
    #[visit(optional)] // Backward compatibility
    max_particles: Option<u32>,
    /// Range of initial lifetime of a particle
    #[visit(rename = "LifeTime")]
    lifetime: Range<f32>,
    /// Range of initial size of a particle
    size: Range<f32>,
    /// Range of initial size modifier of a particle
    size_modifier: Range<f32>,
    /// Range of initial X-component of velocity for a particle
    x_velocity: Range<f32>,
    /// Range of initial Y-component of velocity for a particle
    y_velocity: Range<f32>,
    /// Range of initial Z-component of velocity for a particle
    z_velocity: Range<f32>,
    /// Range of initial rotation speed for a particle
    rotation_speed: Range<f32>,
    /// Range of initial rotation for a particle
    rotation: Range<f32>,
    #[reflect(hidden)]
    pub(crate) alive_particles: u32,
    #[visit(skip)]
    #[reflect(hidden)]
    time: f32,
    #[visit(skip)]
    #[reflect(hidden)]
    pub(crate) particles_to_spawn: u32,
    resurrect_particles: bool,
    #[reflect(hidden)]
    pub(crate) spawned_particles: u64,
}

/// Emitter builder allows you to construct emitter in declarative manner.
/// This is typical implementation of Builder pattern.
pub struct BaseEmitterBuilder {
    position: Option<Vector3<f32>>,
    particle_spawn_rate: Option<u32>,
    max_particles: Option<u32>,
    lifetime: Range<f32>,
    size: Range<f32>,
    size_modifier: Range<f32>,
    x_velocity: Range<f32>,
    y_velocity: Range<f32>,
    z_velocity: Range<f32>,
    rotation_speed: Range<f32>,
    rotation: Range<f32>,
    resurrect_particles: bool,
}

impl Default for BaseEmitterBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl BaseEmitterBuilder {
    /// Creates new emitter builder in declarative manner.
    pub fn new() -> Self {
        Self {
            position: None,
            particle_spawn_rate: None,
            max_particles: None,
            lifetime: 5.0..10.0,
            size: 0.125..0.250,
            size_modifier: 0.0005..0.0010,
            x_velocity: -0.001..0.001,
            y_velocity: -0.001..0.001,
            z_velocity: -0.001..0.001,
            rotation_speed: -0.02..0.02,
            rotation: -std::f32::consts::PI..std::f32::consts::PI,
            resurrect_particles: true,
        }
    }

    /// Sets desired position of emitter in local coordinates.
    pub fn with_position(mut self, position: Vector3<f32>) -> Self {
        self.position = Some(position);
        self
    }

    /// Sets desired particle spawn rate in s⁻¹ (particles per second)
    pub fn with_spawn_rate(mut self, rate: u32) -> Self {
        self.particle_spawn_rate = Some(rate);
        self
    }

    /// Sets desired max amount of particles.
    pub fn with_max_particles(mut self, value: u32) -> Self {
        self.max_particles = Some(value);
        self
    }

    /// Sets desired lifetime range.
    pub fn with_lifetime_range(mut self, time_range: Range<f32>) -> Self {
        self.lifetime = time_range;
        self
    }

    /// Sets desired size range.
    pub fn with_size_range(mut self, size_range: Range<f32>) -> Self {
        self.size = size_range;
        self
    }

    /// Sets desired size modifier range.
    pub fn with_size_modifier_range(mut self, mod_range: Range<f32>) -> Self {
        self.size_modifier = mod_range;
        self
    }

    /// Sets desired x velocity range.
    pub fn with_x_velocity_range(mut self, x_vel_range: Range<f32>) -> Self {
        self.x_velocity = x_vel_range;
        self
    }

    /// Sets desired y velocity range.
    pub fn with_y_velocity_range(mut self, y_vel_range: Range<f32>) -> Self {
        self.y_velocity = y_vel_range;
        self
    }

    /// Sets desired z velocity range.
    pub fn with_z_velocity_range(mut self, z_vel_range: Range<f32>) -> Self {
        self.z_velocity = z_vel_range;
        self
    }

    /// Sets desired rotation speed range.
    pub fn with_rotation_speed_range(mut self, speed_range: Range<f32>) -> Self {
        self.rotation_speed = speed_range;
        self
    }

    /// Sets desired rotation range.
    pub fn with_rotation_range(mut self, angle_range: Range<f32>) -> Self {
        self.rotation = angle_range;
        self
    }

    /// Sets whether to resurrect dead particle or not.
    pub fn resurrect_particles(mut self, value: bool) -> Self {
        self.resurrect_particles = value;
        self
    }

    /// Creates new instance of emitter.
    pub fn build(self) -> BaseEmitter {
        BaseEmitter {
            position: self.position.unwrap_or_default(),
            particle_spawn_rate: self.particle_spawn_rate.unwrap_or(25),
            max_particles: self.max_particles,
            lifetime: self.lifetime,
            size: self.size,
            size_modifier: self.size_modifier,
            x_velocity: self.x_velocity,
            y_velocity: self.y_velocity,
            z_velocity: self.z_velocity,
            rotation_speed: self.rotation_speed,
            rotation: self.rotation,
            alive_particles: 0,
            time: 0.0,
            particles_to_spawn: 0,
            resurrect_particles: self.resurrect_particles,
            spawned_particles: 0,
        }
    }
}

impl BaseEmitter {
    /// Updates emitter and emits required amount of particles each call. There is no
    /// need to call it manually, it will be automatically called by scene update call.
    pub fn tick(&mut self, dt: f32) {
        self.time += dt;
        let time_amount_per_particle = 1.0 / self.particle_spawn_rate as f32;
        self.particles_to_spawn = (self.time / time_amount_per_particle) as u32;
        self.time -= time_amount_per_particle * self.particles_to_spawn as f32;
        if let Some(max_particles) = self.max_particles {
            let alive_particles = self.alive_particles;
            if alive_particles < max_particles
                && alive_particles + self.particles_to_spawn > max_particles
            {
                self.particles_to_spawn = max_particles.saturating_sub(alive_particles);
            }
            if !self.resurrect_particles && self.spawned_particles >= u64::from(max_particles) {
                self.particles_to_spawn = 0;
            }
        }
        self.spawned_particles += self.particles_to_spawn as u64;
    }

    /// Initializes particle with new state. Every custom emitter must call this method,
    /// otherwise you will get weird behavior of emitted particles.
    pub fn emit(&self, particle: &mut Particle, rng: &mut ParticleSystemRng) {
        particle.lifetime = 0.0;
        particle.initial_lifetime = self.lifetime.random(rng);
        particle.color = Color::WHITE;
        particle.size = self.size.random(rng);
        particle.size_modifier = self.size_modifier.random(rng);
        particle.velocity = Vector3::new(
            self.x_velocity.random(rng),
            self.y_velocity.random(rng),
            self.z_velocity.random(rng),
        );
        particle.rotation = self.rotation.random(rng);
        particle.rotation_speed = self.rotation_speed.random(rng);
    }

    /// Sets new position of emitter in local coordinates.
    pub fn set_position(&mut self, position: Vector3<f32>) -> &mut Self {
        self.position = position;
        self
    }

    /// Returns position of emitter in local coordinates.
    pub fn position(&self) -> Vector3<f32> {
        self.position
    }

    /// Sets new spawn rate in particle per second.
    pub fn set_spawn_rate(&mut self, rate: u32) -> &mut Self {
        self.particle_spawn_rate = rate;
        self
    }

    /// Return spawn rate in particles per second.
    pub fn spawn_rate(&self) -> u32 {
        self.particle_spawn_rate
    }

    /// Sets maximum amount of particles.
    pub fn set_max_particles(&mut self, max: Option<u32>) -> &mut Self {
        self.max_particles = max;
        self
    }

    /// Returns maximum amount of particles.
    pub fn max_particles(&self) -> Option<u32> {
        self.max_particles
    }

    /// Sets new range of lifetimes which will be used to generate random lifetime
    /// of new particle.
    pub fn set_life_time_range(&mut self, range: Range<f32>) -> &mut Self {
        self.lifetime = range;
        self
    }

    /// Returns current lifetime range.
    pub fn life_time_range(&self) -> Range<f32> {
        self.lifetime.clone()
    }

    /// Sets new range of sizes which will be used to generate random size
    /// of new particle.
    pub fn set_size_range(&mut self, range: Range<f32>) -> &mut Self {
        self.size = range;
        self
    }

    /// Returns current size range.
    pub fn size_range(&self) -> Range<f32> {
        self.size.clone()
    }

    /// Sets new range of size modifier which will be used to generate random size modifier
    /// of new particle.
    pub fn set_size_modifier_range(&mut self, range: Range<f32>) -> &mut Self {
        self.size_modifier = range;
        self
    }

    /// Returns current size modifier.
    pub fn size_modifier_range(&self) -> Range<f32> {
        self.size_modifier.clone()
    }

    /// Sets new range of initial x velocity that will be used to generate random
    /// value of initial x velocity of a particle.
    pub fn set_x_velocity_range(&mut self, range: Range<f32>) -> &mut Self {
        self.x_velocity = range;
        self
    }

    /// Returns current range of initial x velocity that will be used to generate
    /// random value of initial x velocity of a particle.
    pub fn x_velocity_range(&self) -> Range<f32> {
        self.x_velocity.clone()
    }

    /// Sets new range of initial y velocity that will be used to generate random
    /// value of initial y velocity of a particle.
    pub fn set_y_velocity_range(&mut self, range: Range<f32>) -> &mut Self {
        self.y_velocity = range;
        self
    }

    /// Returns current range of initial y velocity that will be used to generate
    /// random value of initial y velocity of a particle.
    pub fn y_velocity_range(&self) -> Range<f32> {
        self.y_velocity.clone()
    }

    /// Sets new range of initial z velocity that will be used to generate random
    /// value of initial z velocity of a particle.
    pub fn set_z_velocity_range(&mut self, range: Range<f32>) -> &mut Self {
        self.z_velocity = range;
        self
    }

    /// Returns current range of initial z velocity that will be used to generate
    /// random value of initial z velocity of a particle.
    pub fn z_velocity_range(&self) -> Range<f32> {
        self.z_velocity.clone()
    }

    /// Sets new range of rotation speed that will be used to generate random value
    /// of rotation speed of a particle.
    pub fn set_rotation_speed_range(&mut self, range: Range<f32>) -> &mut Self {
        self.rotation_speed = range;
        self
    }

    /// Returns current range of rotation speed that will be used to generate random
    /// value of rotation speed of a particle.
    pub fn rotation_speed_range(&self) -> Range<f32> {
        self.rotation_speed.clone()
    }

    /// Sets new range of initial rotations that will be used to generate random
    /// value of initial rotation of a particle.
    pub fn set_rotation_range(&mut self, range: Range<f32>) -> &mut Self {
        self.rotation = range;
        self
    }

    /// Returns current range of initial rotations that will be used to generate
    /// random value of initial rotation of a particle.
    pub fn rotation_range(&self) -> Range<f32> {
        self.rotation.clone()
    }

    /// Enables or disables automatic particle resurrection. Setting this option to
    /// true is useful for "endless" effects.
    pub fn enable_particle_resurrection(&mut self, state: bool) -> &mut Self {
        self.resurrect_particles = state;
        self
    }

    /// Returns true if dead particles will be automatically resurrected, false - otherwise.
    pub fn is_particles_resurrects(&self) -> bool {
        self.resurrect_particles
    }

    /// Returns amount of spawned particles from moment of creation of particle system.
    pub fn spawned_particles(&self) -> u64 {
        self.spawned_particles
    }
}

impl Clone for BaseEmitter {
    fn clone(&self) -> Self {
        Self {
            position: self.position,
            particle_spawn_rate: self.particle_spawn_rate,
            max_particles: self.max_particles,
            lifetime: self.lifetime.clone(),
            size: self.size.clone(),
            size_modifier: self.size_modifier.clone(),
            x_velocity: self.x_velocity.clone(),
            y_velocity: self.y_velocity.clone(),
            z_velocity: self.z_velocity.clone(),
            rotation_speed: self.rotation_speed.clone(),
            rotation: self.rotation.clone(),
            alive_particles: self.alive_particles,
            time: self.time,
            particles_to_spawn: 0,
            resurrect_particles: self.resurrect_particles,
            spawned_particles: self.spawned_particles,
        }
    }
}

impl Default for BaseEmitter {
    fn default() -> Self {
        Self {
            position: Vector3::default(),
            particle_spawn_rate: 100,
            max_particles: None,
            lifetime: 5.0..10.0,
            size: 0.125..0.250,
            size_modifier: 0.0005..0.0010,
            x_velocity: -0.001..0.001,
            y_velocity: -0.001..0.001,
            z_velocity: -0.001..0.001,
            rotation_speed: -0.02..0.02,
            rotation: -std::f32::consts::PI..std::f32::consts::PI,
            alive_particles: 0,
            time: 0.0,
            particles_to_spawn: 0,
            resurrect_particles: true,
            spawned_particles: 0,
        }
    }
}
