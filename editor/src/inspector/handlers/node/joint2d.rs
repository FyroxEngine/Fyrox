use crate::{
    handle_properties, handle_property_changed,
    inspector::handlers::node::base::handle_base_property_changed, scene::commands::joint2d::*,
    SceneCommand,
};
use fyrox::{
    core::pool::Handle,
    gui::inspector::{FieldKind, PropertyChanged},
    scene::{dim2::joint::*, node::Node},
};
use std::any::TypeId;

pub fn handle_joint2d_property_changed(
    args: &PropertyChanged,
    handle: Handle<Node>,
    joint: &Joint,
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
        BallJoint::LOCAL_ANCHOR_1 => SetBallJointAnchor1Command,
        BallJoint::LOCAL_ANCHOR_2 => SetBallJointAnchor2Command
    )
}

pub fn handle_prismatic_joint(
    args: &PropertyChanged,
    handle: Handle<Node>,
) -> Option<SceneCommand> {
    handle_property_changed!(args, handle,
        PrismaticJoint::LOCAL_ANCHOR_1 => SetPrismaticJointAnchor1Command,
        PrismaticJoint::LOCAL_ANCHOR_2 => SetPrismaticJointAnchor2Command,
        PrismaticJoint::LOCAL_AXIS_1 => SetPrismaticJointAxis1Command,
        PrismaticJoint::LOCAL_AXIS_2 => SetPrismaticJointAxis2Command
    )
}

pub fn handle_fixed_joint(args: &PropertyChanged, handle: Handle<Node>) -> Option<SceneCommand> {
    handle_property_changed!(args, handle,
         FixedJoint::LOCAL_ANCHOR_1_TRANSLATION => SetFixedJointAnchor1TranslationCommand,
         FixedJoint::LOCAL_ANCHOR_2_TRANSLATION => SetFixedJointAnchor2TranslationCommand,
         FixedJoint::LOCAL_ANCHOR_1_ROTATION => SetFixedJointAnchor1RotationCommand,
         FixedJoint::LOCAL_ANCHOR_2_ROTATION => SetFixedJointAnchor2RotationCommand
    )
}
