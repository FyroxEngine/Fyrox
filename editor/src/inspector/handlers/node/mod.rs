use std::any::TypeId;

use crate::{
    inspector::handlers::node::{
        camera::handle_camera_property_changed, collider::handle_collider_property_changed,
        collider2d::handle_collider2d_property_changed, joint::handle_joint_property_changed,
        joint2d::handle_joint2d_property_changed, light::*, mesh::handle_mesh_property_changed,
        particle_system::ParticleSystemHandler, rigid_body::handle_rigid_body_property_changed,
        rigid_body2d::handle_rigid_body2d_property_changed,
        terrain::handle_terrain_property_changed,
    },
    scene::commands::SetNodePropertyCommand,
    SceneCommand,
};
use fyrox::{
    core::pool::Handle,
    gui::{inspector::PropertyChanged, UserInterface},
    scene::{
        camera::Camera,
        collider::Collider,
        dim2,
        joint::Joint,
        light::{directional::DirectionalLight, point::PointLight, spot::SpotLight},
        mesh::Mesh,
        node::Node,
        particle_system::ParticleSystem,
        rigidbody::RigidBody,
        terrain::Terrain,
    },
};

pub mod base;
pub mod camera;
pub mod collider;
pub mod collider2d;
pub mod joint;
pub mod joint2d;
pub mod light;
pub mod mesh;
pub mod particle_system;
pub mod rigid_body;
pub mod rigid_body2d;
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
        } else if args.owner_type_id == TypeId::of::<DirectionalLight>() {
            handle_directional_light_property_changed(args, handle, node)
        } else if args.owner_type_id == TypeId::of::<PointLight>() {
            handle_point_light_property_changed(args, handle, node)
        } else if args.owner_type_id == TypeId::of::<SpotLight>() {
            handle_spot_light_property_changed(args, handle, node)
        } else if args.owner_type_id == TypeId::of::<ParticleSystem>() {
            self.particle_system_handler.handle(args, handle, node, ui)
        } else if args.owner_type_id == TypeId::of::<Terrain>() {
            handle_terrain_property_changed(args, handle, node)
        } else if args.owner_type_id == TypeId::of::<Mesh>() {
            handle_mesh_property_changed(args, handle, node)
        } else if args.owner_type_id == TypeId::of::<RigidBody>() {
            handle_rigid_body_property_changed(args, handle, node.as_rigid_body_mut())
        } else if args.owner_type_id == TypeId::of::<dim2::rigidbody::RigidBody>() {
            handle_rigid_body2d_property_changed(args, handle, node.as_rigid_body2d_mut())
        } else if args.owner_type_id == TypeId::of::<Collider>() {
            handle_collider_property_changed(args, handle, node.as_collider_mut())
        } else if args.owner_type_id == TypeId::of::<dim2::collider::Collider>() {
            handle_collider2d_property_changed(args, handle, node.as_collider2d_mut())
        } else if args.owner_type_id == TypeId::of::<Joint>() {
            handle_joint_property_changed(args, handle, node.as_joint_mut())
        } else if args.owner_type_id == TypeId::of::<dim2::joint::Joint>() {
            handle_joint2d_property_changed(args, handle, node.as_joint2d_mut())
        } else {
            Some(SceneCommand::new(
                SetNodePropertyCommand::from_property_changed(handle, args),
            ))
        }
    }
}
