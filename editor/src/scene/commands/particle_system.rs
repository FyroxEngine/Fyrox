use crate::{command::Command, define_swap_command, scene::commands::SceneContext};
use fyrox::{
    core::{algebra::Vector3, pool::Handle},
    resource::texture::Texture,
    scene::{
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
            .get_mut()
            .push(self.emitter.take().unwrap());
    }

    fn revert(&mut self, context: &mut SceneContext) {
        self.emitter = Some(
            context.scene.graph[self.particle_system]
                .as_particle_system_mut()
                .emitters
                .get_mut()
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
                .get_mut()
                .remove(self.emitter_index),
        );
    }

    fn revert(&mut self, context: &mut SceneContext) {
        let particle_system: &mut ParticleSystem =
            context.scene.graph[self.particle_system].as_particle_system_mut();
        if self.emitter_index == 0 {
            particle_system
                .emitters
                .get_mut()
                .push(self.emitter.take().unwrap());
        } else {
            particle_system
                .emitters
                .get_mut()
                .insert(self.emitter_index, self.emitter.take().unwrap());
        }
    }
}

macro_rules! define_emitter_command {
    // core impl
    ($(#[$meta:meta])* $type:ident($value_type:ty): $name:expr, $swap:expr) => {
        $(#[$meta])*
        #[derive(Debug)]
        pub struct $type {
            handle: $crate::fyrox::core::pool::Handle<Node>,
            index: usize,
            value: $value_type,
        }

        impl $type {
            pub fn new(handle: $crate::fyrox::core::pool::Handle<Node>, index: usize, value: $value_type) -> Self {
                Self { handle, index, value }
            }

            fn swap(&mut self, graph: &mut $crate::fyrox::scene::graph::Graph) {
                let emitter = &mut graph[self.handle].as_particle_system_mut().emitters.get_mut()[self.index];
                #[allow(clippy::redundant_closure_call)]
                ($swap)(self, emitter)
            }
        }

        impl Command for $type {
            fn name(&mut self, _context: &SceneContext) -> String {
                $name.to_owned()
            }

            fn execute(&mut self, context: &mut SceneContext) {
                self.swap(&mut context.scene.graph);
            }

            fn revert(&mut self, context: &mut SceneContext) {
                self.swap(&mut context.scene.graph);
            }
        }
    };

    // cast `&mut Emitter` and use their setter/getter methods for swapping them
    ($(
        $(#[$meta:meta])* $type:ident($value_type:ty): $get:ident, $set:ident, $name:expr;
     )*) => {
        $(
            define_emitter_command! {
                $(#[$meta:meta])*
                $type($value_type):
                $name, |me: &mut $type, emitter: &mut Emitter| {
                    let old = emitter.$get();
                    let _ = emitter.$set(me.value.clone());
                    me.value = old;
                }
            }
        )+
    };
}

macro_rules! define_emitter_variant_command {
    ($( $(#[$meta:meta])* $ty_name:ident($value_type:ty): $variant:ident, $get:ident, $set:ident, $name_expr:expr;)*) => {
        $(
            define_emitter_command! {
                $(#[$meta:meta])*
                $ty_name($value_type): $name_expr, |me: &mut Self, emitter: &mut Emitter| {
                    let variant = match emitter {
                        Emitter::$variant(x) => x,
                        _ => unreachable!(),
                    };
                    let old = variant.$get();
                    let _ = variant.$set(me.value.clone());
                    me.value = old;
                }
            }
        )*
    };
}

define_swap_command! {
    Node::as_particle_system_mut,
    SetParticleSystemTextureCommand(Option<Texture>): texture, set_texture, "Set Particle System Texture";
    SetAccelerationCommand(Vector3<f32>): acceleration, set_acceleration, "Set Particle System Acceleration";
    SetParticleSystemEnabledCommand(bool): is_enabled, set_enabled, "Set Particle System Enabled";
    SetSoftBoundarySharpnessFactorCommand(f32): soft_boundary_sharpness_factor, set_soft_boundary_sharpness_factor, "Set Soft Boundary Sharpness Factor";
}

define_emitter_variant_command! {
    SetSphereEmitterRadiusCommand(f32): Sphere, radius, set_radius, "Set Sphere Emitter Radius";
    SetCylinderEmitterRadiusCommand(f32): Cylinder, radius, set_radius, "Set Cylinder Emitter Radius";
    SetCylinderEmitterHeightCommand(f32): Cylinder, height, set_height, "Set Cylinder Emitter Radius";
    SetBoxEmitterHalfWidthCommand(f32): Cuboid, half_width, set_half_width, "Set Box Emitter Half Width";
    SetBoxEmitterHalfHeightCommand(f32): Cuboid, half_height, set_half_height, "Set Box Emitter Half Height";
    SetBoxEmitterHalfDepthCommand(f32): Cuboid, half_depth, set_half_depth, "Set Box Emitter Half Depth";
}

define_emitter_command! {
    SetEmitterPositionCommand(Vector3<f32>): position, set_position, "Set Emitter Position";
    SetEmitterSpawnRateCommand(u32): spawn_rate, set_spawn_rate, "Set Emitter Spawn Rate";
    SetEmitterParticleLimitCommand(ParticleLimit): max_particles, set_max_particles, "Set Emitter Particle Limit";
    SetEmitterLifetimeRangeCommand(Range<f32>): life_time_range, set_life_time_range, "Set Emitter Lifetime Range";
    SetEmitterSizeRangeCommand(Range<f32>): size_range, set_size_range, "Set Emitter Size Range";
    SetEmitterSizeModifierRangeCommand(Range<f32>): size_modifier_range, set_size_modifier_range, "Set Emitter Size Modifier Range";
    SetEmitterXVelocityRangeCommand(Range<f32>): x_velocity_range, set_x_velocity_range, "Set Emitter X Velocity Range";
    SetEmitterYVelocityRangeCommand(Range<f32>): y_velocity_range, set_y_velocity_range, "Set Emitter Y Velocity Range";
    SetEmitterZVelocityRangeCommand(Range<f32>): z_velocity_range, set_z_velocity_range, "Set Emitter Z Velocity Range";
    SetEmitterRotationSpeedRangeCommand(Range<f32>): rotation_speed_range, set_rotation_speed_range,
    "Set Emitter Rotation Speed Range";
    SetEmitterRotationRangeCommand(Range<f32>): rotation_range, set_rotation_range, "Set Emitter Rotation Range";
    SetEmitterResurrectParticlesCommand(bool): is_particles_resurrects, enable_particle_resurrection, "Set Emitter Resurrect Particles";
}
