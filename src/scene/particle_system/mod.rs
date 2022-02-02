//! Contains all structures and methods to create and manage particle systems.
//!
//! Particle system used to create visual effects that consists of many small parts,
//! this can be smoke, fire, dust, sparks, etc. Particle system optimized to operate
//! on many small parts, so it is much efficient to use particle system instead of
//! separate scene nodes. Downside of particle system is that there almost no way
//! to control separate particles, all particles controlled by parameters of particle
//! emitters.
//!
//! # Emitters
//!
//! Particle system can contain multiple particle emitters, each emitter has its own
//! set of properties and it defines law of change of particle parameters over time.
//!
//! # Performance
//!
//! In general particle system can be considered as heavy visual effect, but total impact
//! on performance defined by amount of particles and amount of pixels they take to render.
//! A rule of thumb will be to decrease amount of particles until effect will look good
//! enough, alternatively amount of particles can be defined by some coefficient based on
//! graphics quality settings.
//!
//! # Example
//!
//! Simple smoke effect can be create like so:
//!
//! ```
//! use fyrox::scene::particle_system::{
//!     emitter::sphere::SphereEmitter, ParticleSystemBuilder, emitter::Emitter,
//!     emitter::base::BaseEmitterBuilder, emitter::sphere::SphereEmitterBuilder
//! };
//! use fyrox::engine::resource_manager::ResourceManager;
//! use fyrox::core::algebra::Vector3;
//! use fyrox::scene::graph::Graph;
//! use fyrox::scene::node::Node;
//! use fyrox::scene::transform::TransformBuilder;
//! use fyrox::core::color_gradient::{GradientPoint, ColorGradient};
//! use fyrox::scene::base::BaseBuilder;
//! use fyrox::core::color::Color;
//! use std::path::Path;
//! use fyrox::resource::texture::TexturePixelKind;
//!
//! fn create_smoke(graph: &mut Graph, resource_manager: &mut ResourceManager, pos: Vector3<f32>) {
//!      ParticleSystemBuilder::new(BaseBuilder::new()
//!         .with_lifetime(5.0)
//!         .with_local_transform(TransformBuilder::new()
//!             .with_local_position(pos)
//!             .build()))
//!         .with_acceleration(Vector3::new(0.0, 0.0, 0.0))
//!         .with_color_over_lifetime_gradient({
//!             let mut gradient = ColorGradient::new();
//!             gradient.add_point(GradientPoint::new(0.00, Color::from_rgba(150, 150, 150, 0)));
//!             gradient.add_point(GradientPoint::new(0.05, Color::from_rgba(150, 150, 150, 220)));
//!             gradient.add_point(GradientPoint::new(0.85, Color::from_rgba(255, 255, 255, 180)));
//!             gradient.add_point(GradientPoint::new(1.00, Color::from_rgba(255, 255, 255, 0)));
//!             gradient
//!         })
//!         .with_emitters(vec![
//!             SphereEmitterBuilder::new(BaseEmitterBuilder::new()
//!                 .with_max_particles(100)
//!                 .with_spawn_rate(50)
//!                 .with_x_velocity_range(-0.01..0.01)
//!                 .with_y_velocity_range(0.02..0.03)
//!                 .with_z_velocity_range(-0.01..0.01))
//!                 .with_radius(0.01)
//!                 .build()
//!         ])
//!         .with_texture(resource_manager.request_texture(Path::new("data/particles/smoke_04.tga")))
//!         .build(graph);
//! }
//! ```

use crate::engine::resource_manager::ResourceManager;
use crate::scene::variable::TemplateVariable;
use crate::{
    core::{
        algebra::{Vector2, Vector3},
        color::Color,
        color_gradient::ColorGradient,
        inspect::{Inspect, PropertyInfo},
        math::TriangleDefinition,
        pool::Handle,
        visitor::prelude::*,
    },
    resource::texture::Texture,
    scene::{
        base::{Base, BaseBuilder},
        graph::Graph,
        node::Node,
        particle_system::{
            draw::{DrawData, Vertex},
            emitter::{Emit, Emitter},
            particle::Particle,
        },
    },
};
use std::{
    cmp::Ordering,
    fmt::Debug,
    ops::{Deref, DerefMut},
};

pub(crate) mod draw;
pub mod emitter;
pub mod particle;

/// Particle limit for emitter.
#[derive(Copy, Clone, Debug)]
pub enum ParticleLimit {
    /// No limit in amount of particles.
    Unlimited,
    /// Strict limit in amount of particles.
    Strict(u32),
}

impl Visit for ParticleLimit {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        let mut amount = match self {
            Self::Unlimited => -1,
            Self::Strict(value) => *value as i32,
        };

        amount.visit("Amount", visitor)?;

        if visitor.is_reading() {
            *self = if amount < 0 {
                Self::Unlimited
            } else {
                Self::Strict(amount as u32)
            };
        }

        visitor.leave_region()
    }
}

/// See module docs.
#[derive(Debug, Visit, Inspect)]
pub struct ParticleSystem {
    base: Base,

    /// List of emitters of the particle system.
    #[inspect(getter = "Deref::deref")]
    pub emitters: TemplateVariable<Vec<Emitter>>,

    #[inspect(getter = "Deref::deref")]
    texture: TemplateVariable<Option<Texture>>,

    #[inspect(getter = "Deref::deref")]
    acceleration: TemplateVariable<Vector3<f32>>,

    #[visit(rename = "ColorGradient")]
    #[inspect(getter = "Deref::deref")]
    color_over_lifetime: TemplateVariable<Option<ColorGradient>>,

    #[visit(optional)] // Backward compatibility.
    #[inspect(getter = "Deref::deref")]
    soft_boundary_sharpness_factor: TemplateVariable<f32>,

    #[visit(optional)] // Backward compatibility.
    #[inspect(getter = "Deref::deref")]
    enabled: TemplateVariable<bool>,

    #[inspect(skip)]
    particles: Vec<Particle>,
    #[inspect(skip)]
    free_particles: Vec<u32>,
}

impl Deref for ParticleSystem {
    type Target = Base;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for ParticleSystem {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl ParticleSystem {
    /// Creates a raw copy of a particle system node.
    pub fn raw_copy(&self) -> Self {
        Self {
            base: self.base.raw_copy(),
            particles: self.particles.clone(),
            free_particles: self.free_particles.clone(),
            emitters: self.emitters.clone(),
            texture: self.texture.clone(),
            acceleration: self.acceleration.clone(),
            color_over_lifetime: self.color_over_lifetime.clone(),
            soft_boundary_sharpness_factor: self.soft_boundary_sharpness_factor.clone(),
            enabled: self.enabled.clone(),
        }
    }

    /// Returns current acceleration for particles in particle system.
    pub fn acceleration(&self) -> Vector3<f32> {
        *self.acceleration
    }

    /// Set new acceleration that will be applied to all particles,
    /// can be used to change "gravity" vector of particles.
    pub fn set_acceleration(&mut self, accel: Vector3<f32>) {
        self.acceleration.set(accel);
    }

    /// Sets new "color curve" that will evaluate color over lifetime.
    pub fn set_color_over_lifetime_gradient(&mut self, gradient: ColorGradient) {
        self.color_over_lifetime.set(Some(gradient));
    }

    /// Return current soft boundary sharpness factor.
    pub fn soft_boundary_sharpness_factor(&self) -> f32 {
        *self.soft_boundary_sharpness_factor
    }

    /// Enables or disables particle system. Disabled particle system remains in "frozen" state
    /// until enabled again.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled.set(enabled);
    }

    /// Returns current particle system status.
    pub fn is_enabled(&self) -> bool {
        *self.enabled
    }

    /// Sets soft boundary sharpness factor. This value defines how wide soft boundary will be.
    /// The greater the factor is the more thin the boundary will be, and vice versa. This
    /// parameter allows you to manipulate particle "softness" - the engine automatically adds
    /// fading to those pixels of a particle which is close enough to other geometry in a scene.
    pub fn set_soft_boundary_sharpness_factor(&mut self, factor: f32) {
        self.soft_boundary_sharpness_factor.set(factor);
    }

    /// Removes all generated particles.
    pub fn clear_particles(&mut self) {
        self.particles.clear();
        self.free_particles.clear();
        for emitter in self.emitters.get_mut_silent().iter_mut() {
            emitter.alive_particles = 0;
        }
    }

    /// Updates state of particle system, this means that it moves particles,
    /// changes their color, size, rotation, etc. This method should not be
    /// used directly, it will be automatically called by scene update.
    pub fn update(&mut self, dt: f32) {
        if !*self.enabled {
            return;
        }

        for emitter in self.emitters.get_mut_silent().iter_mut() {
            emitter.tick(dt);
        }

        for (i, emitter) in self.emitters.get_mut_silent().iter_mut().enumerate() {
            for _ in 0..emitter.particles_to_spawn {
                let mut particle = Particle {
                    emitter_index: i as u32,
                    ..Particle::default()
                };
                emitter.alive_particles += 1;
                emitter.emit(&mut particle);
                if let Some(free_index) = self.free_particles.pop() {
                    self.particles[free_index as usize] = particle;
                } else {
                    self.particles.push(particle);
                }
            }
        }

        let acceleration_offset = self.acceleration.scale(dt * dt);

        for (i, particle) in self.particles.iter_mut().enumerate() {
            if particle.alive {
                particle.lifetime += dt;
                if particle.lifetime >= particle.initial_lifetime {
                    self.free_particles.push(i as u32);
                    if let Some(emitter) = self
                        .emitters
                        .get_mut()
                        .get_mut(particle.emitter_index as usize)
                    {
                        emitter.alive_particles -= 1;
                    }
                    particle.alive = false;
                    particle.lifetime = particle.initial_lifetime;
                } else {
                    particle.velocity += acceleration_offset;
                    particle.position += particle.velocity;
                    particle.size += particle.size_modifier * dt;
                    if particle.size < 0.0 {
                        particle.size = 0.0;
                    }
                    particle.rotation += particle.rotation_speed * dt;
                    if let Some(color_over_lifetime) = self.color_over_lifetime.as_ref() {
                        let k = particle.lifetime / particle.initial_lifetime;
                        particle.color = color_over_lifetime.get_color(k);
                    } else {
                        particle.color = Color::WHITE;
                    }
                }
            }
        }
    }

    /// Generates new draw data for current frame. Should not be used directly, unless you
    /// absolutely need draw data before rendering. It is automatically called by renderer.
    pub fn generate_draw_data(
        &self,
        sorted_particles: &mut Vec<u32>,
        draw_data: &mut DrawData,
        camera_pos: &Vector3<f32>,
    ) {
        sorted_particles.clear();
        for (i, particle) in self.particles.iter().enumerate() {
            if particle.alive {
                let actual_position = particle.position + self.base.global_position();
                particle
                    .sqr_distance_to_camera
                    .set((camera_pos - actual_position).norm_squared());
                sorted_particles.push(i as u32);
            }
        }

        let particles = &self.particles;

        sorted_particles.sort_by(|a, b| {
            let particle_a = particles.get(*a as usize).unwrap();
            let particle_b = particles.get(*b as usize).unwrap();

            // Reverse ordering because we want to sort back-to-front.
            if particle_a.sqr_distance_to_camera < particle_b.sqr_distance_to_camera {
                Ordering::Greater
            } else if particle_a.sqr_distance_to_camera > particle_b.sqr_distance_to_camera {
                Ordering::Less
            } else {
                Ordering::Equal
            }
        });

        draw_data.clear();

        for (i, particle_index) in sorted_particles.iter().enumerate() {
            let particle = self.particles.get(*particle_index as usize).unwrap();

            let linear_color = particle.color.srgb_to_linear();

            draw_data.vertices.push(Vertex {
                position: particle.position,
                tex_coord: Vector2::default(),
                size: particle.size,
                rotation: particle.rotation,
                color: linear_color,
            });

            draw_data.vertices.push(Vertex {
                position: particle.position,
                tex_coord: Vector2::new(1.0, 0.0),
                size: particle.size,
                rotation: particle.rotation,
                color: linear_color,
            });

            draw_data.vertices.push(Vertex {
                position: particle.position,
                tex_coord: Vector2::new(1.0, 1.0),
                size: particle.size,
                rotation: particle.rotation,
                color: linear_color,
            });

            draw_data.vertices.push(Vertex {
                position: particle.position,
                tex_coord: Vector2::new(0.0, 1.0),
                size: particle.size,
                rotation: particle.rotation,
                color: linear_color,
            });

            let base_index = (i * 4) as u32;

            draw_data.triangles.push(TriangleDefinition([
                base_index,
                base_index + 1,
                base_index + 2,
            ]));
            draw_data.triangles.push(TriangleDefinition([
                base_index,
                base_index + 2,
                base_index + 3,
            ]));
        }
    }

    /// Sets new texture for particle system.
    pub fn set_texture(&mut self, texture: Option<Texture>) {
        self.texture.set(texture);
    }

    /// Returns current texture used by particle system.
    pub fn texture(&self) -> Option<Texture> {
        (*self.texture).clone()
    }

    /// Returns current texture used by particle system by ref.
    pub fn texture_ref(&self) -> Option<&Texture> {
        self.texture.as_ref()
    }

    pub(crate) fn restore_resources(&mut self, resource_manager: ResourceManager) {
        let mut state = resource_manager.state();
        let texture_container = &mut state.containers_mut().textures;
        texture_container.try_restore_template_resource(&mut self.texture);
    }

    // Prefab inheritance resolving.
    pub(crate) fn inherit(&mut self, parent: &Node) {
        self.base.inherit_properties(parent);
        if let Node::ParticleSystem(parent) = parent {
            self.emitters.try_inherit(&parent.emitters);
            self.texture.try_inherit(&parent.texture);
            self.acceleration.try_inherit(&parent.acceleration);
            self.color_over_lifetime
                .try_inherit(&parent.color_over_lifetime);
            self.soft_boundary_sharpness_factor
                .try_inherit(&parent.soft_boundary_sharpness_factor);
            self.enabled.try_inherit(&parent.enabled);
        }
    }
}

impl Default for ParticleSystem {
    fn default() -> Self {
        ParticleSystemBuilder::new(BaseBuilder::new()).build_particle_system()
    }
}

/// Particle system builder allows you to construct particle system in declarative manner.
/// This is typical implementation of Builder pattern.
pub struct ParticleSystemBuilder {
    base_builder: BaseBuilder,
    emitters: Vec<Emitter>,
    texture: Option<Texture>,
    acceleration: Vector3<f32>,
    particles: Vec<Particle>,
    color_over_lifetime: Option<ColorGradient>,
    soft_boundary_sharpness_factor: f32,
    enabled: bool,
}

impl ParticleSystemBuilder {
    /// Creates new builder with default parameters.
    pub fn new(base_builder: BaseBuilder) -> Self {
        Self {
            base_builder,
            emitters: Default::default(),
            texture: None,
            particles: Default::default(),
            acceleration: Vector3::new(0.0, -9.81, 0.0),
            color_over_lifetime: None,
            soft_boundary_sharpness_factor: 2.5,
            enabled: true,
        }
    }

    /// Sets desired emitters for particle system.
    pub fn with_emitters(mut self, emitters: Vec<Emitter>) -> Self {
        self.emitters = emitters;
        self
    }

    /// Sets desired texture for particle system.
    pub fn with_texture(mut self, texture: Texture) -> Self {
        self.texture = Some(texture);
        self
    }

    /// Sets desired texture for particle system.
    pub fn with_opt_texture(mut self, texture: Option<Texture>) -> Self {
        self.texture = texture;
        self
    }

    /// Sets desired soft boundary sharpness factor.
    pub fn with_soft_boundary_sharpness_factor(mut self, factor: f32) -> Self {
        self.soft_boundary_sharpness_factor = factor;
        self
    }

    /// Sets desired acceleration for particle system.
    pub fn with_acceleration(mut self, acceleration: Vector3<f32>) -> Self {
        self.acceleration = acceleration;
        self
    }

    /// Sets color gradient over lifetime for particle system.
    pub fn with_color_over_lifetime_gradient(mut self, color_over_lifetime: ColorGradient) -> Self {
        self.color_over_lifetime = Some(color_over_lifetime);
        self
    }

    /// Sets an initial set of particles that not belongs to any emitter. This method
    /// could be useful if you need a custom position/velocity/etc. of each particle.
    pub fn with_particles(mut self, particles: Vec<Particle>) -> Self {
        self.particles = particles;
        self
    }

    /// Sets initial particle system state.
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    fn build_particle_system(self) -> ParticleSystem {
        ParticleSystem {
            base: self.base_builder.build_base(),
            particles: self.particles,
            free_particles: Vec::new(),
            emitters: self.emitters.into(),
            texture: self.texture.into(),
            acceleration: self.acceleration.into(),
            color_over_lifetime: self.color_over_lifetime.into(),
            soft_boundary_sharpness_factor: self.soft_boundary_sharpness_factor.into(),
            enabled: self.enabled.into(),
        }
    }

    /// Creates new instance of particle system.
    pub fn build_node(self) -> Node {
        Node::ParticleSystem(self.build_particle_system())
    }

    /// Creates new instance of particle system and adds it to the graph.
    pub fn build(self, graph: &mut Graph) -> Handle<Node> {
        graph.add_node(self.build_node())
    }
}
