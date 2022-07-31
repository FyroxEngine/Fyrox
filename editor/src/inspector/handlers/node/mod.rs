use std::any::TypeId;

use crate::scene::commands::make_set_node_property_command;
use crate::{
    inspector::handlers::node::{
        camera::handle_camera_property_changed, collider::handle_collider_property_changed,
        collider2d::handle_collider2d_property_changed, particle_system::ParticleSystemHandler,
        terrain::handle_terrain_property_changed,
    },
    SceneCommand,
};
use fyrox::{
    core::pool::Handle,
    gui::{inspector::PropertyChanged, UserInterface},
    scene::{
        camera::Camera, collider::Collider, dim2, node::Node, particle_system::ParticleSystem,
        terrain::Terrain,
    },
};

pub mod base;
pub mod camera;
pub mod collider;
pub mod collider2d;
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
        if args.owner_type_id == TypeId::of::<Camera>() {
            handle_camera_property_changed(args, handle, node)
        } else if args.owner_type_id == TypeId::of::<ParticleSystem>() {
            self.particle_system_handler.handle(args, handle, node, ui)
        } else if args.owner_type_id == TypeId::of::<Terrain>() {
            handle_terrain_property_changed(args, handle, node)
        } else if args.owner_type_id == TypeId::of::<Collider>() {
            handle_collider_property_changed(args, handle, node.as_collider_mut())
        } else if args.owner_type_id == TypeId::of::<dim2::collider::Collider>() {
            handle_collider2d_property_changed(args, handle, node.as_collider2d_mut())
        } else {
            Some(make_set_node_property_command(handle, args))
        }
    }
}
