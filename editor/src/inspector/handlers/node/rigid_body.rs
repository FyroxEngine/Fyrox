use crate::{
    handle_properties, inspector::handlers::node::base::handle_base_property_changed,
    scene::commands::rigidbody::*, SceneCommand,
};
use rg3d::{
    core::pool::Handle,
    gui::inspector::{FieldKind, PropertyChanged},
    scene::{node::Node, rigidbody::RigidBody},
};

pub fn handle_rigid_body_property_changed(
    args: &PropertyChanged,
    handle: Handle<Node>,
    rigid_body: &RigidBody,
) -> Option<SceneCommand> {
    match args.value {
        FieldKind::Object(ref value) => {
            handle_properties!(args.name.as_ref(), handle, value,
                RigidBody::MASS => SetBodyMassCommand,
                RigidBody::LIN_VEL => SetBodyLinVelCommand,
                RigidBody::ANG_VEL => SetBodyAngVelCommand,
                RigidBody::BODY_TYPE => SetBodyStatusCommand,
                RigidBody::X_ROTATION_LOCKED => SetBodyXRotationLockedCommand,
                RigidBody::Y_ROTATION_LOCKED => SetBodyYRotationLockedCommand,
                RigidBody::Z_ROTATION_LOCKED => SetBodyZRotationLockedCommand,
                RigidBody::TRANSLATION_LOCKED => SetBodyTranslationLockedCommand,
                RigidBody::CAN_SLEEP => SetBodyCanSleepCommand,
                RigidBody::CCD_ENABLED => SetBodyCcdEnabledCommand
            )
        }
        FieldKind::Inspectable(ref inner) => match args.name.as_ref() {
            RigidBody::BASE => handle_base_property_changed(inner, handle, rigid_body),
            _ => None,
        },
        _ => None,
    }
}
