use crate::{make_command, scene::commands::physics::*, SceneCommand};
use rg3d::{
    core::pool::Handle,
    gui::inspector::{FieldKind, PropertyChanged},
    scene::node::Node,
    scene::rigidbody::RigidBody,
};

pub fn handle_rigid_body_property_changed(
    args: &PropertyChanged,
    handle: Handle<Node>,
) -> Option<SceneCommand> {
    match args.value {
        FieldKind::Object(ref value) => match args.name.as_ref() {
            RigidBody::MASS => {
                make_command!(SetBodyMassCommand, handle, value)
            }
            RigidBody::LIN_VEL => {
                make_command!(SetBodyLinVelCommand, handle, value)
            }
            RigidBody::ANG_VEL => {
                make_command!(SetBodyAngVelCommand, handle, value)
            }
            RigidBody::BODY_TYPE => {
                make_command!(SetBodyStatusCommand, handle, value)
            }
            RigidBody::X_ROTATION_LOCKED => {
                make_command!(SetBodyXRotationLockedCommand, handle, value)
            }
            RigidBody::Y_ROTATION_LOCKED => {
                make_command!(SetBodyYRotationLockedCommand, handle, value)
            }
            RigidBody::Z_ROTATION_LOCKED => {
                make_command!(SetBodyZRotationLockedCommand, handle, value)
            }
            RigidBody::TRANSLATION_LOCKED => {
                make_command!(SetBodyTranslationLockedCommand, handle, value)
            }
            _ => None,
        },
        _ => None,
    }
}
