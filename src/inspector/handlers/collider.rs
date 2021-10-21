use crate::{do_command, inspector::SenderHelper, physics::Collider, scene::commands::physics::*};
use rg3d::physics3d::desc::InteractionGroupsDesc;
use rg3d::{
    core::pool::Handle,
    gui::{message::FieldKind, message::PropertyChanged},
};

pub fn handle_collider_property_changed(
    args: &PropertyChanged,
    handle: Handle<Collider>,
    helper: &SenderHelper,
) -> Option<()> {
    match args.value {
        FieldKind::Object(ref value) => match args.name.as_ref() {
            Collider::FRICTION => {
                do_command!(helper, SetColliderFrictionCommand, handle, value)
            }
            Collider::RESTITUTION => {
                do_command!(helper, SetColliderRestitutionCommand, handle, value)
            }
            Collider::IS_SENSOR => {
                do_command!(helper, SetColliderIsSensorCommand, handle, value)
            }
            Collider::DENSITY => {
                do_command!(helper, SetColliderDensityCommand, handle, value)
            }
            Collider::TRANSLATION => {
                do_command!(helper, SetColliderPositionCommand, handle, value)
            }
            Collider::ROTATION => {
                do_command!(helper, SetColliderRotationCommand, handle, value)
            }
            _ => None,
        },
        FieldKind::Inspectable(ref inner_property) => match args.name.as_ref() {
            Collider::COLLISION_GROUPS => match inner_property.value {
                FieldKind::Object(ref value) => match inner_property.name.as_ref() {
                    InteractionGroupsDesc::MEMBERSHIPS => {
                        do_command!(
                            helper,
                            SetColliderCollisionGroupsMembershipsCommand,
                            handle,
                            value
                        )
                    }
                    InteractionGroupsDesc::FILTER => {
                        do_command!(
                            helper,
                            SetColliderCollisionGroupsFilterCommand,
                            handle,
                            value
                        )
                    }
                    _ => None,
                },
                _ => None,
            },
            Collider::SOLVER_GROUPS => match inner_property.value {
                FieldKind::Object(ref value) => match inner_property.name.as_ref() {
                    InteractionGroupsDesc::MEMBERSHIPS => {
                        do_command!(
                            helper,
                            SetColliderSolverGroupsMembershipsCommand,
                            handle,
                            value
                        )
                    }
                    InteractionGroupsDesc::FILTER => {
                        do_command!(helper, SetColliderSolverGroupsFilterCommand, handle, value)
                    }
                    _ => None,
                },
                _ => None,
            },
            _ => None,
        },
        _ => None,
    }
}
