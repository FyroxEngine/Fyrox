//! Contains all structures and methods to create and manage particle systems. See [`ParticleSystem`] docs for more
//! info and usage examples.

use crate::scene::mesh::buffer::VertexTrait;
use crate::scene::node::RdcControlFlow;
use crate::{
    core::{
        algebra::{Point3, Vector2, Vector3},
        color_gradient::ColorGradient,
        log::Log,
        math::{aabb::AxisAlignedBoundingBox, TriangleDefinition},
        pool::Handle,
        reflect::prelude::*,
        sstorage::ImmutableString,
        uuid::{uuid, Uuid},
        variable::InheritableVariable,
        visitor::prelude::*,
        TypeUuidProvider,
    },
    material::{self, Material, MaterialResource, PropertyValue},
    rand::{prelude::StdRng, Error, RngCore, SeedableRng},
    renderer::{self, bundle::RenderContext},
    scene::{
        base::{Base, BaseBuilder},
        graph::Graph,
        mesh::RenderPath,
        node::{Node, NodeTrait, UpdateContext},
        particle_system::{
            draw::Vertex,
            emitter::{Emit, Emitter},
            particle::Particle,
        },
    },
};
use fyrox_core::value_as_u8_slice;
use fyrox_graph::BaseSceneGraph;
use std::{
    cmp::Ordering,
    fmt::Debug,
    ops::{Deref, DerefMut},
};

pub(crate) mod draw;
pub mod emitter;
pub mod particle;

/// Pseudo-random numbers generator for particle systems.
#[derive(Debug, Clone, Reflect)]
pub struct ParticleSystemRng {
    rng_seed: u64,

    #[reflect(hidden)]
    rng: StdRng,
}

impl Default for ParticleSystemRng {
    fn default() -> Self {
        Self::new(0xDEADBEEF)
    }
}

impl ParticleSystemRng {
    /// Creates new PRNG with a given seed. Fixed seed guarantees that particle system's behaviour will be
    /// deterministic.
    pub fn new(seed: u64) -> Self {
        Self {
            rng_seed: seed,
            rng: StdRng::seed_from_u64(seed),
        }
    }

    /// Resets the state of PRNG.
    #[inline]
    pub fn reset(&mut self) {
        self.rng = StdRng::seed_from_u64(self.rng_seed);
    }
}

impl RngCore for ParticleSystemRng {
    #[inline]
    fn next_u32(&mut self) -> u32 {
        self.rng.next_u32()
    }

    #[inline]
    fn next_u64(&mut self) -> u64 {
        self.rng.next_u64()
    }

    #[inline]
    fn fill_bytes(&mut self, dest: &mut [u8]) {
        self.rng.fill_bytes(dest)
    }

    #[inline]
    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), Error> {
        self.rng.try_fill_bytes(dest)
    }
}

impl Visit for ParticleSystemRng {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut guard = visitor.enter_region(name)?;

        self.rng_seed.visit("Seed", &mut guard)?;

        // Re-initialize the RNG to keep determinism.
        if guard.is_reading() {
            self.rng = StdRng::seed_from_u64(self.rng_seed);
        }

        Ok(())
    }
}

/// Particle system used to create visual effects that consists of many small parts,
/// this can be smoke, fire, dust, sparks, etc. Particle system optimized to operate
/// on many small parts, so it is much efficient to use particle system instead of
/// separate scene nodes. Downside of particle system is that there almost no way
/// to control separate particles, all particles controlled by parameters of particle
/// emitters.
///
/// # Emitters
///
/// Particle system can contain multiple particle emitters, each emitter has its own
/// set of properties and it defines law of change of particle parameters over time.
///
/// # Performance
///
/// In general particle system can be considered as heavy visual effect, but total impact
/// on performance defined by amount of particles and amount of pixels they take to render.
/// A rule of thumb will be to decrease amount of particles until effect will look good
/// enough, alternatively amount of particles can be defined by some coefficient based on
/// graphics quality settings.
///
/// # Example
///
/// Simple smoke effect can be create like so:
///
/// ```
/// # use fyrox_impl::{
/// #     asset::manager::ResourceManager,
/// #     core::{
/// #         algebra::Vector3,
/// #         color::Color,
/// #         color_gradient::{ColorGradient, GradientPoint},
/// #         sstorage::ImmutableString,
/// #     },
/// #     material::{Material, PropertyValue, MaterialResource},
/// #     resource::texture::Texture,
/// #     scene::{
/// #         base::BaseBuilder,
/// #         graph::Graph,
/// #         particle_system::{
/// #             emitter::base::BaseEmitterBuilder, emitter::sphere::SphereEmitterBuilder,
/// #             ParticleSystemBuilder,
/// #         },
/// #         transform::TransformBuilder,
/// #     },
/// # };
/// # use std::path::Path;
/// #
/// fn create_smoke(graph: &mut Graph, resource_manager: &mut ResourceManager, pos: Vector3<f32>) {
///     let mut material = Material::standard_particle_system();
///     material
///         .set_property(
///             &ImmutableString::new("diffuseTexture"),
///             PropertyValue::Sampler {
///                 value: Some(
///                     resource_manager
///                         .request::<Texture>(Path::new("data/particles/smoke_04.tga")),
///                 ),
///                 fallback: Default::default(),
///             },
///         )
///         .unwrap();
///
///     ParticleSystemBuilder::new(
///         BaseBuilder::new()
///             .with_lifetime(5.0)
///             .with_local_transform(TransformBuilder::new().with_local_position(pos).build()),
///     )
///     .with_acceleration(Vector3::new(0.0, 0.0, 0.0))
///     .with_color_over_lifetime_gradient({
///         let mut gradient = ColorGradient::new();
///         gradient.add_point(GradientPoint::new(0.00, Color::from_rgba(150, 150, 150, 0)));
///         gradient.add_point(GradientPoint::new(
///             0.05,
///             Color::from_rgba(150, 150, 150, 220),
///         ));
///         gradient.add_point(GradientPoint::new(
///             0.85,
///             Color::from_rgba(255, 255, 255, 180),
///         ));
///         gradient.add_point(GradientPoint::new(1.00, Color::from_rgba(255, 255, 255, 0)));
///         gradient
///     })
///     .with_emitters(vec![SphereEmitterBuilder::new(
///         BaseEmitterBuilder::new()
///             .with_max_particles(100)
///             .with_spawn_rate(50)
///             .with_x_velocity_range(-0.01..0.01)
///             .with_y_velocity_range(0.02..0.03)
///             .with_z_velocity_range(-0.01..0.01),
///     )
///     .with_radius(0.01)
///     .build()])
///     .with_material(MaterialResource::new_ok(Default::default(), material))
///     .build(graph);
/// }
/// ```
#[derive(Debug, Clone, Reflect)]
pub struct ParticleSystem {
    base: Base,

    /// List of emitters of the particle system.
    pub emitters: InheritableVariable<Vec<Emitter>>,

    #[reflect(setter = "set_material")]
    material: InheritableVariable<MaterialResource>,

    #[reflect(setter = "set_acceleration")]
    acceleration: InheritableVariable<Vector3<f32>>,

    #[reflect(setter = "set_color_over_lifetime_gradient")]
    color_over_lifetime: InheritableVariable<ColorGradient>,

    #[reflect(setter = "play")]
    is_playing: InheritableVariable<bool>,

    #[reflect(hidden)]
    particles: Vec<Particle>,

    #[reflect(hidden)]
    free_particles: Vec<u32>,

    rng: ParticleSystemRng,
}

impl Visit for ParticleSystem {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        let mut region = visitor.enter_region(name)?;

        self.base.visit("Base", &mut region)?;
        self.emitters.visit("Emitters", &mut region)?;
        self.acceleration.visit("Acceleration", &mut region)?;
        self.color_over_lifetime
            .visit("ColorGradient", &mut region)?;
        self.is_playing.visit("Enabled", &mut region)?;
        self.particles.visit("Particles", &mut region)?;
        self.free_particles.visit("FreeParticles", &mut region)?;
        let _ = self.rng.visit("Rng", &mut region);

        // Backward compatibility.
        if region.is_reading() {
            if let Some(material) = material::visit_old_texture_as_material(
                &mut region,
                Material::standard_particle_system,
            ) {
                self.material = material.into();
            } else {
                self.material.visit("Material", &mut region)?;
            }
        } else {
            self.material.visit("Material", &mut region)?;
        }

        let mut soft_boundary_sharpness_factor = 100.0;
        if soft_boundary_sharpness_factor
            .visit("SoftBoundarySharpnessFactor", &mut region)
            .is_ok()
        {
            Log::verify(self.material.data_ref().set_property(
                &ImmutableString::new("softBoundarySharpnessFactor"),
                PropertyValue::Float(soft_boundary_sharpness_factor),
            ));
        }

        Ok(())
    }
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

impl TypeUuidProvider for ParticleSystem {
    fn type_uuid() -> Uuid {
        uuid!("8b210eff-97a4-494f-ba7a-a581d3f4a442")
    }
}

impl ParticleSystem {
    /// Returns current acceleration for particles in particle system.
    pub fn acceleration(&self) -> Vector3<f32> {
        *self.acceleration
    }

    /// Set new acceleration that will be applied to all particles,
    /// can be used to change "gravity" vector of particles.
    pub fn set_acceleration(&mut self, accel: Vector3<f32>) -> Vector3<f32> {
        self.acceleration.set_value_and_mark_modified(accel)
    }

    /// Sets new "color curve" that will evaluate color over lifetime.
    pub fn set_color_over_lifetime_gradient(&mut self, gradient: ColorGradient) -> ColorGradient {
        self.color_over_lifetime
            .set_value_and_mark_modified(gradient)
    }

    /// Plays or pauses the particle system. Paused particle system remains in "frozen" state
    /// until played again again. You can manually reset state of the system by calling [`Self::clear_particles`].
    pub fn play(&mut self, is_playing: bool) -> bool {
        self.is_playing.set_value_and_mark_modified(is_playing)
    }

    /// Returns current particle system status.
    pub fn is_playing(&self) -> bool {
        *self.is_playing
    }

    /// Replaces the particles in the particle system with pre-generated set. It could be useful
    /// to create procedural particle effects; when particles cannot be pre-made.
    pub fn set_particles(&mut self, particles: Vec<Particle>) {
        self.free_particles.clear();
        self.particles = particles;
    }

    /// Returns a reference to a slice to the current set of particles, generated by the particle system.
    pub fn particles(&self) -> &[Particle] {
        &self.particles
    }

    /// Removes all generated particles.
    pub fn clear_particles(&mut self) {
        self.particles.clear();
        self.free_particles.clear();
        for emitter in self.emitters.get_value_mut_silent().iter_mut() {
            emitter.alive_particles = 0;
            emitter.spawned_particles = 0;
        }
    }

    /// Sets the new material for the particle system.
    pub fn set_material(&mut self, material: MaterialResource) -> MaterialResource {
        self.material.set_value_and_mark_modified(material)
    }

    /// Returns current material used by particle system.
    pub fn texture(&self) -> MaterialResource {
        (*self.material).clone()
    }

    /// Returns current material used by particle system by ref.
    pub fn texture_ref(&self) -> &MaterialResource {
        &self.material
    }

    fn tick(&mut self, dt: f32) {
        for emitter in self.emitters.get_value_mut_silent().iter_mut() {
            emitter.tick(dt);
        }

        for (i, emitter) in self.emitters.get_value_mut_silent().iter_mut().enumerate() {
            for _ in 0..emitter.particles_to_spawn {
                let mut particle = Particle {
                    emitter_index: i as u32,
                    ..Particle::default()
                };
                emitter.alive_particles += 1;
                emitter.emit(&mut particle, &mut self.rng);
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
                        .get_value_mut_and_mark_modified()
                        .get_mut(particle.emitter_index as usize)
                    {
                        emitter.alive_particles = emitter.alive_particles.saturating_sub(1);
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

                    let k = particle.lifetime / particle.initial_lifetime;
                    particle.color = self.color_over_lifetime.get_color(k);
                }
            }
        }
    }

    /// Simulates particle system for the given `time` with given time step (`dt`). `dt` is usually `1.0 / 60.0`.
    pub fn rewind(&mut self, dt: f32, time: f32) {
        assert!(dt > 0.0);

        self.rng.reset();
        self.clear_particles();

        let mut t = 0.0;
        while t < time {
            self.tick(dt);
            t += dt;
        }
    }
}

impl Default for ParticleSystem {
    fn default() -> Self {
        ParticleSystemBuilder::new(BaseBuilder::new()).build_particle_system()
    }
}

impl NodeTrait for ParticleSystem {
    crate::impl_query_component!();

    fn local_bounding_box(&self) -> AxisAlignedBoundingBox {
        AxisAlignedBoundingBox::unit()
    }

    fn world_bounding_box(&self) -> AxisAlignedBoundingBox {
        self.local_bounding_box()
            .transform(&self.global_transform())
    }

    fn id(&self) -> Uuid {
        Self::type_uuid()
    }

    fn update(&mut self, context: &mut UpdateContext) {
        let dt = context.dt;

        if *self.is_playing {
            self.tick(dt);
        }
    }

    fn collect_render_data(&self, ctx: &mut RenderContext) -> RdcControlFlow {
        if !self.global_visibility()
            || !self.is_globally_enabled()
            || (self.frustum_culling()
                && !ctx
                    .frustum
                    .map_or(true, |f| f.is_intersects_aabb(&self.world_bounding_box())))
        {
            return RdcControlFlow::Continue;
        }

        if renderer::is_shadow_pass(ctx.render_pass_name) && !self.cast_shadows() {
            return RdcControlFlow::Continue;
        }

        let mut sorted_particles = Vec::new();
        for (i, particle) in self.particles.iter().enumerate() {
            if particle.alive {
                let actual_position = particle.position + self.base.global_position();
                particle
                    .sqr_distance_to_camera
                    .set((*ctx.observer_position - actual_position).norm_squared());
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

        let global_transform = self.global_transform();
        let sort_index = ctx.calculate_sorting_index(self.global_position());

        ctx.storage.push_triangles(
            Vertex::layout(),
            &self.material,
            RenderPath::Forward,
            0,
            sort_index,
            false,
            self.self_handle,
            &mut move |mut vertex_buffer, mut triangle_buffer| {
                let vertices = sorted_particles.iter().flat_map(move |particle_index| {
                    let particle = self.particles.get(*particle_index as usize).unwrap();

                    let position = global_transform
                        .transform_point(&Point3::from(particle.position))
                        .coords;

                    [
                        Vertex {
                            position,
                            tex_coord: Vector2::default(),
                            size: particle.size,
                            rotation: particle.rotation,
                            color: particle.color,
                        },
                        Vertex {
                            position,
                            tex_coord: Vector2::new(1.0, 0.0),
                            size: particle.size,
                            rotation: particle.rotation,
                            color: particle.color,
                        },
                        Vertex {
                            position,
                            tex_coord: Vector2::new(1.0, 1.0),
                            size: particle.size,
                            rotation: particle.rotation,
                            color: particle.color,
                        },
                        Vertex {
                            position,
                            tex_coord: Vector2::new(0.0, 1.0),
                            size: particle.size,
                            rotation: particle.rotation,
                            color: particle.color,
                        },
                    ]
                });

                let triangles = (0..sorted_particles.len()).flat_map(|i| {
                    let base_index = (i * 4) as u32;

                    [
                        TriangleDefinition([base_index, base_index + 1, base_index + 2]),
                        TriangleDefinition([base_index, base_index + 2, base_index + 3]),
                    ]
                });

                let start_vertex_index = vertex_buffer.vertex_count();

                for vertex in vertices {
                    vertex_buffer
                        .push_vertex_raw(value_as_u8_slice(&vertex))
                        .unwrap();
                }

                triangle_buffer.push_triangles_iter_with_offset(start_vertex_index, triangles)
            },
        );

        RdcControlFlow::Continue
    }
}

/// Particle system builder allows you to construct particle system in declarative manner.
/// This is typical implementation of Builder pattern.
pub struct ParticleSystemBuilder {
    base_builder: BaseBuilder,
    emitters: Vec<Emitter>,
    material: MaterialResource,
    acceleration: Vector3<f32>,
    particles: Vec<Particle>,
    color_over_lifetime: ColorGradient,
    is_playing: bool,
    rng: ParticleSystemRng,
}

impl ParticleSystemBuilder {
    /// Creates new builder with default parameters.
    pub fn new(base_builder: BaseBuilder) -> Self {
        Self {
            base_builder,
            emitters: Default::default(),
            material: MaterialResource::new_ok(
                Default::default(),
                Material::standard_particle_system(),
            ),
            particles: Default::default(),
            acceleration: Vector3::new(0.0, -9.81, 0.0),
            color_over_lifetime: Default::default(),
            is_playing: true,
            rng: ParticleSystemRng::default(),
        }
    }

    /// Sets desired emitters for particle system.
    pub fn with_emitters(mut self, emitters: Vec<Emitter>) -> Self {
        self.emitters = emitters;
        self
    }

    /// Sets desired material for particle system.
    pub fn with_material(mut self, material: MaterialResource) -> Self {
        self.material = material;
        self
    }

    /// Sets desired acceleration for particle system.
    pub fn with_acceleration(mut self, acceleration: Vector3<f32>) -> Self {
        self.acceleration = acceleration;
        self
    }

    /// Sets color gradient over lifetime for particle system.
    pub fn with_color_over_lifetime_gradient(mut self, color_over_lifetime: ColorGradient) -> Self {
        self.color_over_lifetime = color_over_lifetime;
        self
    }

    /// Sets an initial set of particles that not belongs to any emitter. This method
    /// could be useful if you need a custom position/velocity/etc. of each particle.
    pub fn with_particles(mut self, particles: Vec<Particle>) -> Self {
        self.particles = particles;
        self
    }

    /// Sets initial particle system state.
    pub fn with_playing(mut self, enabled: bool) -> Self {
        self.is_playing = enabled;
        self
    }

    /// Sets desired pseudo-random numbers generator.
    pub fn with_rng(mut self, rng: ParticleSystemRng) -> Self {
        self.rng = rng;
        self
    }

    fn build_particle_system(self) -> ParticleSystem {
        ParticleSystem {
            base: self.base_builder.build_base(),
            particles: self.particles,
            free_particles: Vec::new(),
            emitters: self.emitters.into(),
            material: self.material.into(),
            acceleration: self.acceleration.into(),
            color_over_lifetime: self.color_over_lifetime.into(),
            is_playing: self.is_playing.into(),
            rng: self.rng,
        }
    }

    /// Creates new instance of particle system.
    pub fn build_node(self) -> Node {
        Node::new(self.build_particle_system())
    }

    /// Creates new instance of particle system and adds it to the graph.
    pub fn build(self, graph: &mut Graph) -> Handle<Node> {
        graph.add_node(self.build_node())
    }
}
