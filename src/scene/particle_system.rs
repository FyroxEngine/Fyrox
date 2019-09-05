use crate::{
    math::{
        vec3::Vec3,
        vec2::Vec2,
    },
    utils::{
        visitor::{Visit, Visitor, VisitResult},
        color_gradient::ColorGradient,
    },
    gui::draw::Color,
    resource::Resource,
    utils::visitor::VisitError,
};
use std::{
    cell::{
        RefCell,
        Cell,
    },
    rc::Rc,
    cmp::Ordering,
};
use std::mem::size_of;
use rand::Rng;

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
    indices: Vec<u32>,
}

impl Default for DrawData {
    fn default() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
        }
    }
}

impl DrawData {
    fn clear(&mut self) {
        self.vertices.clear();
        self.indices.clear();
    }

    pub fn get_vertices(&self) -> &[Vertex] {
        &self.vertices
    }

    pub fn get_indices(&self) -> &[u32] {
        &self.indices
    }

    pub fn vertex_size() -> i32 {
        size_of::<Vertex>() as i32
    }

    pub fn index_size() -> i32 {
        size_of::<u32>() as i32
    }
}

#[derive(Clone, Debug)]
pub struct Particle {
    position: Vec3,
    velocity: Vec3,
    size: f32,
    alive: bool,
    /// Modifier for size which will be added to size each update tick.
    size_modifier: f32,
    /// Particle is alive if lifetime > 0
    lifetime: f32,
    initial_lifetime: f32,
    rotation_speed: f32,
    rotation: f32,
    color: Color,
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
            color: Color::white(),
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
            half_depth: 0.5
        }
    }
}

impl Emit for BoxEmitter {
    fn emit(&self, emitter: &Emitter, _: &ParticleSystem, particle: &mut Particle) {
        let mut rng = rand::thread_rng();
        particle.position = Vec3::make(
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
        Self {
            radius: 0.5
        }
    }
}

impl SphereEmitter {
    pub fn new(radius: f32) -> Self {
        Self {
            radius
        }
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
        particle.position = Vec3::make(
            radius * sin_theta * cos_phi,
            radius * sin_theta * sin_phi,
            radius * cos_theta,
        );
    }
}

impl Clone for SphereEmitter {
    fn clone(&self) -> Self {
        Self {
            radius: 0.0
        }
    }
}

pub enum EmitterKind {
    /// Unknown kind here is just to have ability to implement Default trait,
    /// must not be used at runtime!
    Unknown,
    Box(BoxEmitter),
    Sphere(SphereEmitter),
}

impl Emit for EmitterKind {
    fn emit(&self, emitter: &Emitter, particle_system: &ParticleSystem, particle: &mut Particle) {
        match self {
            EmitterKind::Unknown => (),
            EmitterKind::Box(box_emitter) => box_emitter.emit(emitter, particle_system, particle),
            EmitterKind::Sphere(sphere_emitter) => sphere_emitter.emit(emitter, particle_system, particle),
        }
    }
}

impl Clone for EmitterKind {
    fn clone(&self) -> Self {
        match self {
            EmitterKind::Unknown => EmitterKind::Unknown,
            EmitterKind::Box(box_emitter) => EmitterKind::Box(box_emitter.clone()),
            EmitterKind::Sphere(sphere_emitter) => EmitterKind::Sphere(sphere_emitter.clone()),
        }
    }
}

impl Visit for EmitterKind {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        let mut kind: u8 = match self {
            EmitterKind::Unknown => 0,
            EmitterKind::Box(_) => 1,
            EmitterKind::Sphere(_) => 2,
        };
        kind.visit("Id", visitor)?;

        if visitor.is_reading() {
            *self = match kind {
                0 => EmitterKind::Unknown,
                1 => EmitterKind::Box(Default::default()),
                2 => EmitterKind::Sphere(Default::default()),
                _ => return Err(VisitError::User(format!("invalid emitter kind id {}", kind)))
            }
        }

        match self {
            EmitterKind::Unknown => (),
            EmitterKind::Box(box_emitter) => box_emitter.visit("Data", visitor)?,
            EmitterKind::Sphere(sphere_emitter) => sphere_emitter.visit("Data", visitor)?
        }

        visitor.leave_region()
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
    min_lifetime: f32,
    max_lifetime: f32,
    /// Range of initial size of a particle
    min_size: f32,
    max_size: f32,
    /// Range of initial size modifier of a particle
    min_size_modifier: f32,
    max_size_modifier: f32,
    /// Range of initial X-component of velocity for a particle
    min_x_velocity: f32,
    max_x_velocity: f32,
    /// Range of initial Y-component of velocity for a particle
    min_y_velocity: f32,
    max_y_velocity: f32,
    /// Range of initial Z-component of velocity for a particle
    min_z_velocity: f32,
    max_z_velocity: f32,
    /// Range of initial rotation speed for a particle
    min_rotation_speed: f32,
    max_rotation_speed: f32,
    /// Range of initial rotation for a particle
    min_rotation: f32,
    max_rotation: f32,
    alive_particles: Cell<u32>,
    time: f32,
    particles_to_spawn: usize,
}

impl Emitter {
    pub fn new(kind: EmitterKind) -> Self {
        Self {
            kind,
            position: Vec3::zero(),
            particle_spawn_rate: 25,
            max_particles: ParticleLimit::Unlimited,
            min_lifetime: 5.0,
            max_lifetime: 10.0,
            min_size: 0.125,
            max_size: 0.250,
            min_size_modifier: 0.0005,
            max_size_modifier: 0.0010,
            min_x_velocity: -0.001,
            max_x_velocity: 0.001,
            min_y_velocity: -0.001,
            max_y_velocity: 0.001,
            min_z_velocity: -0.001,
            max_z_velocity: 0.001,
            min_rotation_speed: -0.02,
            max_rotation_speed: 0.02,
            min_rotation: -std::f32::consts::PI,
            max_rotation: std::f32::consts::PI,
            alive_particles: Cell::new(0),
            time: 0.0,
            particles_to_spawn: 0,
        }
    }

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
        }
        self.particles_to_spawn = particle_count as usize;
    }

    pub fn emit(&self, particle_system: &ParticleSystem, particle: &mut Particle) {
        let mut rng = rand::thread_rng();
        particle.lifetime = 0.0;
        particle.initial_lifetime = rng.gen_range(self.min_lifetime, self.max_lifetime);
        particle.color = Color::white();
        particle.size = rng.gen_range(self.min_size, self.max_size);
        particle.size_modifier = rng.gen_range(self.min_size_modifier, self.max_size_modifier);
        particle.velocity = Vec3::make(
            rng.gen_range(self.min_x_velocity, self.max_x_velocity),
            rng.gen_range(self.min_y_velocity, self.max_y_velocity),
            rng.gen_range(self.min_z_velocity, self.max_z_velocity),
        );
        particle.rotation = rng.gen_range(self.min_rotation, self.max_rotation);
        particle.rotation_speed = rng.gen_range(self.min_rotation_speed, self.max_rotation_speed);
        self.kind.emit(self, particle_system, particle);
    }
}

impl Visit for Emitter {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.kind.visit("Kind", visitor)?;
        self.position.visit("Position", visitor)?;
        self.particle_spawn_rate.visit("SpawnRate", visitor)?;
        self.max_particles.visit("MaxParticles", visitor)?;
        self.min_lifetime.visit("MinLifeTime", visitor)?;
        self.max_lifetime.visit("MaxLifeTime", visitor)?;
        self.min_size.visit("MinSize", visitor)?;
        self.max_size.visit("MaxSize", visitor)?;
        self.min_size_modifier.visit("MinSizeModifier", visitor)?;
        self.max_size_modifier.visit("MaxSizeModifier", visitor)?;
        self.min_x_velocity.visit("MinXVelocity", visitor)?;
        self.max_x_velocity.visit("MaxXVelocity", visitor)?;
        self.min_y_velocity.visit("MinYVelocity", visitor)?;
        self.max_y_velocity.visit("MaxYVelocity", visitor)?;
        self.min_z_velocity.visit("MinZVelocity", visitor)?;
        self.max_z_velocity.visit("MaxZVelocity", visitor)?;
        self.min_rotation_speed.visit("MinRotationSpeed", visitor)?;
        self.max_rotation_speed.visit("MaxRotationSpeed", visitor)?;
        self.min_rotation.visit("MinRotation", visitor)?;
        self.max_rotation.visit("MaxRotation", visitor)?;
        self.alive_particles.visit("AliveParticles", visitor)?;
        self.time.visit("Time", visitor)?;

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
            min_lifetime: self.min_lifetime,
            max_lifetime: self.max_lifetime,
            min_size: self.min_size,
            max_size: self.max_size,
            min_size_modifier: self.min_size_modifier,
            max_size_modifier: self.max_size_modifier,
            min_x_velocity: self.min_x_velocity,
            max_x_velocity: self.max_x_velocity,
            min_y_velocity: self.min_y_velocity,
            max_y_velocity: self.max_y_velocity,
            min_z_velocity: self.min_z_velocity,
            max_z_velocity: self.max_z_velocity,
            min_rotation_speed: self.min_rotation_speed,
            max_rotation_speed: self.max_rotation_speed,
            min_rotation: self.min_rotation,
            max_rotation: self.max_rotation,
            alive_particles: self.alive_particles.clone(),
            time: self.time,
            particles_to_spawn: 0,
        }
    }
}

impl Default for Emitter {
    fn default() -> Self {
        Self {
            kind: EmitterKind::Unknown,
            position: Vec3::zero(),
            particle_spawn_rate: 0,
            max_particles: ParticleLimit::Unlimited,
            min_lifetime: 5.0,
            max_lifetime: 10.0,
            min_size: 0.125,
            max_size: 0.250,
            min_size_modifier: 0.0005,
            max_size_modifier: 0.0010,
            min_x_velocity: -0.001,
            max_x_velocity: 0.001,
            min_y_velocity: -0.001,
            max_y_velocity: 0.001,
            min_z_velocity: -0.001,
            max_z_velocity: 0.001,
            min_rotation_speed: -0.02,
            max_rotation_speed: 0.02,
            min_rotation: -std::f32::consts::PI,
            max_rotation: std::f32::consts::PI,
            alive_particles: Cell::new(0),
            time: 0.0,
            particles_to_spawn: 0,
        }
    }
}

pub struct ParticleSystem {
    particles: Vec<Particle>,
    /// Set of indices to alive particles sorted in back-to-front order.
    sorted_particles: RefCell<Vec<u32>>,
    free_particles: Vec<u32>,
    emitters: Vec<Emitter>,
    texture: Option<Rc<RefCell<Resource>>>,
    acceleration: Vec3,
    color_over_lifetime: Option<ColorGradient>,
}

impl ParticleSystem {
    pub fn new() -> Self {
        Self {
            particles: Vec::new(),
            sorted_particles: RefCell::new(Vec::new()),
            free_particles: Vec::new(),
            emitters: Vec::new(),
            texture: None,
            acceleration: Vec3::make(0.0, -9.81, 0.0),
            color_over_lifetime: None,
        }
    }

    pub fn add_emitter(&mut self, emitter: Emitter) {
        self.emitters.push(emitter)
    }

    pub fn set_acceleration(&mut self, accel: Vec3) {
        self.acceleration = accel;
    }

    pub fn set_color_over_lifetime_gradient(&mut self, gradient: ColorGradient) {
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
                emitter.alive_particles.set(emitter.alive_particles.get() + 1);
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
                        emitter.alive_particles.set(emitter.alive_particles.get() - 1);
                    }
                    particle.alive = false;
                    particle.lifetime = particle.initial_lifetime;
                } else {
                    particle.velocity += acceleration_offset;
                    particle.position += particle.velocity;
                    particle.size += particle.size_modifier;
                    particle.rotation += particle.rotation_speed;
                    if let Some(color_over_lifetime) = &self.color_over_lifetime {
                        let k = particle.lifetime / particle.initial_lifetime;
                        particle.color = color_over_lifetime.get_color(k);
                    } else {
                        particle.color = Color::white();
                    }
                }
            }
        }
    }

    pub fn generate_draw_data(&self, global_position: &Vec3, camera_pos: &Vec3, draw_data: &mut DrawData) {
        let mut sorted_particles = self.sorted_particles.borrow_mut();
        sorted_particles.clear();
        for (i, particle) in self.particles.iter().enumerate() {
            if particle.alive {
                let actual_position = particle.position + *global_position;
                particle.sqr_distance_to_camera.set(camera_pos.sqr_distance(&actual_position));
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
                tex_coord: Vec2::zero(),
                size: particle.size,
                rotation: particle.rotation,
                color: particle.color,
            });

            draw_data.vertices.push(Vertex {
                position: particle.position,
                tex_coord: Vec2::make(1.0, 0.0),
                size: particle.size,
                rotation: particle.rotation,
                color: particle.color,
            });

            draw_data.vertices.push(Vertex {
                position: particle.position,
                tex_coord: Vec2::make(1.0, 1.0),
                size: particle.size,
                rotation: particle.rotation,
                color: particle.color,
            });

            draw_data.vertices.push(Vertex {
                position: particle.position,
                tex_coord: Vec2::make(0.0, 1.0),
                size: particle.size,
                rotation: particle.rotation,
                color: particle.color,
            });

            let base_index = (i * 4) as u32;
            draw_data.indices.push(base_index);
            draw_data.indices.push(base_index + 1);
            draw_data.indices.push(base_index + 2);

            draw_data.indices.push(base_index);
            draw_data.indices.push(base_index + 2);
            draw_data.indices.push(base_index + 3);
        }
    }

    pub fn set_texture(&mut self, texture: Rc<RefCell<Resource>>) {
        self.texture = Some(texture)
    }

    pub fn get_texture(&self) -> Option<Rc<RefCell<Resource>>> {
        match &self.texture {
            Some(texture) => Some(texture.clone()),
            None => None
        }
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

        visitor.leave_region()
    }
}

impl Clone for ParticleSystem {
    fn clone(&self) -> Self {
        Self {
            color_over_lifetime: self.color_over_lifetime.clone(),
            particles: self.particles.clone(),
            sorted_particles: RefCell::new(Vec::new()), // Do not clone, since it is temporary array.
            free_particles: self.free_particles.clone(),
            emitters: self.emitters.clone(),
            texture: match &self.texture {
                Some(texture) => Some(texture.clone()),
                None => None
            },
            acceleration: self.acceleration,
        }
    }
}

impl Default for ParticleSystem {
    fn default() -> Self {
        Self::new()
    }
}