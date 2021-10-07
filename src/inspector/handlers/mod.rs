use crate::inspector::{
    handlers::{
        base::handle_base_property_changed, camera::handle_camera_property_changed, light::*,
        particle_system::ParticleSystemHandler, sprite::handle_sprite_property_changed,
        terrain::handle_terrain_property_changed, transform::handle_transform_property_changed,
    },
    SenderHelper,
};
use rg3d::{
    core::pool::Handle,
    gui::{message::PropertyChanged, UserInterface},
    scene::{
        base::Base,
        camera::Camera,
        decal::Decal,
        light::{point::PointLight, spot::SpotLight, BaseLight},
        node::Node,
        particle_system::ParticleSystem,
        sprite::Sprite,
        terrain::Terrain,
        transform::Transform,
        Scene,
    },
};
use std::any::TypeId;

pub mod base;
pub mod camera;
pub mod light;
pub mod particle_system;
pub mod sound;
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
        node_handle: Handle<Node>,
        node: &Node,
        helper: &SenderHelper,
        ui: &UserInterface,
        scene: &Scene,
    ) -> Option<()> {
        if args.owner_type_id == TypeId::of::<Base>() {
            handle_base_property_changed(args, node_handle, &helper)
        } else if args.owner_type_id == TypeId::of::<Transform>() {
            handle_transform_property_changed(args, node_handle, node, &helper)
        } else if args.owner_type_id == TypeId::of::<Camera>() {
            handle_camera_property_changed(args, node_handle, node, &helper)
        } else if args.owner_type_id == TypeId::of::<Sprite>() {
            handle_sprite_property_changed(args, node_handle, &helper)
        } else if args.owner_type_id == TypeId::of::<BaseLight>() {
            handle_base_light_property_changed(args, node_handle, helper)
        } else if args.owner_type_id == TypeId::of::<PointLight>() {
            handle_point_light_property_changed(args, node_handle, helper)
        } else if args.owner_type_id == TypeId::of::<SpotLight>() {
            handle_spot_light_property_changed(args, node_handle, helper)
        } else if args.owner_type_id == TypeId::of::<ParticleSystem>() {
            self.particle_system_handler
                .handle(args, node_handle, &helper, ui)
        } else if args.owner_type_id == TypeId::of::<Decal>() {
            Some(()) // TODO
        } else if args.owner_type_id == TypeId::of::<Terrain>() {
            handle_terrain_property_changed(args, node_handle, &helper, &scene.graph)
        } else {
            Some(())
        }
    }
}
