//! Base emitter contains properties for all other "derived" emitters.

use crate::{
    core::{algebra::Vector3, color::Color, numeric_range::NumericRange, visitor::prelude::*},
    scene::particle_system::{Particle, ParticleLimit},
};
use std::cell::Cell;

/// See module docs.
#[derive(Debug, Visit)]
pub struct BaseEmitter {
    /// Offset from center of particle system.
    position: Vector3<f32>,
    /// Particle spawn rate in unit-per-second. If < 0, spawns `max_particles`,
    /// spawns nothing if `max_particles` < 0
    particle_spawn_rate: u32,
    /// Maximum amount of particles emitter can emit. Unlimited if < 0
    max_particles: ParticleLimit,
    /// Range of initial lifetime of a particle
    lifetime: NumericRange,
    /// Range of initial size of a particle
    size: NumericRange,
    /// Range of initial size modifier of a particle
    size_modifier: NumericRange,
    /// Range of initial X-component of velocity for a particle
    x_velocity: NumericRange,
    /// Range of initial Y-component of velocity for a particle
    y_velocity: NumericRange,
    /// Range of initial Z-component of velocity for a particle
    z_velocity: NumericRange,
    /// Range of initial rotation speed for a particle
    rotation_speed: NumericRange,
    /// Range of initial rotation for a particle
    rotation: NumericRange,
    pub(crate) alive_particles: Cell<u32>,
    time: f32,
    pub(crate) particles_to_spawn: u32,
    resurrect_particles: bool,
    spawned_particles: u64,
}

/// Emitter builder allows you to construct emitter in declarative manner.
/// This is typical implementation of Builder pattern.
pub struct BaseEmitterBuilder {
    position: Option<Vector3<f32>>,
    particle_spawn_rate: Option<u32>,
    max_particles: Option<u32>,
    lifetime: Option<NumericRange>,
    size: Option<NumericRange>,
    size_modifier: Option<NumericRange>,
    x_velocity: Option<NumericRange>,
    y_velocity: Option<NumericRange>,
    z_velocity: Option<NumericRange>,
    rotation_speed: Option<NumericRange>,
    rotation: Option<NumericRange>,
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
            lifetime: None,
            size: None,
            size_modifier: None,
            x_velocity: None,
            y_velocity: None,
            z_velocity: None,
            rotation_speed: None,
            rotation: None,
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
    pub fn with_lifetime_range(mut self, time_range: NumericRange) -> Self {
        self.lifetime = Some(time_range);
        self
    }

    /// Sets desired size range.
    pub fn with_size_range(mut self, size_range: NumericRange) -> Self {
        self.size = Some(size_range);
        self
    }

    /// Sets desired size modifier range.
    pub fn with_size_modifier_range(mut self, mod_range: NumericRange) -> Self {
        self.size_modifier = Some(mod_range);
        self
    }

    /// Sets desired x velocity range.
    pub fn with_x_velocity_range(mut self, x_vel_range: NumericRange) -> Self {
        self.x_velocity = Some(x_vel_range);
        self
    }

    /// Sets desired y velocity range.
    pub fn with_y_velocity_range(mut self, y_vel_range: NumericRange) -> Self {
        self.y_velocity = Some(y_vel_range);
        self
    }

    /// Sets desired z velocity range.
    pub fn with_z_velocity_range(mut self, z_vel_range: NumericRange) -> Self {
        self.z_velocity = Some(z_vel_range);
        self
    }

    /// Sets desired rotation speed range.
    pub fn with_rotation_speed_range(mut self, speed_range: NumericRange) -> Self {
        self.rotation_speed = Some(speed_range);
        self
    }

    /// Sets desired rotation range.
    pub fn with_rotation_range(mut self, angle_range: NumericRange) -> Self {
        self.rotation = Some(angle_range);
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
            max_particles: self
                .max_particles
                .map_or(ParticleLimit::Unlimited, ParticleLimit::Strict),
            lifetime: self
                .lifetime
                .unwrap_or_else(|| NumericRange::new(5.0, 10.0)),
            size: self.size.unwrap_or_else(|| NumericRange::new(0.125, 0.250)),
            size_modifier: self
                .size_modifier
                .unwrap_or_else(|| NumericRange::new(0.0005, 0.0010)),
            x_velocity: self
                .x_velocity
                .unwrap_or_else(|| NumericRange::new(-0.001, 0.001)),
            y_velocity: self
                .y_velocity
                .unwrap_or_else(|| NumericRange::new(-0.001, 0.001)),
            z_velocity: self
                .z_velocity
                .unwrap_or_else(|| NumericRange::new(-0.001, 0.001)),
            rotation_speed: self
                .rotation_speed
                .unwrap_or_else(|| NumericRange::new(-0.02, 0.02)),
            rotation: self
                .rotation
                .unwrap_or_else(|| NumericRange::new(-std::f32::consts::PI, std::f32::consts::PI)),
            alive_particles: Cell::new(0),
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
        let mut particle_count = (self.time / time_amount_per_particle) as u32;
        self.time -= time_amount_per_particle * particle_count as f32;
        if let ParticleLimit::Strict(max_particles) = self.max_particles {
            let alive_particles = self.alive_particles.get();
            if alive_particles < max_particles && alive_particles + particle_count > max_particles {
                particle_count = max_particles - particle_count;
            }
            if !self.resurrect_particles && self.spawned_particles > u64::from(max_particles) {
                self.particles_to_spawn = 0;
                return;
            }
        }
        self.particles_to_spawn = particle_count;
        self.spawned_particles += self.particles_to_spawn as u64;
    }

    /// Initializes particle with new state. Every custom emitter must call this method,
    /// otherwise you will get weird behavior of emitted particles.
    pub fn emit(&self, particle: &mut Particle) {
        particle.lifetime = 0.0;
        particle.initial_lifetime = self.lifetime.random();
        particle.color = Color::WHITE;
        particle.size = self.size.random();
        particle.size_modifier = self.size_modifier.random();
        particle.velocity = Vector3::new(
            self.x_velocity.random(),
            self.y_velocity.random(),
            self.z_velocity.random(),
        );
        particle.rotation = self.rotation.random();
        particle.rotation_speed = self.rotation_speed.random();
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
    pub fn set_max_particles(&mut self, max: ParticleLimit) -> &mut Self {
        self.max_particles = max;
        self
    }

    /// Returns maximum amount of particles.
    pub fn max_particles(&self) -> ParticleLimit {
        self.max_particles
    }

    /// Sets new range of lifetimes which will be used to generate random lifetime
    /// of new particle.
    pub fn set_life_time_range(&mut self, range: NumericRange) -> &mut Self {
        self.lifetime = range;
        self
    }

    /// Returns current lifetime range.
    pub fn life_time_range(&self) -> NumericRange {
        self.lifetime
    }

    /// Sets new range of sizes which will be used to generate random size
    /// of new particle.
    pub fn set_size_range(&mut self, range: NumericRange) -> &mut Self {
        self.size = range;
        self
    }

    /// Returns current size range.
    pub fn size_range(&self) -> NumericRange {
        self.size
    }

    /// Sets new range of size modifier which will be used to generate random size modifier
    /// of new particle.
    pub fn set_size_modifier_range(&mut self, range: NumericRange) -> &mut Self {
        self.size_modifier = range;
        self
    }

    /// Returns current size modifier.
    pub fn size_modifier_range(&self) -> NumericRange {
        self.size_modifier
    }

    /// Sets new range of initial x velocity that will be used to generate random
    /// value of initial x velocity of a particle.
    pub fn set_x_velocity_range(&mut self, range: NumericRange) -> &mut Self {
        self.x_velocity = range;
        self
    }

    /// Returns current range of initial x velocity that will be used to generate
    /// random value of initial x velocity of a particle.
    pub fn x_velocity_range(&self) -> NumericRange {
        self.x_velocity
    }

    /// Sets new range of initial y velocity that will be used to generate random
    /// value of initial y velocity of a particle.
    pub fn set_y_velocity_range(&mut self, range: NumericRange) -> &mut Self {
        self.y_velocity = range;
        self
    }

    /// Returns current range of initial y velocity that will be used to generate
    /// random value of initial y velocity of a particle.
    pub fn y_velocity_range(&self) -> NumericRange {
        self.y_velocity
    }

    /// Sets new range of initial z velocity that will be used to generate random
    /// value of initial z velocity of a particle.
    pub fn set_z_velocity_range(&mut self, range: NumericRange) -> &mut Self {
        self.z_velocity = range;
        self
    }

    /// Returns current range of initial z velocity that will be used to generate
    /// random value of initial z velocity of a particle.
    pub fn z_velocity_range(&self) -> NumericRange {
        self.z_velocity
    }

    /// Sets new range of rotation speed that will be used to generate random value
    /// of rotation speed of a particle.
    pub fn set_rotation_speed_range(&mut self, range: NumericRange) -> &mut Self {
        self.rotation_speed = range;
        self
    }

    /// Returns current range of rotation speed that will be used to generate random
    /// value of rotation speed of a particle.
    pub fn rotation_speed_range(&self) -> NumericRange {
        self.rotation_speed
    }

    /// Sets new range of initial rotations that will be used to generate random
    /// value of initial rotation of a particle.
    pub fn set_rotation_range(&mut self, range: NumericRange) -> &mut Self {
        self.rotation = range;
        self
    }

    /// Returns current range of initial rotations that will be used to generate
    /// random value of initial rotation of a particle.
    pub fn rotation_range(&self) -> NumericRange {
        self.rotation
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
            lifetime: self.lifetime,
            size: self.size,
            size_modifier: self.size_modifier,
            x_velocity: self.x_velocity,
            y_velocity: self.y_velocity,
            z_velocity: self.z_velocity,
            rotation_speed: self.rotation_speed,
            rotation: self.rotation,
            alive_particles: self.alive_particles.clone(),
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
            max_particles: ParticleLimit::Unlimited,
            lifetime: NumericRange::new(5.0, 10.0),
            size: NumericRange::new(0.125, 0.250),
            size_modifier: NumericRange::new(0.0005, 0.0010),
            x_velocity: NumericRange::new(-0.001, 0.001),
            y_velocity: NumericRange::new(-0.001, 0.001),
            z_velocity: NumericRange::new(-0.001, 0.001),
            rotation_speed: NumericRange::new(-0.02, 0.02),
            rotation: NumericRange::new(-std::f32::consts::PI, std::f32::consts::PI),
            alive_particles: Cell::new(0),
            time: 0.0,
            particles_to_spawn: 0,
            resurrect_particles: true,
            spawned_particles: 0,
        }
    }
}
