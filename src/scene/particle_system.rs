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
//! use rg3d::scene::particle_system::{SphereEmitter, ParticleSystemBuilder, Emitter, BaseEmitterBuilder, SphereEmitterBuilder};
//! use rg3d::engine::resource_manager::ResourceManager;
//! use rg3d::core::algebra::Vector3;
//! use rg3d::scene::graph::Graph;
//! use rg3d::scene::node::Node;
//! use rg3d::scene::transform::TransformBuilder;
//! use rg3d::core::color_gradient::{GradientPoint, ColorGradient};
//! use rg3d::core::numeric_range::NumericRange;
//! use rg3d::scene::base::BaseBuilder;
//! use rg3d::core::color::Color;
//! use std::path::Path;
//! use rg3d::resource::texture::TexturePixelKind;
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
//!                 .with_x_velocity_range(NumericRange::new(-0.01, 0.01))
//!                 .with_y_velocity_range(NumericRange::new(0.02, 0.03))
//!                 .with_z_velocity_range(NumericRange::new(-0.01, 0.01)))
//!                 .with_radius(0.01)
//!                 .build()
//!         ])
//!         .with_texture(resource_manager.request_texture(Path::new("data/particles/smoke_04.tga")))
//!         .build(graph);
//! }
//! ```

use crate::{
    core::{
        algebra::{Vector2, Vector3},
        color::Color,
        color_gradient::ColorGradient,
        math::TriangleDefinition,
        numeric_range::NumericRange,
        pool::Handle,
        visitor::{Visit, VisitResult, Visitor},
    },
    resource::texture::Texture,
    scene::{
        base::{Base, BaseBuilder},
        graph::Graph,
        node::Node,
    },
};
use std::{
    cell::Cell,
    cmp::Ordering,
    fmt::Debug,
    ops::{Deref, DerefMut},
};

/// OpenGL expects this structure packed as in C.
#[repr(C)]
#[derive(Debug)]
pub struct Vertex {
    position: Vector3<f32>,
    tex_coord: Vector2<f32>,
    size: f32,
    rotation: f32,
    color: Color,
}

/// Particle system is "rendered" into special buffer, which contains vertices and faces.
pub struct DrawData {
    vertices: Vec<Vertex>,
    triangles: Vec<TriangleDefinition>,
}

impl Default for DrawData {
    fn default() -> Self {
        Self {
            vertices: Vec::new(),
            triangles: Vec::new(),
        }
    }
}

impl DrawData {
    fn clear(&mut self) {
        self.vertices.clear();
        self.triangles.clear();
    }

    /// Returns shared reference to array of vertices.
    pub fn vertices(&self) -> &[Vertex] {
        &self.vertices
    }

    /// Returns shared reference to array of triangles.
    pub fn triangles(&self) -> &[TriangleDefinition] {
        &self.triangles
    }
}

/// Particle is a quad with texture and various other parameters, such as
/// position, velocity, size, lifetime, etc.
#[derive(Clone, Debug)]
pub struct Particle {
    /// Position of particle in local coordinates.
    pub position: Vector3<f32>,
    /// Velocity of particle in local coordinates.
    pub velocity: Vector3<f32>,
    /// Size of particle.
    pub size: f32,
    alive: bool,
    /// Modifier for size which will be added to size each update tick.
    pub size_modifier: f32,
    /// Particle is alive if lifetime > 0
    lifetime: f32,
    /// Lifetime at the moment when particle was created.
    pub initial_lifetime: f32,
    /// Rotation speed of particle in radians per second.
    pub rotation_speed: f32,
    /// Rotation angle in radians.
    pub rotation: f32,
    /// Color of particle.
    pub color: Color,
    emitter_index: u32,
    sqr_distance_to_camera: Cell<f32>,
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

impl Visit for Particle {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.position.visit("Pos", visitor)?;
        self.velocity.visit("Vel", visitor)?;
        self.size.visit("Size", visitor)?;
        self.alive.visit("Alive", visitor)?;
        self.size_modifier.visit("SizeMod", visitor)?;
        self.lifetime.visit("LifeTime", visitor)?;
        self.initial_lifetime.visit("InitLifeTime", visitor)?;
        self.rotation_speed.visit("RotSpeed", visitor)?;
        self.rotation.visit("Rotation", visitor)?;
        self.color.visit("Color", visitor)?;
        self.emitter_index.visit("EmitterIndex", visitor)?;

        visitor.leave_region()
    }
}

/// Emit trait must be implemented for any particle system emitter.
pub trait Emit {
    /// Initializes state of particle using given emitter and particle system.
    fn emit(&self, particle_system: &ParticleSystem, particle: &mut Particle);
}

/// Box emitter emits particles uniformly in its volume. Can be used to create simple fog
/// layer.
#[derive(Debug, Clone)]
pub struct BoxEmitter {
    emitter: BaseEmitter,
    half_width: f32,
    half_height: f32,
    half_depth: f32,
}

impl Deref for BoxEmitter {
    type Target = BaseEmitter;

    fn deref(&self) -> &Self::Target {
        &self.emitter
    }
}

impl DerefMut for BoxEmitter {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.emitter
    }
}

impl BoxEmitter {
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

impl Default for BoxEmitter {
    fn default() -> Self {
        Self {
            emitter: Default::default(),
            half_width: 0.5,
            half_height: 0.5,
            half_depth: 0.5,
        }
    }
}

impl Emit for BoxEmitter {
    fn emit(&self, _particle_system: &ParticleSystem, particle: &mut Particle) {
        self.emitter.emit(particle);
        particle.position = Vector3::new(
            self.position.x + NumericRange::new(-self.half_width, self.half_width).random(),
            self.position.y + NumericRange::new(-self.half_height, self.half_height).random(),
            self.position.z + NumericRange::new(-self.half_depth, self.half_depth).random(),
        )
    }
}

impl Visit for BoxEmitter {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.emitter.visit("Emitter", visitor)?;
        self.half_width.visit("HalfWidth", visitor)?;
        self.half_height.visit("HalfHeight", visitor)?;
        self.half_depth.visit("HalfDepth", visitor)?;

        visitor.leave_region()
    }
}

/// Vertical cylinder emitter.
#[derive(Clone, Debug)]
pub struct CylinderEmitter {
    emitter: BaseEmitter,
    height: f32,
    radius: f32,
}

impl Default for CylinderEmitter {
    fn default() -> Self {
        Self {
            emitter: Default::default(),
            height: 1.0,
            radius: 0.5,
        }
    }
}

impl Deref for CylinderEmitter {
    type Target = BaseEmitter;

    fn deref(&self) -> &Self::Target {
        &self.emitter
    }
}

impl DerefMut for CylinderEmitter {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.emitter
    }
}

impl Visit for CylinderEmitter {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.emitter.visit("Emitter", visitor)?;
        self.radius.visit("Radius", visitor)?;
        self.height.visit("Height", visitor)?;

        visitor.leave_region()
    }
}

impl Emit for CylinderEmitter {
    fn emit(&self, _particle_system: &ParticleSystem, particle: &mut Particle) {
        // Disk point picking extended in 3D - http://mathworld.wolfram.com/DiskPointPicking.html
        let scale: f32 = NumericRange::new(0.0, 1.0).random();
        let theta = NumericRange::new(0.0, 2.0 * std::f32::consts::PI).random();
        let z = NumericRange::new(0.0, self.height).random();
        let radius = scale.sqrt() * self.radius;
        let x = radius * theta.cos();
        let y = radius * theta.sin();
        particle.position = self.position + Vector3::new(x, y, z);
    }
}

impl CylinderEmitter {
    /// Returns radius of the cylinder emitter.
    pub fn radius(&self) -> f32 {
        self.radius
    }

    /// Sets radius of the cylinder emitter.
    pub fn set_radius(&mut self, radius: f32) {
        self.radius = radius.max(0.0);
    }

    /// Returns height of the cylinder emitter.
    pub fn height(&self) -> f32 {
        self.height
    }

    /// Sets height of the cylinder emitter.
    pub fn set_height(&mut self, height: f32) {
        self.height = height.max(0.0);
    }
}

/// Box emitter builder allows you to construct cylinder emitter in declarative manner.
/// This is typical implementation of Builder pattern.
pub struct CylinderEmitterBuilder {
    base: BaseEmitterBuilder,
    height: f32,
    radius: f32,
}

impl CylinderEmitterBuilder {
    /// Creates new cylinder emitter builder.
    pub fn new(base: BaseEmitterBuilder) -> Self {
        Self {
            base,
            height: 1.0,
            radius: 0.5,
        }
    }

    /// Sets desired height of the emitter.
    pub fn with_height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    /// Sets desired radius of the emitter.
    pub fn with_radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    /// Creates new cylinder emitter with given parameters.
    pub fn build(self) -> Emitter {
        Emitter::Cylinder(CylinderEmitter {
            emitter: self.base.build(),
            height: self.height,
            radius: self.radius,
        })
    }
}

/// Box emitter builder allows you to construct box emitter in declarative manner.
/// This is typical implementation of Builder pattern.
pub struct BoxEmitterBuilder {
    base: BaseEmitterBuilder,
    width: f32,
    height: f32,
    depth: f32,
}

impl BoxEmitterBuilder {
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
        Emitter::Box(BoxEmitter {
            emitter: self.base.build(),
            half_width: self.width * 0.5,
            half_height: self.height * 0.5,
            half_depth: self.depth * 0.5,
        })
    }
}

/// Sphere emitter uniformly places particles in spherical volume. Can be used with
/// radius = 0, then it represents point emitter.   
#[derive(Debug, Clone)]
pub struct SphereEmitter {
    emitter: BaseEmitter,
    radius: f32,
}

impl Deref for SphereEmitter {
    type Target = BaseEmitter;

    fn deref(&self) -> &Self::Target {
        &self.emitter
    }
}

impl DerefMut for SphereEmitter {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.emitter
    }
}

impl Default for SphereEmitter {
    fn default() -> Self {
        Self {
            emitter: BaseEmitter::default(),
            radius: 0.5,
        }
    }
}

impl SphereEmitter {
    /// Creates new sphere emitter with given radius.
    pub fn new(emitter: BaseEmitter, radius: f32) -> Self {
        Self { emitter, radius }
    }

    /// Returns current radius.
    pub fn radius(&self) -> f32 {
        self.radius
    }

    /// Sets new sphere radius.
    pub fn set_radius(&mut self, radius: f32) {
        self.radius = radius.max(0.0);
    }
}

impl Visit for SphereEmitter {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.emitter.visit("Emitter", visitor)?;
        self.radius.visit("Radius", visitor)?;

        visitor.leave_region()
    }
}

impl Emit for SphereEmitter {
    fn emit(&self, _particle_system: &ParticleSystem, particle: &mut Particle) {
        self.emitter.emit(particle);
        let phi = NumericRange::new(0.0, std::f32::consts::PI).random();
        let theta = NumericRange::new(0.0, 2.0 * std::f32::consts::PI).random();
        let radius = NumericRange::new(0.0, self.radius).random();
        let cos_theta = theta.cos();
        let sin_theta = theta.sin();
        let cos_phi = phi.cos();
        let sin_phi = phi.sin();
        particle.position = self.position
            + Vector3::new(
                radius * sin_theta * cos_phi,
                radius * sin_theta * sin_phi,
                radius * cos_theta,
            );
    }
}

/// Sphere emitter builder allows you to construct sphere emitter in declarative manner.
/// This is typical implementation of Builder pattern.
pub struct SphereEmitterBuilder {
    base: BaseEmitterBuilder,
    radius: f32,
}

impl SphereEmitterBuilder {
    /// Creates new sphere emitter builder with 0.5 radius.
    pub fn new(base: BaseEmitterBuilder) -> Self {
        Self { base, radius: 0.5 }
    }

    /// Sets desired radius of sphere emitter.
    pub fn with_radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    /// Creates new sphere emitter.
    pub fn build(self) -> Emitter {
        Emitter::Sphere(SphereEmitter {
            emitter: self.base.build(),
            radius: self.radius,
        })
    }
}

/// Emitter is an enum over all possible emitter types, they all must
/// use BaseEmitter which contains base functionality.
#[derive(Debug)]
pub enum Emitter {
    /// Unknown kind here is just to have ability to implement Default trait,
    /// must not be used at runtime!
    Unknown,
    /// See BoxEmitter docs.
    Box(BoxEmitter),
    /// See SphereEmitter docs.
    Sphere(SphereEmitter),
    /// Cylinder emitter.
    Cylinder(CylinderEmitter),
}

impl Emitter {
    /// Creates new emitter from given id.
    pub fn new(id: i32) -> Result<Self, String> {
        match id {
            0 => Ok(Self::Unknown),
            1 => Ok(Self::Box(Default::default())),
            2 => Ok(Self::Sphere(Default::default())),
            3 => Ok(Self::Cylinder(Default::default())),
            _ => Err(format!("Invalid emitter id {}!", id)),
        }
    }

    /// Returns id of current emitter kind.
    pub fn id(&self) -> i32 {
        match self {
            Self::Unknown => 0,
            Self::Box(_) => 1,
            Self::Sphere(_) => 2,
            Self::Cylinder(_) => 3,
        }
    }
}

macro_rules! static_dispatch {
    ($self:ident, $func:ident, $($args:expr),*) => {
        match $self {
            Emitter::Unknown => panic!("Unknown emitter must not be used!"),
            Emitter::Box(v) => v.$func($($args),*),
            Emitter::Sphere(v) => v.$func($($args),*),
            Emitter::Cylinder(v) => v.$func($($args),*),
        }
    };
}

impl Emit for Emitter {
    fn emit(&self, particle_system: &ParticleSystem, particle: &mut Particle) {
        static_dispatch!(self, emit, particle_system, particle)
    }
}

impl Clone for Emitter {
    fn clone(&self) -> Self {
        match self {
            Self::Unknown => panic!("Unknown emitter kind is not supported"),
            Self::Box(box_emitter) => Self::Box(box_emitter.clone()),
            Self::Sphere(sphere_emitter) => Self::Sphere(sphere_emitter.clone()),
            Self::Cylinder(cylinder) => Self::Cylinder(cylinder.clone()),
        }
    }
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
        Self::Unknown
    }
}

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

/// Base emitter contains properties for all other "derived" emitters.
#[derive(Debug)]
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
    alive_particles: Cell<u32>,
    time: f32,
    particles_to_spawn: usize,
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
        self.particles_to_spawn = particle_count as usize;
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

impl Visit for BaseEmitter {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.position.visit("Position", visitor)?;
        self.particle_spawn_rate.visit("SpawnRate", visitor)?;
        self.max_particles.visit("MaxParticles", visitor)?;
        self.lifetime.visit("LifeTime", visitor)?;
        self.size.visit("Size", visitor)?;
        self.size_modifier.visit("SizeModifier", visitor)?;
        self.x_velocity.visit("XVelocity", visitor)?;
        self.y_velocity.visit("YVelocity", visitor)?;
        self.z_velocity.visit("ZVelocity", visitor)?;
        self.rotation_speed.visit("RotationSpeed", visitor)?;
        self.rotation.visit("Rotation", visitor)?;
        self.alive_particles.visit("AliveParticles", visitor)?;
        self.time.visit("Time", visitor)?;
        self.resurrect_particles
            .visit("ResurrectParticles", visitor)?;
        self.spawned_particles.visit("SpawnedParticles", visitor)?;

        visitor.leave_region()
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

/// See module docs.
#[derive(Debug)]
pub struct ParticleSystem {
    base: Base,
    particles: Vec<Particle>,
    free_particles: Vec<u32>,
    /// List of emitters of the particle system.
    pub emitters: Vec<Emitter>,
    texture: Option<Texture>,
    acceleration: Vector3<f32>,
    color_over_lifetime: Option<ColorGradient>,
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
            acceleration: self.acceleration,
            color_over_lifetime: self.color_over_lifetime.clone(),
        }
    }

    /// Returns current acceleration for particles in particle system.
    pub fn acceleration(&self) -> Vector3<f32> {
        self.acceleration
    }

    /// Set new acceleration that will be applied to all particles,
    /// can be used to change "gravity" vector of particles.
    pub fn set_acceleration(&mut self, accel: Vector3<f32>) {
        self.acceleration = accel;
    }

    /// Sets new "color curve" that will evaluate color over lifetime.
    pub fn set_color_over_lifetime_gradient(&mut self, gradient: ColorGradient) {
        self.color_over_lifetime = Some(gradient)
    }

    /// Removes all generated particles.
    pub fn clear_particles(&mut self) {
        self.particles.clear();
        self.free_particles.clear();
        for emitter in self.emitters.iter_mut() {
            emitter.alive_particles.set(0);
        }
    }

    /// Updates state of particle system, this means that it moves particles,
    /// changes their color, size, rotation, etc. This method should not be
    /// used directly, it will be automatically called by scene update.
    pub fn update(&mut self, dt: f32) {
        for emitter in self.emitters.iter_mut() {
            emitter.tick(dt);
        }

        for (i, emitter) in self.emitters.iter().enumerate() {
            for _ in 0..emitter.particles_to_spawn {
                let mut particle = Particle {
                    emitter_index: i as u32,
                    ..Particle::default()
                };
                emitter
                    .alive_particles
                    .set(emitter.alive_particles.get() + 1);
                emitter.emit(self, &mut particle);
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
                    if let Some(emitter) = self.emitters.get(particle.emitter_index as usize) {
                        emitter
                            .alive_particles
                            .set(emitter.alive_particles.get() - 1);
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
                    if let Some(color_over_lifetime) = &self.color_over_lifetime {
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

            draw_data.vertices.push(Vertex {
                position: particle.position,
                tex_coord: Vector2::default(),
                size: particle.size,
                rotation: particle.rotation,
                color: particle.color,
            });

            draw_data.vertices.push(Vertex {
                position: particle.position,
                tex_coord: Vector2::new(1.0, 0.0),
                size: particle.size,
                rotation: particle.rotation,
                color: particle.color,
            });

            draw_data.vertices.push(Vertex {
                position: particle.position,
                tex_coord: Vector2::new(1.0, 1.0),
                size: particle.size,
                rotation: particle.rotation,
                color: particle.color,
            });

            draw_data.vertices.push(Vertex {
                position: particle.position,
                tex_coord: Vector2::new(0.0, 1.0),
                size: particle.size,
                rotation: particle.rotation,
                color: particle.color,
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
        self.texture = texture
    }

    /// Returns current texture used by particle system.
    pub fn texture(&self) -> Option<Texture> {
        self.texture.clone()
    }
}

impl Visit for ParticleSystem {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.particles.visit("Particles", visitor)?;
        self.free_particles.visit("FreeParticles", visitor)?;
        self.texture.visit("Texture", visitor)?;
        self.emitters.visit("Emitters", visitor)?;
        self.acceleration.visit("Acceleration", visitor)?;
        self.color_over_lifetime.visit("ColorGradient", visitor)?;
        self.base.visit("Base", visitor)?;

        visitor.leave_region()
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
    color_over_lifetime: Option<ColorGradient>,
}

impl ParticleSystemBuilder {
    /// Creates new builder with default parameters.
    pub fn new(base_builder: BaseBuilder) -> Self {
        Self {
            base_builder,
            emitters: Default::default(),
            texture: None,
            acceleration: Vector3::new(0.0, -9.81, 0.0),
            color_over_lifetime: None,
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

    fn build_particle_system(self) -> ParticleSystem {
        ParticleSystem {
            base: self.base_builder.build_base(),
            particles: Vec::new(),
            free_particles: Vec::new(),
            emitters: self.emitters,
            texture: self.texture.clone(),
            acceleration: self.acceleration,
            color_over_lifetime: self.color_over_lifetime,
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
