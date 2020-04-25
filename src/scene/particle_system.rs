use crate::{
    core::{
        color::Color,
        color_gradient::ColorGradient,
        math::{vec2::Vec2, vec3::Vec3, TriangleDefinition},
        numeric_range::NumericRange,
        visitor::{Visit, VisitResult, Visitor},
    },
    resource::texture::Texture,
    scene::base::{Base, BaseBuilder},
};
use rand::Rng;
use std::{
    any::Any,
    cell::Cell,
    cmp::Ordering,
    ops::{Deref, DerefMut},
    sync::{Arc, LockResult, Mutex, MutexGuard},
};

/// OpenGL expects this structure packed as in C.
#[repr(C)]
#[derive(Debug)]
pub struct Vertex {
    position: Vec3,
    tex_coord: Vec2,
    size: f32,
    rotation: f32,
    color: Color,
}

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

    pub fn get_vertices(&self) -> &[Vertex] {
        &self.vertices
    }

    pub fn get_triangles(&self) -> &[TriangleDefinition] {
        &self.triangles
    }
}

#[derive(Clone, Debug)]
pub struct Particle {
    pub position: Vec3,
    pub velocity: Vec3,
    pub size: f32,
    alive: bool,
    /// Modifier for size which will be added to size each update tick.
    pub size_modifier: f32,
    /// Particle is alive if lifetime > 0
    lifetime: f32,
    pub initial_lifetime: f32,
    pub rotation_speed: f32,
    pub rotation: f32,
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

pub trait Emit {
    fn emit(&self, emitter: &Emitter, particle_system: &ParticleSystem, particle: &mut Particle);
}

pub struct BoxEmitter {
    half_width: f32,
    half_height: f32,
    half_depth: f32,
}

impl BoxEmitter {
    pub fn new(width: f32, height: f32, depth: f32) -> Self {
        Self {
            half_width: width * 0.5,
            half_height: height * 0.5,
            half_depth: depth * 0.5,
        }
    }
}

impl Default for BoxEmitter {
    fn default() -> Self {
        Self {
            half_width: 0.5,
            half_height: 0.5,
            half_depth: 0.5,
        }
    }
}

impl Emit for BoxEmitter {
    fn emit(&self, emitter: &Emitter, _: &ParticleSystem, particle: &mut Particle) {
        let mut rng = rand::thread_rng();
        particle.position = Vec3::new(
            emitter.position.x + rng.gen_range(-self.half_width, self.half_width),
            emitter.position.y + rng.gen_range(-self.half_height, self.half_height),
            emitter.position.z + rng.gen_range(-self.half_depth, self.half_depth),
        )
    }
}

impl Visit for BoxEmitter {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.half_width.visit("HalfWidth", visitor)?;
        self.half_height.visit("HalfHeight", visitor)?;
        self.half_depth.visit("HalfDepth", visitor)?;

        visitor.leave_region()
    }
}

impl Clone for BoxEmitter {
    fn clone(&self) -> Self {
        Self {
            half_width: 0.0,
            half_height: 0.0,
            half_depth: 0.0,
        }
    }
}

pub struct SphereEmitter {
    radius: f32,
}

impl Default for SphereEmitter {
    fn default() -> Self {
        Self { radius: 0.5 }
    }
}

impl SphereEmitter {
    pub fn new(radius: f32) -> Self {
        Self { radius }
    }
}

impl Visit for SphereEmitter {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.radius.visit("Radius", visitor)?;

        visitor.leave_region()
    }
}

impl Emit for SphereEmitter {
    fn emit(&self, _: &Emitter, _: &ParticleSystem, particle: &mut Particle) {
        let mut rng = rand::thread_rng();
        let phi = rng.gen_range(0.0, std::f32::consts::PI);
        let theta = rng.gen_range(0.0, 2.0 * std::f32::consts::PI);
        let radius = rng.gen_range(0.0, self.radius);
        let cos_theta = theta.cos();
        let sin_theta = theta.sin();
        let cos_phi = phi.cos();
        let sin_phi = phi.sin();
        particle.position = Vec3::new(
            radius * sin_theta * cos_phi,
            radius * sin_theta * sin_phi,
            radius * cos_theta,
        );
    }
}

impl Clone for SphereEmitter {
    fn clone(&self) -> Self {
        Self { radius: 0.0 }
    }
}

pub type CustomEmitterFactoryCallback =
    dyn Fn(i32) -> Result<Box<dyn CustomEmitter>, String> + Send + 'static;

pub struct CustomEmitterFactory {
    callback: Option<Box<CustomEmitterFactoryCallback>>,
}

impl Default for CustomEmitterFactory {
    fn default() -> Self {
        Self { callback: None }
    }
}

impl CustomEmitterFactory {
    pub fn get() -> LockResult<MutexGuard<'static, Self>> {
        CUSTOM_EMITTER_FACTORY_INSTANCE.lock()
    }

    pub fn set_callback(&mut self, call_back: Box<CustomEmitterFactoryCallback>) {
        self.callback = Some(call_back);
    }

    fn spawn(&self, kind: i32) -> Result<Box<dyn CustomEmitter>, String> {
        match &self.callback {
            Some(callback) => callback(kind),
            None => Err(String::from("no callback specified")),
        }
    }
}

lazy_static! {
    static ref CUSTOM_EMITTER_FACTORY_INSTANCE: Mutex<CustomEmitterFactory> =
        Mutex::new(Default::default());
}

pub trait CustomEmitter: Any + Emit + Visit + Send {
    /// Creates boxed copy of custom emitter.
    fn box_clone(&self) -> Box<dyn CustomEmitter>;

    /// Returns unique of custom emitter. Must never be negative!
    /// Negative numbers reserved for built-in kinds.
    fn get_kind(&self) -> i32;
}

pub enum EmitterKind {
    /// Unknown kind here is just to have ability to implement Default trait,
    /// must not be used at runtime!
    Unknown,
    Box(BoxEmitter),
    Sphere(SphereEmitter),
    Custom(Box<dyn CustomEmitter>),
}

impl EmitterKind {
    pub fn new(id: i32) -> Result<Self, String> {
        match id {
            -1 => Ok(EmitterKind::Unknown),
            -2 => Ok(EmitterKind::Box(Default::default())),
            -3 => Ok(EmitterKind::Sphere(Default::default())),
            _ => match CustomEmitterFactory::get() {
                Ok(factory) => Ok(EmitterKind::Custom(factory.spawn(id)?)),
                Err(_) => Err(String::from("Failed get custom emitter factory!")),
            },
        }
    }

    pub fn id(&self) -> i32 {
        match self {
            EmitterKind::Unknown => -1,
            EmitterKind::Box(_) => -2,
            EmitterKind::Sphere(_) => -3,
            EmitterKind::Custom(custom_emitter) => {
                let id = custom_emitter.get_kind();

                if id < 0 {
                    panic!("Negative number for emitter kind are reserved for built-in types!")
                }

                id
            }
        }
    }
}

impl Emit for EmitterKind {
    fn emit(&self, emitter: &Emitter, particle_system: &ParticleSystem, particle: &mut Particle) {
        match self {
            EmitterKind::Unknown => panic!("Unknown emitter kind is not supported"),
            EmitterKind::Box(box_emitter) => box_emitter.emit(emitter, particle_system, particle),
            EmitterKind::Sphere(sphere_emitter) => {
                sphere_emitter.emit(emitter, particle_system, particle)
            }
            EmitterKind::Custom(custom_emitter) => {
                custom_emitter.emit(emitter, particle_system, particle)
            }
        }
    }
}

impl Clone for EmitterKind {
    fn clone(&self) -> Self {
        match self {
            EmitterKind::Unknown => panic!("Unknown emitter kind is not supported"),
            EmitterKind::Box(box_emitter) => EmitterKind::Box(box_emitter.clone()),
            EmitterKind::Sphere(sphere_emitter) => EmitterKind::Sphere(sphere_emitter.clone()),
            EmitterKind::Custom(custom_emitter) => EmitterKind::Custom(custom_emitter.box_clone()),
        }
    }
}

impl Visit for EmitterKind {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        match self {
            EmitterKind::Unknown => panic!("Unknown emitter kind is not supported"),
            EmitterKind::Box(box_emitter) => box_emitter.visit(name, visitor),
            EmitterKind::Sphere(sphere_emitter) => sphere_emitter.visit(name, visitor),
            EmitterKind::Custom(custom_emitter) => custom_emitter.visit(name, visitor),
        }
    }
}

#[derive(Copy, Clone)]
pub enum ParticleLimit {
    Unlimited,
    Strict(u32),
}

impl Visit for ParticleLimit {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        let mut amount = match self {
            ParticleLimit::Unlimited => -1,
            ParticleLimit::Strict(value) => *value as i32,
        };

        amount.visit("Amount", visitor)?;

        if visitor.is_reading() {
            *self = if amount < 0 {
                ParticleLimit::Unlimited
            } else {
                ParticleLimit::Strict(amount as u32)
            };
        }

        visitor.leave_region()
    }
}

pub struct Emitter {
    kind: EmitterKind,
    /// Offset from center of particle system.
    position: Vec3,
    /// Particle spawn rate in unit-per-second. If < 0, spawns `max_particles`,
    /// spawns nothing if `max_particles` < 0
    particle_spawn_rate: u32,
    /// Maximum amount of particles emitter can emit. Unlimited if < 0
    max_particles: ParticleLimit,
    /// Range of initial lifetime of a particle
    lifetime: NumericRange<f32>,
    /// Range of initial size of a particle
    size: NumericRange<f32>,
    /// Range of initial size modifier of a particle
    size_modifier: NumericRange<f32>,
    /// Range of initial X-component of velocity for a particle
    x_velocity: NumericRange<f32>,
    /// Range of initial Y-component of velocity for a particle
    y_velocity: NumericRange<f32>,
    /// Range of initial Z-component of velocity for a particle
    z_velocity: NumericRange<f32>,
    /// Range of initial rotation speed for a particle
    rotation_speed: NumericRange<f32>,
    /// Range of initial rotation for a particle
    rotation: NumericRange<f32>,
    alive_particles: Cell<u32>,
    time: f32,
    particles_to_spawn: usize,
    resurrect_particles: bool,
    spawned_particles: u64,
}

pub struct EmitterBuilder {
    kind: EmitterKind,
    position: Option<Vec3>,
    particle_spawn_rate: Option<u32>,
    max_particles: Option<u32>,
    lifetime: Option<NumericRange<f32>>,
    size: Option<NumericRange<f32>>,
    size_modifier: Option<NumericRange<f32>>,
    x_velocity: Option<NumericRange<f32>>,
    y_velocity: Option<NumericRange<f32>>,
    z_velocity: Option<NumericRange<f32>>,
    rotation_speed: Option<NumericRange<f32>>,
    rotation: Option<NumericRange<f32>>,
    resurrect_particles: bool,
}

impl EmitterBuilder {
    pub fn new(kind: EmitterKind) -> Self {
        Self {
            kind,
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

    pub fn with_position(mut self, position: Vec3) -> Self {
        self.position = Some(position);
        self
    }

    pub fn with_spawn_rate(mut self, rate: u32) -> Self {
        self.particle_spawn_rate = Some(rate);
        self
    }

    pub fn with_max_particles(mut self, value: u32) -> Self {
        self.max_particles = Some(value);
        self
    }

    pub fn with_lifetime_range(mut self, time_range: NumericRange<f32>) -> Self {
        self.lifetime = Some(time_range);
        self
    }

    pub fn with_size_range(mut self, size_range: NumericRange<f32>) -> Self {
        self.size = Some(size_range);
        self
    }

    pub fn with_size_modifier_range(mut self, mod_range: NumericRange<f32>) -> Self {
        self.size_modifier = Some(mod_range);
        self
    }

    pub fn with_x_velocity_range(mut self, x_vel_range: NumericRange<f32>) -> Self {
        self.x_velocity = Some(x_vel_range);
        self
    }

    pub fn with_y_velocity_range(mut self, y_vel_range: NumericRange<f32>) -> Self {
        self.y_velocity = Some(y_vel_range);
        self
    }

    pub fn with_z_velocity_range(mut self, z_vel_range: NumericRange<f32>) -> Self {
        self.z_velocity = Some(z_vel_range);
        self
    }

    pub fn with_rotation_speed_range(mut self, speed_range: NumericRange<f32>) -> Self {
        self.rotation_speed = Some(speed_range);
        self
    }

    pub fn with_rotation_range(mut self, angle_range: NumericRange<f32>) -> Self {
        self.rotation = Some(angle_range);
        self
    }

    pub fn resurrect_particles(mut self, value: bool) -> Self {
        self.resurrect_particles = value;
        self
    }

    pub fn build(self) -> Emitter {
        Emitter {
            kind: self.kind,
            position: self.position.unwrap_or(Vec3::ZERO),
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

impl Emitter {
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

    pub fn emit(&self, particle_system: &ParticleSystem, particle: &mut Particle) {
        particle.lifetime = 0.0;
        particle.initial_lifetime = self.lifetime.random();
        particle.color = Color::WHITE;
        particle.size = self.size.random();
        particle.size_modifier = self.size_modifier.random();
        particle.velocity = Vec3::new(
            self.x_velocity.random(),
            self.y_velocity.random(),
            self.z_velocity.random(),
        );
        particle.rotation = self.rotation.random();
        particle.rotation_speed = self.rotation_speed.random();
        self.kind.emit(self, particle_system, particle);
    }

    pub fn set_position(&mut self, position: Vec3) -> &mut Self {
        self.position = position;
        self
    }

    pub fn position(&self) -> Vec3 {
        self.position
    }

    pub fn set_spawn_rate(&mut self, rate: u32) -> &mut Self {
        self.particle_spawn_rate = rate;
        self
    }

    pub fn spawn_rate(&self) -> u32 {
        self.particle_spawn_rate
    }

    pub fn set_max_particles(&mut self, max: ParticleLimit) -> &mut Self {
        self.max_particles = max;
        self
    }

    pub fn max_particles(&self) -> ParticleLimit {
        self.max_particles
    }

    pub fn set_life_time_range(&mut self, range: NumericRange<f32>) -> &mut Self {
        self.lifetime = range;
        self
    }

    pub fn life_time_range(&self) -> NumericRange<f32> {
        self.lifetime
    }

    pub fn set_size_range(&mut self, range: NumericRange<f32>) -> &mut Self {
        self.size = range;
        self
    }

    pub fn size_range(&self) -> NumericRange<f32> {
        self.size
    }

    pub fn set_size_modifier_range(&mut self, range: NumericRange<f32>) -> &mut Self {
        self.size_modifier = range;
        self
    }

    pub fn size_modifier_range(&self) -> NumericRange<f32> {
        self.size_modifier
    }

    pub fn set_x_velocity_range(&mut self, range: NumericRange<f32>) -> &mut Self {
        self.x_velocity = range;
        self
    }

    pub fn x_velocity_range(&self) -> NumericRange<f32> {
        self.x_velocity
    }

    pub fn set_y_velocity_range(&mut self, range: NumericRange<f32>) -> &mut Self {
        self.y_velocity = range;
        self
    }

    pub fn y_velocity_range(&self) -> NumericRange<f32> {
        self.y_velocity
    }

    pub fn set_z_velocity_range(&mut self, range: NumericRange<f32>) -> &mut Self {
        self.z_velocity = range;
        self
    }

    pub fn z_velocity_range(&self) -> NumericRange<f32> {
        self.z_velocity
    }

    pub fn set_rotation_speed_range(&mut self, range: NumericRange<f32>) -> &mut Self {
        self.rotation_speed = range;
        self
    }

    pub fn rotation_speed_range(&self) -> NumericRange<f32> {
        self.rotation_speed
    }

    pub fn set_rotation_range(&mut self, range: NumericRange<f32>) -> &mut Self {
        self.rotation = range;
        self
    }

    pub fn rotation_range(&self) -> NumericRange<f32> {
        self.rotation
    }

    pub fn enable_particle_resurrection(&mut self, state: bool) -> &mut Self {
        self.resurrect_particles = state;
        self
    }

    pub fn is_particles_resurrects(&self) -> bool {
        self.resurrect_particles
    }

    pub fn spawned_particles(&self) -> u64 {
        self.spawned_particles
    }
}

impl Visit for Emitter {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        let mut kind_id: i32 = self.kind.id();
        kind_id.visit("KindId", visitor)?;
        if visitor.is_reading() {
            self.kind = EmitterKind::new(kind_id)?;
        }

        self.kind.visit("Kind", visitor)?;
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

impl Clone for Emitter {
    fn clone(&self) -> Self {
        Self {
            kind: self.kind.clone(),
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

impl Default for Emitter {
    fn default() -> Self {
        Self {
            kind: EmitterKind::Unknown,
            position: Vec3::ZERO,
            particle_spawn_rate: 0,
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

#[derive(Clone)]
pub struct ParticleSystem {
    base: Base,
    particles: Vec<Particle>,
    free_particles: Vec<u32>,
    emitters: Vec<Emitter>,
    texture: Option<Arc<Mutex<Texture>>>,
    acceleration: Vec3,
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
    pub fn add_emitter(&mut self, emitter: Emitter) {
        self.emitters.push(emitter)
    }

    pub fn acceleration(&mut self, accel: Vec3) {
        self.acceleration = accel;
    }

    pub fn color_over_lifetime_gradient(&mut self, gradient: ColorGradient) {
        self.color_over_lifetime = Some(gradient)
    }

    pub fn update(&mut self, dt: f32) {
        for emitter in self.emitters.iter_mut() {
            emitter.tick(dt);
        }

        for (i, emitter) in self.emitters.iter().enumerate() {
            for _ in 0..emitter.particles_to_spawn {
                let mut particle = Particle::default();
                particle.emitter_index = i as u32;
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
                    particle.rotation += particle.rotation_speed;
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

    pub fn generate_draw_data(
        &self,
        sorted_particles: &mut Vec<u32>,
        draw_data: &mut DrawData,
        camera_pos: &Vec3,
    ) {
        sorted_particles.clear();
        for (i, particle) in self.particles.iter().enumerate() {
            if particle.alive {
                let actual_position = particle.position + self.base.global_position();
                particle
                    .sqr_distance_to_camera
                    .set(camera_pos.sqr_distance(&actual_position));
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
                tex_coord: Vec2::ZERO,
                size: particle.size,
                rotation: particle.rotation,
                color: particle.color,
            });

            draw_data.vertices.push(Vertex {
                position: particle.position,
                tex_coord: Vec2::new(1.0, 0.0),
                size: particle.size,
                rotation: particle.rotation,
                color: particle.color,
            });

            draw_data.vertices.push(Vertex {
                position: particle.position,
                tex_coord: Vec2::new(1.0, 1.0),
                size: particle.size,
                rotation: particle.rotation,
                color: particle.color,
            });

            draw_data.vertices.push(Vertex {
                position: particle.position,
                tex_coord: Vec2::new(0.0, 1.0),
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

    pub fn set_texture(&mut self, texture: Arc<Mutex<Texture>>) {
        self.texture = Some(texture)
    }

    pub fn texture(&self) -> Option<Arc<Mutex<Texture>>> {
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
        ParticleSystemBuilder::new(BaseBuilder::new()).build()
    }
}

pub struct ParticleSystemBuilder {
    base_builder: BaseBuilder,
    emitters: Option<Vec<Emitter>>,
    texture: Option<Arc<Mutex<Texture>>>,
    acceleration: Option<Vec3>,
    color_over_lifetime: Option<ColorGradient>,
}

impl ParticleSystemBuilder {
    pub fn new(base_builder: BaseBuilder) -> Self {
        Self {
            base_builder,
            emitters: None,
            texture: None,
            acceleration: None,
            color_over_lifetime: None,
        }
    }

    pub fn with_emitters(mut self, emitters: Vec<Emitter>) -> Self {
        self.emitters = Some(emitters);
        self
    }

    pub fn with_texture(mut self, texture: Arc<Mutex<Texture>>) -> Self {
        self.texture = Some(texture);
        self
    }

    pub fn with_opt_texture(mut self, texture: Option<Arc<Mutex<Texture>>>) -> Self {
        self.texture = texture;
        self
    }

    pub fn with_acceleration(mut self, acceleration: Vec3) -> Self {
        self.acceleration = Some(acceleration);
        self
    }

    pub fn with_color_over_lifetime_gradient(mut self, color_over_lifetime: ColorGradient) -> Self {
        self.color_over_lifetime = Some(color_over_lifetime);
        self
    }

    pub fn build(self) -> ParticleSystem {
        ParticleSystem {
            base: self.base_builder.build(),
            particles: Vec::new(),
            free_particles: Vec::new(),
            emitters: self.emitters.unwrap_or_default(),
            texture: self.texture.clone(),
            acceleration: self
                .acceleration
                .unwrap_or_else(|| Vec3::new(0.0, -9.81, 0.0)),
            color_over_lifetime: self.color_over_lifetime,
        }
    }
}
