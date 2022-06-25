use crate::{command::Command, define_swap_command, scene::commands::SceneContext};
use fyrox::{
    core::algebra::Vector2,
    scene::{node::Node, rigidbody::RigidBodyType},
};

define_swap_command! {
    Node::as_rigid_body2d_mut,
    SetBodyMassCommand(f32): mass, set_mass, "Set 2D Body Mass";
    SetBodyLinVelCommand(Vector2<f32>): lin_vel, set_lin_vel, "Set 2D Body Linear Velocity";
    SetBodyAngVelCommand(f32): ang_vel, set_ang_vel, "Set 2D Body Angular Velocity";
    SetBodyStatusCommand(RigidBodyType): body_type, set_body_type, "Set 2D Body Status";
    SetBodyRotationLockedCommand(bool): is_rotation_locked, lock_rotations, "Set 2D Body Rotation Locked";
    SetBodyTranslationLockedCommand(bool): is_translation_locked, lock_translation, "Set 2D Body Translation Locked";
    SetBodyCanSleepCommand(bool): is_can_sleep, set_can_sleep, "Set 2D Body Can Sleep";
    SetBodyCcdEnabledCommand(bool): is_ccd_enabled, enable_ccd, "Set 2D Body Ccd Enabled";
    SetBodyLinDampingCommand(f32): lin_damping, set_lin_damping, "Set Lin Damping";
    SetBodyAngDampingCommand(f32): ang_damping, set_ang_damping, "Set Ang Damping";
}
