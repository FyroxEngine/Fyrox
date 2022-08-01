use std::any::TypeId;

use crate::{
    inspector::handlers::node::{
        particle_system::ParticleSystemHandler, terrain::handle_terrain_property_changed,
    },
    scene::commands::make_set_node_property_command,
    SceneCommand,
};
use fyrox::{
    core::pool::Handle,
    gui::{inspector::PropertyChanged, UserInterface},
    scene::{node::Node, particle_system::ParticleSystem, terrain::Terrain},
};

pub mod base;
pub mod particle_system;
pub mod terrain;
pub mod transform;

pub struct SceneNodePropertyChangedHandler {
    pub particle_system_handler: ParticleSystemHandler,
}

impl SceneNodePropertyChangedHandler {
    pub fn handle(
        &self,
        args: &PropertyChanged,
        handle: Handle<Node>,
        node: &mut Node,
        ui: &UserInterface,
    ) -> Option<SceneCommand> {
        if args.owner_type_id == TypeId::of::<ParticleSystem>() {
            self.particle_system_handler.handle(args, handle, node, ui)
        } else if args.owner_type_id == TypeId::of::<Terrain>() {
            handle_terrain_property_changed(args, handle, node)
        } else {
            Some(make_set_node_property_command(handle, args))
        }
    }
}
