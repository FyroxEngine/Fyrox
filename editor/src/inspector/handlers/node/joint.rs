use crate::{
    handle_properties, handle_property_changed,
    inspector::handlers::node::base::handle_base_property_changed, scene::commands::joint::*,
    SceneCommand,
};
use fyrox::{
    core::pool::Handle,
    gui::inspector::{FieldKind, PropertyChanged},
    scene::{joint::*, node::Node},
};
use std::any::TypeId;

pub fn handle_joint_property_changed(
    args: &PropertyChanged,
    handle: Handle<Node>,
    joint: &mut Joint,
) -> Option<SceneCommand> {
    match args.value {
        FieldKind::Object(ref value) => {
            handle_properties!(args.name.as_ref(), handle, value,
                Joint::BODY_1 => SetJointBody1Command,
                Joint::BODY_2 => SetJointBody2Command
            )
        }
        FieldKind::Inspectable(ref inner) => match args.name.as_ref() {
            Joint::PARAMS => {
                if inner.owner_type_id == TypeId::of::<BallJoint>() {
                    handle_ball_joint(inner, handle)
                } else if inner.owner_type_id == TypeId::of::<RevoluteJoint>() {
                    handle_revolute_joint(inner, handle)
                } else if inner.owner_type_id == TypeId::of::<FixedJoint>() {
                    handle_fixed_joint(inner, handle)
                } else if inner.owner_type_id == TypeId::of::<PrismaticJoint>() {
                    handle_prismatic_joint(inner, handle)
                } else {
                    None
                }
            }
            Joint::BASE => handle_base_property_changed(inner, handle, joint),
            _ => None,
        },
        _ => None,
    }
}

pub fn handle_ball_joint(args: &PropertyChanged, handle: Handle<Node>) -> Option<SceneCommand> {
    handle_property_changed!(args, handle,
        BallJoint::X_LIMITS_ANGLES => SetBallJointXLimitsCommand,
        BallJoint::X_LIMITS_ENABLED => SetBallJointXLimitsEnabledCommand,

        BallJoint::Y_LIMITS_ANGLES => SetBallJointYLimitsCommand,
        BallJoint::Y_LIMITS_ENABLED => SetBallJointYLimitsEnabledCommand,

        BallJoint::Z_LIMITS_ANGLES => SetBallJointZLimitsCommand,
        BallJoint::Z_LIMITS_ENABLED => SetBallJointZLimitsEnabledCommand
    )
}

pub fn handle_revolute_joint(args: &PropertyChanged, handle: Handle<Node>) -> Option<SceneCommand> {
    handle_property_changed!(args, handle,
        RevoluteJoint::LIMITS => SetRevoluteJointLimitsCommand,
        RevoluteJoint::LIMITS_ENABLED => SetRevoluteJointLimitsEnabledCommand
    )
}

pub fn handle_prismatic_joint(
    args: &PropertyChanged,
    handle: Handle<Node>,
) -> Option<SceneCommand> {
    handle_property_changed!(args, handle,
        PrismaticJoint::LIMITS => SetPrismaticJointLimitsCommand,
        PrismaticJoint::LIMITS_ENABLED => SetPrismaticJointLimitsEnabledCommand
    )
}

pub fn handle_fixed_joint(_args: &PropertyChanged, _handle: Handle<Node>) -> Option<SceneCommand> {
    // There are no properties.
    None
}
