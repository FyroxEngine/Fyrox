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
use std::ops::Range;

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

define_node_command!(SetAccelerationCommand("Set Particle System Acceleration", Vector3<f32>) where fn swap(self, node) {
    get_set_swap!(self, node.as_particle_system_mut(), acceleration, set_acceleration);
});

define_node_command!(SetParticleSystemEnabledCommand("Set Particle System Enabled", bool) where fn swap(self, node) {
    get_set_swap!(self, node.as_particle_system_mut(), is_enabled, set_enabled);
});

define_node_command!(SetSoftBoundarySharpnessFactorCommand("Set Soft Boundary Sharpness Factor", f32) where fn swap(self, node) {
    get_set_swap!(self, node.as_particle_system_mut(), soft_boundary_sharpness_factor, set_soft_boundary_sharpness_factor);
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

define_emitter_command!(SetEmitterSpawnRateCommand("Set Emitter Spawn Rate", u32) where fn swap(self, emitter) {
    get_set_swap!(self, emitter, spawn_rate, set_spawn_rate);
});

define_emitter_command!(SetEmitterParticleLimitCommand("Set Emitter Particle Limit", ParticleLimit) where fn swap(self, emitter) {
    get_set_swap!(self, emitter, max_particles, set_max_particles);
});

define_emitter_command!(SetEmitterLifetimeRangeCommand("Set Emitter Lifetime Range", Range<f32>) where fn swap(self, emitter) {
    get_set_swap!(self, emitter, life_time_range, set_life_time_range);
});

define_emitter_command!(SetEmitterSizeRangeCommand("Set Emitter Size Range", Range<f32>) where fn swap(self, emitter) {
    get_set_swap!(self, emitter, size_range, set_size_range);
});

define_emitter_command!(SetEmitterSizeModifierRangeCommand("Set Emitter Size Modifier Range", Range<f32>) where fn swap(self, emitter) {
    get_set_swap!(self, emitter, size_modifier_range, set_size_modifier_range);
});

define_emitter_command!(SetEmitterXVelocityRangeCommand("Set Emitter X Velocity Range", Range<f32>) where fn swap(self, emitter) {
    get_set_swap!(self, emitter, x_velocity_range, set_x_velocity_range);
});

define_emitter_command!(SetEmitterYVelocityRangeCommand("Set Emitter Y Velocity Range", Range<f32>) where fn swap(self, emitter) {
    get_set_swap!(self, emitter, y_velocity_range, set_y_velocity_range);
});

define_emitter_command!(SetEmitterZVelocityRangeCommand("Set Emitter Z Velocity Range", Range<f32>) where fn swap(self, emitter) {
    get_set_swap!(self, emitter, z_velocity_range, set_z_velocity_range);
});

define_emitter_command!(SetEmitterRotationSpeedRangeCommand("Set Emitter Rotation Speed Range", Range<f32>) where fn swap(self, emitter) {
    get_set_swap!(self, emitter, rotation_speed_range, set_rotation_speed_range);
});

define_emitter_command!(SetEmitterRotationRangeCommand("Set Emitter Rotation Range", Range<f32>) where fn swap(self, emitter) {
    get_set_swap!(self, emitter, rotation_range, set_rotation_range);
});

define_emitter_command!(SetEmitterResurrectParticlesCommand("Set Emitter Resurrect Particles", bool) where fn swap(self, emitter) {
    get_set_swap!(self, emitter, is_particles_resurrects, enable_particle_resurrection);
});
