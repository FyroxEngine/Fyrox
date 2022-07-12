use crate::{command::Command, define_swap_command, scene::commands::SceneContext};
use fyrox::{
    core::pool::Handle,
    scene::{graph::Graph, joint::*, node::Node},
};
use std::ops::Range;

macro_rules! define_joint_variant_command {
    ($($ty_name:ident($value_ty:ty): $variant:ident, $field:ident, $name:expr;)*) => {
        $(
            define_swap_command! {
                $ty_name($value_ty): $name, |me: &mut $ty_name, graph: &mut Graph| {
                    let node = &mut graph[me.handle];
                    let variant = match *node.as_joint_mut().params_mut() {
                        JointParams::$variant(ref mut x) => x,
                        _ => unreachable!()
                    };
                    ::core::mem::swap(&mut variant.$field, &mut me.value);
                }
            }
        )*
    };
}

define_joint_variant_command! {
    SetBallJointXLimitsCommand(Range<f32>): BallJoint, x_limits_angles, "Set Ball Joint X Limits";
    SetBallJointXLimitsEnabledCommand(bool): BallJoint, x_limits_enabled, "Set Ball Joint X Limits Enabled";
    SetBallJointYLimitsCommand(Range<f32>): BallJoint, y_limits_angles, "Set Ball Joint Y Limits";
    SetBallJointYLimitsEnabledCommand(bool): BallJoint, y_limits_enabled, "Set Ball Joint Y Limits Enabled";
    SetBallJointZLimitsCommand(Range<f32>): BallJoint, z_limits_angles, "Set Ball Joint Z Limits";
    SetBallJointZLimitsEnabledCommand(bool): BallJoint, z_limits_enabled, "Set Ball Joint Z Limits Enabled";

    SetRevoluteJointLimitsCommand(Range<f32>): RevoluteJoint, limits, "Set Revolute Joint Limits";
    SetRevoluteJointLimitsEnabledCommand(bool): RevoluteJoint, limits_enabled, "Set Revolute Joint Limits Enabled";

    SetPrismaticJointLimitsCommand(Range<f32>): PrismaticJoint, limits, "Set Prismatic Joint Limits";
    SetPrismaticJointLimitsEnabledCommand(bool): PrismaticJoint, limits_enabled, "Set Prismatic Joint Limits Enabled";
}

define_swap_command! {
    Node::as_joint_mut,
    SetJointBody1Command(Handle<Node>): body1, set_body1, "Set Joint Body 1";
    SetJointBody2Command(Handle<Node>): body2, set_body2, "Set Joint Body 2";
}
