use crate::{command::Command, define_node_command, get_set_swap, scene::commands::SceneContext};
use rg3d::{
    core::{algebra::Vector3, pool::Handle},
    scene::{graph::Graph, node::Node, rigidbody::*},
};

define_node_command!(SetBodyMassCommand("Set Body Mass", f32) where fn swap(self, node) {
    get_set_swap!(self, node.as_rigid_body_mut(), mass, set_mass)
});

define_node_command!(SetBodyLinVelCommand("Set Body Linear Velocity", Vector3<f32>) where fn swap(self, node) {
    get_set_swap!(self, node.as_rigid_body_mut(), lin_vel, set_lin_vel)
});

define_node_command!(SetBodyAngVelCommand("Set Body Angular Velocity", Vector3<f32>) where fn swap(self, node) {
    get_set_swap!(self, node.as_rigid_body_mut(), ang_vel, set_ang_vel)
});

define_node_command!(SetBodyStatusCommand("Set Body Status", RigidBodyType) where fn swap(self, node) {
    get_set_swap!(self, node.as_rigid_body_mut(), body_type, set_body_type)
});

define_node_command!(SetBodyXRotationLockedCommand("Set Body X Rotation Locked", bool) where fn swap(self, node) {
    get_set_swap!(self, node.as_rigid_body_mut(), is_x_rotation_locked, lock_x_rotations)
});

define_node_command!(SetBodyYRotationLockedCommand("Set Body Y Rotation Locked", bool) where fn swap(self, node) {
    get_set_swap!(self, node.as_rigid_body_mut(), is_y_rotation_locked, lock_y_rotations)
});

define_node_command!(SetBodyZRotationLockedCommand("Set Body Z Rotation Locked", bool) where fn swap(self, node) {
    get_set_swap!(self, node.as_rigid_body_mut(), is_z_rotation_locked, lock_z_rotations)
});

define_node_command!(SetBodyTranslationLockedCommand("Set Body Translation Locked", bool) where fn swap(self, node) {
    get_set_swap!(self, node.as_rigid_body_mut(), is_translation_locked, lock_translation)
});

define_node_command!(SetBodyCanSleepCommand("Set Body Can Sleep", bool) where fn swap(self, node) {
    get_set_swap!(self, node.as_rigid_body_mut(), is_can_sleep, set_can_sleep)
});

define_node_command!(SetBodyCcdEnabledCommand("Set Body Ccd Enabled", bool) where fn swap(self, node) {
    get_set_swap!(self, node.as_rigid_body_mut(), is_ccd_enabled, enable_ccd)
});
