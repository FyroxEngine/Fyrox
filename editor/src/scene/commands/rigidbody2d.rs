use crate::{command::Command, define_node_command, get_set_swap, scene::commands::SceneContext};
use rg3d::{
    core::{algebra::Vector2, pool::Handle},
    scene::{graph::Graph, node::Node, rigidbody::RigidBodyType},
};

define_node_command!(SetBodyMassCommand("Set 2D Body Mass", f32) where fn swap(self, node) {
    get_set_swap!(self, node.as_rigid_body2d_mut(), mass, set_mass)
});

define_node_command!(SetBodyLinVelCommand("Set 2D Body Linear Velocity", Vector2<f32>) where fn swap(self, node) {
    get_set_swap!(self, node.as_rigid_body2d_mut(), lin_vel, set_lin_vel)
});

define_node_command!(SetBodyAngVelCommand("Set 2D Body Angular Velocity", f32) where fn swap(self, node) {
    get_set_swap!(self, node.as_rigid_body2d_mut(), ang_vel, set_ang_vel)
});

define_node_command!(SetBodyStatusCommand("Set 2D Body Status", RigidBodyType) where fn swap(self, node) {
    get_set_swap!(self, node.as_rigid_body2d_mut(), body_type, set_body_type)
});

define_node_command!(SetBodyRotationLockedCommand("Set 2D Body Rotation Locked", bool) where fn swap(self, node) {
    get_set_swap!(self, node.as_rigid_body2d_mut(), is_rotation_locked, lock_rotations)
});

define_node_command!(SetBodyTranslationLockedCommand("Set 2D Body Translation Locked", bool) where fn swap(self, node) {
    get_set_swap!(self, node.as_rigid_body2d_mut(), is_translation_locked, lock_translation)
});

define_node_command!(SetBodyCanSleepCommand("Set 2D Body Can Sleep", bool) where fn swap(self, node) {
    get_set_swap!(self, node.as_rigid_body2d_mut(), is_can_sleep, set_can_sleep)
});

define_node_command!(SetBodyCcdEnabledCommand("Set 2D Body Ccd Enabled", bool) where fn swap(self, node) {
    get_set_swap!(self, node.as_rigid_body2d_mut(), is_ccd_enabled, enable_ccd)
});
