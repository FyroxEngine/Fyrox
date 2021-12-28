use crate::{
    handle_properties, inspector::handlers::node::base::handle_base_property_changed,
    scene::commands::rigidbody2d::*, SceneCommand,
};
use rg3d::{
    core::pool::Handle,
    gui::inspector::{FieldKind, PropertyChanged},
    scene::{dim2::rigidbody::RigidBody, node::Node},
};

pub fn handle_rigid_body2d_property_changed(
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
                RigidBody::ROTATION_LOCKED => SetBodyRotationLockedCommand,
                RigidBody::TRANSLATION_LOCKED => SetBodyTranslationLockedCommand
            )
        }
        FieldKind::Inspectable(ref inner) => match args.name.as_ref() {
            RigidBody::BASE => handle_base_property_changed(inner, handle, rigid_body),
            _ => None,
        },
        _ => None,
    }
}
