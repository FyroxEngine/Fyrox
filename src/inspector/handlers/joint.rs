use crate::{do_command, inspector::SenderHelper, physics::Joint, scene::commands::physics::*};
use rg3d::{
    core::pool::Handle,
    gui::{message::FieldKind, message::PropertyChanged},
    physics3d::desc::{
        BallJointDesc, FixedJointDesc, JointParamsDesc, PrismaticJointDesc, RevoluteJointDesc,
    },
};
use std::any::TypeId;

pub fn handle_joint_property_changed(
    args: &PropertyChanged,
    handle: Handle<Joint>,
    joint: &Joint,
    helper: &SenderHelper,
) -> Option<()> {
    match args.value {
        FieldKind::Object(ref value) => match args.name.as_ref() {
            Joint::BODY_1 => {
                do_command!(helper, SetJointBody1Command, handle, value)
            }
            Joint::BODY_2 => {
                do_command!(helper, SetJointBody2Command, handle, value)
            }
            _ => None,
        },
        FieldKind::Inspectable(ref inner) => {
            if let Joint::PARAMS = args.name.as_ref() {
                let params = &joint.params;
                if inner.owner_type_id == TypeId::of::<BallJointDesc>() {
                    handle_ball_joint_property_changed(inner, handle, params, helper)
                } else if inner.owner_type_id == TypeId::of::<RevoluteJointDesc>() {
                    handle_revolute_joint_property_changed(inner, handle, params, helper)
                } else if inner.owner_type_id == TypeId::of::<FixedJointDesc>() {
                    handle_fixed_joint_property_changed(inner, handle, params, helper)
                } else if inner.owner_type_id == TypeId::of::<PrismaticJointDesc>() {
                    handle_prismatic_joint_property_changed(inner, handle, params, helper)
                } else {
                    None
                }
            } else {
                None
            }
        }
        _ => None,
    }
}

pub fn handle_ball_joint_property_changed(
    args: &PropertyChanged,
    handle: Handle<Joint>,
    params: &JointParamsDesc,
    helper: &SenderHelper,
) -> Option<()> {
    if let JointParamsDesc::BallJoint(_) = params {
        match args.value {
            FieldKind::Object(ref value) => match args.name.as_ref() {
                BallJointDesc::LOCAL_ANCHOR_1 => helper.do_scene_command(
                    SetBallJointAnchor1Command::new(handle, value.cast_value().cloned()?),
                ),
                BallJointDesc::LOCAL_ANCHOR_2 => helper.do_scene_command(
                    SetBallJointAnchor2Command::new(handle, value.cast_value().cloned()?),
                ),
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
    handle: Handle<Joint>,
    params: &JointParamsDesc,
    helper: &SenderHelper,
) -> Option<()> {
    if let JointParamsDesc::RevoluteJoint(_) = params {
        match args.value {
            FieldKind::Object(ref value) => match args.name.as_ref() {
                RevoluteJointDesc::LOCAL_ANCHOR_1 => helper.do_scene_command(
                    SetRevoluteJointAnchor1Command::new(handle, value.cast_value().cloned()?),
                ),
                RevoluteJointDesc::LOCAL_ANCHOR_2 => helper.do_scene_command(
                    SetRevoluteJointAnchor2Command::new(handle, value.cast_value().cloned()?),
                ),
                RevoluteJointDesc::LOCAL_AXIS_1 => helper.do_scene_command(
                    SetRevoluteJointAxis1Command::new(handle, value.cast_value().cloned()?),
                ),
                RevoluteJointDesc::LOCAL_AXIS_2 => helper.do_scene_command(
                    SetRevoluteJointAxis2Command::new(handle, value.cast_value().cloned()?),
                ),
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
    handle: Handle<Joint>,
    params: &JointParamsDesc,
    helper: &SenderHelper,
) -> Option<()> {
    if let JointParamsDesc::PrismaticJoint(_) = params {
        match args.value {
            FieldKind::Object(ref value) => match args.name.as_ref() {
                PrismaticJointDesc::LOCAL_ANCHOR_1 => helper.do_scene_command(
                    SetPrismaticJointAnchor1Command::new(handle, value.cast_value().cloned()?),
                ),
                PrismaticJointDesc::LOCAL_ANCHOR_2 => helper.do_scene_command(
                    SetPrismaticJointAnchor2Command::new(handle, value.cast_value().cloned()?),
                ),
                PrismaticJointDesc::LOCAL_AXIS_1 => helper.do_scene_command(
                    SetPrismaticJointAxis1Command::new(handle, value.cast_value().cloned()?),
                ),
                PrismaticJointDesc::LOCAL_AXIS_2 => helper.do_scene_command(
                    SetPrismaticJointAxis2Command::new(handle, value.cast_value().cloned()?),
                ),
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
    handle: Handle<Joint>,
    params: &JointParamsDesc,
    helper: &SenderHelper,
) -> Option<()> {
    if let JointParamsDesc::FixedJoint(_) = params {
        match args.value {
            FieldKind::Object(ref value) => match args.name.as_ref() {
                FixedJointDesc::LOCAL_ANCHOR_1_TRANSLATION => {
                    helper.do_scene_command(SetFixedJointAnchor1TranslationCommand::new(
                        handle,
                        value.cast_value().cloned()?,
                    ))
                }
                FixedJointDesc::LOCAL_ANCHOR_2_TRANSLATION => {
                    helper.do_scene_command(SetFixedJointAnchor2TranslationCommand::new(
                        handle,
                        value.cast_value().cloned()?,
                    ))
                }
                FixedJointDesc::LOCAL_ANCHOR_1_ROTATION => helper.do_scene_command(
                    SetFixedJointAnchor1RotationCommand::new(handle, value.cast_value().cloned()?),
                ),
                FixedJointDesc::LOCAL_ANCHOR_2_ROTATION => helper.do_scene_command(
                    SetFixedJointAnchor2RotationCommand::new(handle, value.cast_value().cloned()?),
                ),
                _ => None,
            },
            _ => None,
        }
    } else {
        None
    }
}
