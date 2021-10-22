use crate::{command::Command, define_node_command, get_set_swap, scene::commands::SceneContext};
use rg3d::{
    core::{algebra::Vector3, pool::Handle},
    resource::texture::Texture,
    scene::{
        graph::Graph,
        node::Node,
        particle_system::{emitter::Emitter, ParticleLimit, ParticleSystem},
    },
};

#[derive(Debug)]
pub struct AddParticleSystemEmitterCommand {
    particle_system: Handle<Node>,
    emitter: Option<Emitter>,
}

impl AddParticleSystemEmitterCommand {
    pub fn new(particle_system: Handle<Node>, emitter: Emitter) -> Self {
        Self {
            particle_system,
            emitter: Some(emitter),
        }
    }
}

impl Command for AddParticleSystemEmitterCommand {
    fn name(&mut self, _context: &SceneContext) -> String {
        "Add Particle System Emitter".to_owned()
    }

    fn execute(&mut self, context: &mut SceneContext) {
        context.scene.graph[self.particle_system]
            .as_particle_system_mut()
            .emitters
            .push(self.emitter.take().unwrap());
    }

    fn revert(&mut self, context: &mut SceneContext) {
        self.emitter = Some(
            context.scene.graph[self.particle_system]
                .as_particle_system_mut()
                .emitters
                .pop()
                .unwrap(),
        );
    }
}

#[derive(Debug)]
pub struct DeleteEmitterCommand {
    particle_system: Handle<Node>,
    emitter: Option<Emitter>,
    emitter_index: usize,
}

impl DeleteEmitterCommand {
    pub fn new(particle_system: Handle<Node>, emitter_index: usize) -> Self {
        Self {
            particle_system,
            emitter: None,
            emitter_index,
        }
    }
}

impl Command for DeleteEmitterCommand {
    fn name(&mut self, _context: &SceneContext) -> String {
        "Delete Particle System Emitter".to_owned()
    }

    fn execute(&mut self, context: &mut SceneContext) {
        self.emitter = Some(
            context.scene.graph[self.particle_system]
                .as_particle_system_mut()
                .emitters
                .remove(self.emitter_index),
        );
    }

    fn revert(&mut self, context: &mut SceneContext) {
        let particle_system: &mut ParticleSystem =
            context.scene.graph[self.particle_system].as_particle_system_mut();
        if self.emitter_index == 0 {
            particle_system.emitters.push(self.emitter.take().unwrap());
        } else {
            particle_system
                .emitters
                .insert(self.emitter_index, self.emitter.take().unwrap());
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum EmitterNumericParameter {
    SpawnRate,
    MaxParticles,
    MinLifetime,
    MaxLifetime,
    MinSizeModifier,
    MaxSizeModifier,
    MinXVelocity,
    MaxXVelocity,
    MinYVelocity,
    MaxYVelocity,
    MinZVelocity,
    MaxZVelocity,
    MinRotationSpeed,
    MaxRotationSpeed,
    MinRotation,
    MaxRotation,
}

impl EmitterNumericParameter {
    fn name(self) -> &'static str {
        match self {
            EmitterNumericParameter::SpawnRate => "SpawnRate",
            EmitterNumericParameter::MaxParticles => "MaxParticles",
            EmitterNumericParameter::MinLifetime => "MinLifetime",
            EmitterNumericParameter::MaxLifetime => "MaxLifetime",
            EmitterNumericParameter::MinSizeModifier => "MinSizeModifier",
            EmitterNumericParameter::MaxSizeModifier => "MaxSizeModifier",
            EmitterNumericParameter::MinXVelocity => "MinXVelocity",
            EmitterNumericParameter::MaxXVelocity => "MaxXVelocity",
            EmitterNumericParameter::MinYVelocity => "MinYVelocity",
            EmitterNumericParameter::MaxYVelocity => "MaxYVelocity",
            EmitterNumericParameter::MinZVelocity => "MinZVelocity",
            EmitterNumericParameter::MaxZVelocity => "MaxZVelocity",
            EmitterNumericParameter::MinRotationSpeed => "MinRotationSpeed",
            EmitterNumericParameter::MaxRotationSpeed => "MaxRotationSpeed",
            EmitterNumericParameter::MinRotation => "MinRotation",
            EmitterNumericParameter::MaxRotation => "MaxRotation",
        }
    }
}

#[derive(Debug)]
pub struct SetEmitterNumericParameterCommand {
    node: Handle<Node>,
    parameter: EmitterNumericParameter,
    value: f32,
    emitter_index: usize,
}

impl SetEmitterNumericParameterCommand {
    pub fn new(
        node: Handle<Node>,
        emitter_index: usize,
        parameter: EmitterNumericParameter,
        value: f32,
    ) -> Self {
        Self {
            node,
            parameter,
            value,
            emitter_index,
        }
    }

    fn swap(&mut self, context: &mut SceneContext) {
        let emitter: &mut Emitter = &mut context.scene.graph[self.node]
            .as_particle_system_mut()
            .emitters[self.emitter_index];
        match self.parameter {
            EmitterNumericParameter::SpawnRate => {
                let old = emitter.spawn_rate();
                emitter.set_spawn_rate(self.value as u32);
                self.value = old as f32;
            }
            EmitterNumericParameter::MaxParticles => {
                let old = emitter.max_particles();
                emitter.set_max_particles(if self.value < 0.0 {
                    ParticleLimit::Unlimited
                } else {
                    ParticleLimit::Strict(self.value as u32)
                });
                self.value = match old {
                    ParticleLimit::Unlimited => -1.0,
                    ParticleLimit::Strict(value) => value as f32,
                };
            }
            EmitterNumericParameter::MinLifetime => {
                let old = emitter.life_time_range();
                emitter.set_life_time_range(self.value..old.end);
                self.value = old.start;
            }
            EmitterNumericParameter::MaxLifetime => {
                let old = emitter.life_time_range();
                emitter.set_life_time_range(old.start..self.value);
                self.value = old.end;
            }
            EmitterNumericParameter::MinSizeModifier => {
                let old = emitter.size_modifier_range();
                emitter.set_size_modifier_range(self.value..old.end);
                self.value = old.start;
            }
            EmitterNumericParameter::MaxSizeModifier => {
                let old = emitter.size_modifier_range();
                emitter.set_size_modifier_range(old.start..self.value);
                self.value = old.end;
            }
            EmitterNumericParameter::MinXVelocity => {
                let old = emitter.x_velocity_range();
                emitter.set_x_velocity_range(self.value..old.end);
                self.value = old.start;
            }
            EmitterNumericParameter::MaxXVelocity => {
                let old = emitter.x_velocity_range();
                emitter.set_x_velocity_range(old.start..self.value);
                self.value = old.end;
            }
            EmitterNumericParameter::MinYVelocity => {
                let old = emitter.y_velocity_range();
                emitter.set_y_velocity_range(self.value..old.end);
                self.value = old.start;
            }
            EmitterNumericParameter::MaxYVelocity => {
                let old = emitter.y_velocity_range();
                emitter.set_y_velocity_range(old.start..self.value);
                self.value = old.end;
            }
            EmitterNumericParameter::MinZVelocity => {
                let old = emitter.z_velocity_range();
                emitter.set_z_velocity_range(self.value..old.end);
                self.value = old.start;
            }
            EmitterNumericParameter::MaxZVelocity => {
                let old = emitter.z_velocity_range();
                emitter.set_z_velocity_range(old.start..self.value);
                self.value = old.end;
            }
            EmitterNumericParameter::MinRotationSpeed => {
                let old = emitter.rotation_speed_range();
                emitter.set_rotation_speed_range(self.value..old.end);
                self.value = old.start;
            }
            EmitterNumericParameter::MaxRotationSpeed => {
                let old = emitter.rotation_speed_range();
                emitter.set_rotation_speed_range(old.start..self.value);
                self.value = old.end;
            }
            EmitterNumericParameter::MinRotation => {
                let old = emitter.rotation_range();
                emitter.set_rotation_range(self.value..old.end);
                self.value = old.start;
            }
            EmitterNumericParameter::MaxRotation => {
                let old = emitter.rotation_range();
                emitter.set_rotation_range(old.start..self.value);
                self.value = old.end;
            }
        };
    }
}

impl Command for SetEmitterNumericParameterCommand {
    fn name(&mut self, _context: &SceneContext) -> String {
        format!("Set Emitter F32 Parameter: {}", self.parameter.name())
    }

    fn execute(&mut self, context: &mut SceneContext) {
        self.swap(context);
    }

    fn revert(&mut self, context: &mut SceneContext) {
        self.swap(context);
    }
}

macro_rules! define_emitter_command {
    ($name:ident($human_readable_name:expr, $value_type:ty) where fn swap($self:ident, $emitter:ident) $apply_method:block ) => {
        #[derive(Debug)]
        pub struct $name {
            handle: Handle<Node>,
            value: $value_type,
            index: usize
        }

        impl $name {
            pub fn new(handle: Handle<Node>, index: usize, value: $value_type) -> Self {
                Self { handle, index, value }
            }

            fn swap(&mut $self, graph: &mut Graph) {
                let $emitter = &mut graph[$self.handle].as_particle_system_mut().emitters[$self.index];
                $apply_method
            }
        }

        impl Command for $name {


            fn name(&mut self, _context: &SceneContext) -> String {
                $human_readable_name.to_owned()
            }

            fn execute(&mut self, context: &mut SceneContext) {
                self.swap(&mut context.scene.graph);
            }

            fn revert(&mut self, context: &mut SceneContext) {
                self.swap(&mut context.scene.graph);
            }
        }
    };
}

define_node_command!(SetParticleSystemTextureCommand("Set Particle System Texture", Option<Texture>) where fn swap(self, node) {
    get_set_swap!(self, node.as_particle_system_mut(), texture, set_texture);
});

define_node_command!(SetParticleSystemAccelerationCommand("Set Particle System Acceleration", Vector3<f32>) where fn swap(self, node) {
    get_set_swap!(self, node.as_particle_system_mut(), acceleration, set_acceleration);
});

macro_rules! define_emitter_variant_command {
    ($name:ident($human_readable_name:expr, $value_type:ty) where fn swap($self:ident, $emitter:ident, $variant:ident, $var:ident) $apply_method:block ) => {
        define_emitter_command!($name($human_readable_name, $value_type) where fn swap($self, $emitter) {
            if let Emitter::$variant($var) = $emitter {
                $apply_method
            } else {
                unreachable!()
            }
        });
    };
}

define_emitter_variant_command!(SetSphereEmitterRadiusCommand("Set Sphere Emitter Radius", f32) where fn swap(self, emitter, Sphere, sphere) {
    get_set_swap!(self, sphere, radius, set_radius);
});

define_emitter_variant_command!(SetCylinderEmitterRadiusCommand("Set Cylinder Emitter Radius", f32) where fn swap(self, emitter, Cylinder, cylinder) {
    get_set_swap!(self, cylinder, radius, set_radius);
});

define_emitter_variant_command!(SetCylinderEmitterHeightCommand("Set Cylinder Emitter Radius", f32) where fn swap(self, emitter, Cylinder, cylinder) {
    get_set_swap!(self, cylinder, height, set_height);
});

define_emitter_variant_command!(SetBoxEmitterHalfWidthCommand("Set Box Emitter Half Width", f32) where fn swap(self, emitter, Cuboid, box_emitter) {
    get_set_swap!(self, box_emitter, half_width, set_half_width);
});

define_emitter_variant_command!(SetBoxEmitterHalfHeightCommand("Set Box Emitter Half Height", f32) where fn swap(self, emitter, Cuboid, box_emitter) {
    get_set_swap!(self, box_emitter, half_height, set_half_height);
});

define_emitter_variant_command!(SetBoxEmitterHalfDepthCommand("Set Box Emitter Half Depth", f32) where fn swap(self, emitter, Cuboid, box_emitter) {
    get_set_swap!(self, box_emitter, half_depth, set_half_depth);
});

define_emitter_command!(SetEmitterPositionCommand("Set Emitter Position", Vector3<f32>) where fn swap(self, emitter) {
    get_set_swap!(self, emitter, position, set_position);
});

define_emitter_command!(SetEmitterResurrectParticlesCommand("Set Emitter Resurrect Particles", bool) where fn swap(self, emitter) {
    get_set_swap!(self, emitter, is_particles_resurrects, enable_particle_resurrection);
});
