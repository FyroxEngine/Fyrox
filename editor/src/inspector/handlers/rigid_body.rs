use crate::{make_command, physics::RigidBody, scene::commands::physics::*, SceneCommand};
use rg3d::{
    core::pool::Handle,
    gui::inspector::{CollectionChanged, FieldKind, PropertyChanged},
};

pub fn handle_rigid_body_property_changed(
    args: &PropertyChanged,
    handle: Handle<RigidBody>,
    rigid_body: &RigidBody,
) -> Option<SceneCommand> {
    match args.value {
        FieldKind::Object(ref value) => match args.name.as_ref() {
            RigidBody::MASS => {
                make_command!(SetBodyMassCommand, handle, value)
            }
            RigidBody::POSITION => {
                make_command!(SetBodyPositionCommand, handle, value)
            }
            RigidBody::ROTATION => {
                make_command!(SetBodyRotationCommand, handle, value)
            }
            RigidBody::LIN_VEL => {
                make_command!(SetBodyLinVelCommand, handle, value)
            }
            RigidBody::ANG_VEL => {
                make_command!(SetBodyAngVelCommand, handle, value)
            }
            RigidBody::STATUS => {
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
        FieldKind::Collection(ref collection_changed) => {
            if args.name == RigidBody::COLLIDERS {
                match **collection_changed {
                    CollectionChanged::Add => {
                        // TODO
                        None
                    }
                    CollectionChanged::Remove(index) => Some(SceneCommand::new(
                        DeleteColliderCommand::new(rigid_body.colliders[index].into()),
                    )),
                    CollectionChanged::ItemChanged { .. } => {
                        // TODO
                        None
                    }
                }
            } else {
                None
            }
        }
        _ => None,
    }
}
