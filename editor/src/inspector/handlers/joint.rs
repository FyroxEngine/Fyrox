use crate::{
    inspector::handlers::node::base::handle_base_property_changed, make_command,
    scene::commands::joint::*, SceneCommand,
};
use rg3d::{
    core::pool::Handle,
    gui::inspector::{FieldKind, PropertyChanged},
    scene::{joint::*, node::Node},
};
use std::any::TypeId;

pub fn handle_joint_property_changed(
    args: &PropertyChanged,
    handle: Handle<Node>,
    joint: &Joint,
) -> Option<SceneCommand> {
    match args.value {
        FieldKind::Object(ref value) => match args.name.as_ref() {
            Joint::BODY_1 => {
                make_command!(SetJointBody1Command, handle, value)
            }
            Joint::BODY_2 => {
                make_command!(SetJointBody2Command, handle, value)
            }
            _ => None,
        },
        FieldKind::Inspectable(ref inner) => match args.name.as_ref() {
            Joint::PARAMS => {
                let params = joint.params();
                if inner.owner_type_id == TypeId::of::<BallJointDesc>() {
                    handle_ball_joint_property_changed(inner, handle, params)
                } else if inner.owner_type_id == TypeId::of::<RevoluteJointDesc>() {
                    handle_revolute_joint_property_changed(inner, handle, params)
                } else if inner.owner_type_id == TypeId::of::<FixedJointDesc>() {
                    handle_fixed_joint_property_changed(inner, handle, params)
                } else if inner.owner_type_id == TypeId::of::<PrismaticJointDesc>() {
                    handle_prismatic_joint_property_changed(inner, handle, params)
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

pub fn handle_ball_joint_property_changed(
    args: &PropertyChanged,
    handle: Handle<Node>,
    params: &JointParamsDesc,
) -> Option<SceneCommand> {
    if let JointParamsDesc::BallJoint(_) = params {
        match args.value {
            FieldKind::Object(ref value) => match args.name.as_ref() {
                BallJointDesc::LOCAL_ANCHOR_1 => Some(SceneCommand::new(
                    SetBallJointAnchor1Command::new(handle, value.cast_value().cloned()?),
                )),
                BallJointDesc::LOCAL_ANCHOR_2 => Some(SceneCommand::new(
                    SetBallJointAnchor2Command::new(handle, value.cast_value().cloned()?),
                )),
                _ => None,
            },
            _ => None,
        }
    } else {
        None
    }
}

pub fn handle_revolute_joint_property_changed(
    args: &PropertyChanged,
    handle: Handle<Node>,
    params: &JointParamsDesc,
) -> Option<SceneCommand> {
    if let JointParamsDesc::RevoluteJoint(_) = params {
        match args.value {
            FieldKind::Object(ref value) => match args.name.as_ref() {
                RevoluteJointDesc::LOCAL_ANCHOR_1 => Some(SceneCommand::new(
                    SetRevoluteJointAnchor1Command::new(handle, value.cast_value().cloned()?),
                )),
                RevoluteJointDesc::LOCAL_ANCHOR_2 => Some(SceneCommand::new(
                    SetRevoluteJointAnchor2Command::new(handle, value.cast_value().cloned()?),
                )),
                RevoluteJointDesc::LOCAL_AXIS_1 => Some(SceneCommand::new(
                    SetRevoluteJointAxis1Command::new(handle, value.cast_value().cloned()?),
                )),
                RevoluteJointDesc::LOCAL_AXIS_2 => Some(SceneCommand::new(
                    SetRevoluteJointAxis2Command::new(handle, value.cast_value().cloned()?),
                )),
                _ => None,
            },
            _ => None,
        }
    } else {
        None
    }
}

pub fn handle_prismatic_joint_property_changed(
    args: &PropertyChanged,
    handle: Handle<Node>,
    params: &JointParamsDesc,
) -> Option<SceneCommand> {
    if let JointParamsDesc::PrismaticJoint(_) = params {
        match args.value {
            FieldKind::Object(ref value) => match args.name.as_ref() {
                PrismaticJointDesc::LOCAL_ANCHOR_1 => Some(SceneCommand::new(
                    SetPrismaticJointAnchor1Command::new(handle, value.cast_value().cloned()?),
                )),
                PrismaticJointDesc::LOCAL_ANCHOR_2 => Some(SceneCommand::new(
                    SetPrismaticJointAnchor2Command::new(handle, value.cast_value().cloned()?),
                )),
                PrismaticJointDesc::LOCAL_AXIS_1 => Some(SceneCommand::new(
                    SetPrismaticJointAxis1Command::new(handle, value.cast_value().cloned()?),
                )),
                PrismaticJointDesc::LOCAL_AXIS_2 => Some(SceneCommand::new(
                    SetPrismaticJointAxis2Command::new(handle, value.cast_value().cloned()?),
                )),
                _ => None,
            },
            _ => None,
        }
    } else {
        None
    }
}

pub fn handle_fixed_joint_property_changed(
    args: &PropertyChanged,
    handle: Handle<Node>,
    params: &JointParamsDesc,
) -> Option<SceneCommand> {
    if let JointParamsDesc::FixedJoint(_) = params {
        match args.value {
            FieldKind::Object(ref value) => match args.name.as_ref() {
                FixedJointDesc::LOCAL_ANCHOR_1_TRANSLATION => Some(SceneCommand::new(
                    SetFixedJointAnchor1TranslationCommand::new(
                        handle,
                        value.cast_value().cloned()?,
                    ),
                )),
                FixedJointDesc::LOCAL_ANCHOR_2_TRANSLATION => Some(SceneCommand::new(
                    SetFixedJointAnchor2TranslationCommand::new(
                        handle,
                        value.cast_value().cloned()?,
                    ),
                )),
                FixedJointDesc::LOCAL_ANCHOR_1_ROTATION => Some(SceneCommand::new(
                    SetFixedJointAnchor1RotationCommand::new(handle, value.cast_value().cloned()?),
                )),
                FixedJointDesc::LOCAL_ANCHOR_2_ROTATION => Some(SceneCommand::new(
                    SetFixedJointAnchor2RotationCommand::new(handle, value.cast_value().cloned()?),
                )),
                _ => None,
            },
            _ => None,
        }
    } else {
        None
    }
}
