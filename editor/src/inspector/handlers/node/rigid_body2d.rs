use crate::{
    handle_properties, inspector::handlers::node::base::handle_base_property_changed,
    scene::commands::rigidbody2d::*, SceneCommand,
};
use fyrox::{
    core::pool::Handle,
    gui::inspector::{FieldKind, PropertyChanged},
    scene::{dim2::rigidbody::RigidBody, node::Node},
};

pub fn handle_rigid_body2d_property_changed(
    args: &PropertyChanged,
    handle: Handle<Node>,
    rigid_body: &mut RigidBody,
) -> Option<SceneCommand> {
    match args.value {
        FieldKind::Object(ref value) => {
            handle_properties!(args.name.as_ref(), handle, value,
                RigidBody::MASS => SetBodyMassCommand,
                RigidBody::LIN_VEL => SetBodyLinVelCommand,
                RigidBody::ANG_VEL => SetBodyAngVelCommand,
                RigidBody::BODY_TYPE => SetBodyStatusCommand,
                RigidBody::ROTATION_LOCKED => SetBodyRotationLockedCommand,
                RigidBody::TRANSLATION_LOCKED => SetBodyTranslationLockedCommand,
                RigidBody::CAN_SLEEP => SetBodyCanSleepCommand,
                RigidBody::CCD_ENABLED => SetBodyCcdEnabledCommand,
                RigidBody::LIN_DAMPING => SetBodyLinDampingCommand,
                RigidBody::ANG_DAMPING => SetBodyAngDampingCommand
            )
        }
        FieldKind::Inspectable(ref inner) => match args.name.as_ref() {
            RigidBody::BASE => handle_base_property_changed(inner, handle, rigid_body),
            _ => None,
        },
        _ => None,
    }
}
