use crate::{do_command, inspector::SenderHelper, physics::RigidBody, scene::commands::physics::*};
use rg3d::{
    core::pool::Handle,
    gui::{message::FieldKind, message::PropertyChanged},
};

pub fn handle_rigid_body_property_changed(
    args: &PropertyChanged,
    handle: Handle<RigidBody>,
    helper: &SenderHelper,
) -> Option<()> {
    match args.value {
        FieldKind::Object(ref value) => match args.name.as_ref() {
            RigidBody::MASS => {
                do_command!(helper, SetBodyMassCommand, handle, value)
            }
            RigidBody::POSITION => {
                do_command!(helper, SetBodyPositionCommand, handle, value)
            }
            RigidBody::ROTATION => {
                do_command!(helper, SetBodyRotationCommand, handle, value)
            }
            RigidBody::LIN_VEL => {
                do_command!(helper, SetBodyLinVelCommand, handle, value)
            }
            RigidBody::ANG_VEL => {
                do_command!(helper, SetBodyAngVelCommand, handle, value)
            }
            RigidBody::STATUS => {
                do_command!(helper, SetBodyStatusCommand, handle, value)
            }
            RigidBody::X_ROTATION_LOCKED => {
                do_command!(helper, SetBodyXRotationLockedCommand, handle, value)
            }
            RigidBody::Y_ROTATION_LOCKED => {
                do_command!(helper, SetBodyYRotationLockedCommand, handle, value)
            }
            RigidBody::Z_ROTATION_LOCKED => {
                do_command!(helper, SetBodyZRotationLockedCommand, handle, value)
            }
            RigidBody::TRANSLATION_LOCKED => {
                do_command!(helper, SetBodyTranslationLockedCommand, handle, value)
            }
            _ => None,
        },
        _ => None,
    }
}
