use std::any::TypeId;

use crate::{
    inspector::handlers::node::{
        base::handle_base_property_changed, camera::handle_camera_property_changed,
        collider::handle_collider_property_changed, collider2d::handle_collider2d_property_changed,
        decal::handle_decal_property_changed, joint::handle_joint_property_changed,
        joint2d::handle_joint2d_property_changed, light::*, mesh::handle_mesh_property_changed,
        particle_system::ParticleSystemHandler, rectangle::handle_rectangle_property_changed,
        rigid_body::handle_rigid_body_property_changed,
        rigid_body2d::handle_rigid_body2d_property_changed, sound::handle_sound_property_changed,
        sprite::handle_sprite_property_changed, terrain::handle_terrain_property_changed,
    },
    SceneCommand,
};
use fyrox::{
    core::pool::Handle,
    gui::{inspector::PropertyChanged, UserInterface},
    scene::{
        base::Base,
        camera::Camera,
        collider::Collider,
        decal::Decal,
        dim2,
        joint::Joint,
        light::{directional::DirectionalLight, point::PointLight, spot::SpotLight},
        mesh::Mesh,
        node::Node,
        particle_system::ParticleSystem,
        rigidbody::RigidBody,
        sound::Sound,
        sprite::Sprite,
        terrain::Terrain,
        Scene,
    },
};

pub mod base;
pub mod camera;
pub mod collider;
pub mod collider2d;
pub mod decal;
pub mod joint;
pub mod joint2d;
pub mod light;
pub mod mesh;
pub mod particle_system;
pub mod rectangle;
pub mod rigid_body;
pub mod rigid_body2d;
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
        } else if args.owner_type_id == TypeId::of::<RigidBody>() {
            handle_rigid_body_property_changed(args, handle, node.as_rigid_body())
        } else if args.owner_type_id == TypeId::of::<dim2::rigidbody::RigidBody>() {
            handle_rigid_body2d_property_changed(args, handle, node.as_rigid_body2d())
        } else if args.owner_type_id == TypeId::of::<Collider>() {
            handle_collider_property_changed(args, handle, node.as_collider())
        } else if args.owner_type_id == TypeId::of::<dim2::collider::Collider>() {
            handle_collider2d_property_changed(args, handle, node.as_collider2d())
        } else if args.owner_type_id == TypeId::of::<Joint>() {
            handle_joint_property_changed(args, handle, node.as_joint())
        } else if args.owner_type_id == TypeId::of::<dim2::joint::Joint>() {
            handle_joint2d_property_changed(args, handle, node.as_joint2d())
        } else if args.owner_type_id == TypeId::of::<dim2::rectangle::Rectangle>() {
            handle_rectangle_property_changed(args, handle, node)
        } else if args.owner_type_id == TypeId::of::<Sound>() {
            handle_sound_property_changed(args, handle)
        } else {
            None
        }
    }
}
