use crate::{command::Command, define_swap_command, scene::commands::SceneContext};
use fyrox::{
    core::algebra::Vector3,
    scene::{node::Node, rigidbody::*},
};

define_swap_command! {
    Node::as_rigid_body_mut,
    SetBodyMassCommand(f32): mass, set_mass, "Set Body Mass";
    SetBodyLinVelCommand(Vector3<f32>): lin_vel, set_lin_vel, "Set Body Linear Velocity";
    SetBodyAngVelCommand(Vector3<f32>): ang_vel, set_ang_vel, "Set Body Angular Velocity";
    SetBodyStatusCommand(RigidBodyType): body_type, set_body_type, "Set Body Status";
    SetBodyXRotationLockedCommand(bool): is_x_rotation_locked, lock_x_rotations, "Set Body X Rotation Locked";
    SetBodyYRotationLockedCommand(bool): is_y_rotation_locked, lock_y_rotations, "Set Body Y Rotation Locked";
    SetBodyZRotationLockedCommand(bool): is_z_rotation_locked, lock_z_rotations, "Set Body Z Rotation Locked";
    SetBodyTranslationLockedCommand(bool): is_translation_locked, lock_translation, "Set Body Translation Locked";
    SetBodyCanSleepCommand(bool): is_can_sleep, set_can_sleep, "Set Body Can Sleep";
    SetBodyCcdEnabledCommand(bool): is_ccd_enabled, enable_ccd, "Set Body Ccd Enabled";
}
