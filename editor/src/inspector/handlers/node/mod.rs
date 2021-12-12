use std::any::TypeId;

use rg3d::{
    core::pool::Handle,
    gui::{inspector::PropertyChanged, UserInterface},
    scene::{
        base::Base,
        camera::Camera,
        decal::Decal,
        light::{directional::DirectionalLight, point::PointLight, spot::SpotLight},
        mesh::Mesh,
        node::Node,
        particle_system::ParticleSystem,
        sprite::Sprite,
        terrain::Terrain,
        Scene,
    },
};

use crate::inspector::handlers::node::{
    base::handle_base_property_changed, camera::handle_camera_property_changed,
    decal::handle_decal_property_changed, light::*, mesh::handle_mesh_property_changed,
    particle_system::ParticleSystemHandler, sprite::handle_sprite_property_changed,
    terrain::handle_terrain_property_changed,
};
use crate::SceneCommand;

pub mod base;
pub mod camera;
pub mod decal;
pub mod light;
pub mod mesh;
pub mod particle_system;
pub mod sprite;
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
        node: &Node,
        ui: &UserInterface,
        scene: &Scene,
    ) -> Option<SceneCommand> {
        if args.owner_type_id == TypeId::of::<Base>() {
            handle_base_property_changed(args, handle, node)
        } else if args.owner_type_id == TypeId::of::<Camera>() {
            handle_camera_property_changed(args, handle, node)
        } else if args.owner_type_id == TypeId::of::<Sprite>() {
            handle_sprite_property_changed(args, handle, node)
        } else if args.owner_type_id == TypeId::of::<DirectionalLight>() {
            handle_directional_light_property_changed(args, handle, node)
        } else if args.owner_type_id == TypeId::of::<PointLight>() {
            handle_point_light_property_changed(args, handle, node)
        } else if args.owner_type_id == TypeId::of::<SpotLight>() {
            handle_spot_light_property_changed(args, handle, node)
        } else if args.owner_type_id == TypeId::of::<ParticleSystem>() {
            self.particle_system_handler.handle(args, handle, node, ui)
        } else if args.owner_type_id == TypeId::of::<Decal>() {
            handle_decal_property_changed(args, handle, node)
        } else if args.owner_type_id == TypeId::of::<Terrain>() {
            handle_terrain_property_changed(args, handle, node, &scene.graph)
        } else if args.owner_type_id == TypeId::of::<Mesh>() {
            handle_mesh_property_changed(args, handle, node)
        } else {
            None
        }
    }
}
