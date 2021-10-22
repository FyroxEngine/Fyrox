use crate::{do_command, inspector::SenderHelper, physics::Collider, scene::commands::physics::*};
use rg3d::{
    core::pool::Handle,
    gui::{message::FieldKind, message::PropertyChanged},
    physics3d::desc::*,
};
use std::any::TypeId;

pub fn handle_collider_property_changed(
    args: &PropertyChanged,
    handle: Handle<Collider>,
    collider: &Collider,
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
            Collider::SHAPE => {
                if inner_property.owner_type_id == TypeId::of::<CuboidDesc>() {
                    handle_cuboid_desc_property_changed(handle, collider, inner_property, helper)
                } else if inner_property.owner_type_id == TypeId::of::<BallDesc>() {
                    handle_ball_desc_property_changed(handle, collider, inner_property, helper)
                } else if inner_property.owner_type_id == TypeId::of::<CylinderDesc>() {
                    handle_cylinder_desc_property_changed(handle, collider, inner_property, helper)
                } else if inner_property.owner_type_id == TypeId::of::<RoundCylinderDesc>() {
                    handle_round_cylinder_desc_property_changed(
                        handle,
                        collider,
                        inner_property,
                        helper,
                    )
                } else if inner_property.owner_type_id == TypeId::of::<ConeDesc>() {
                    handle_cone_desc_property_changed(handle, collider, inner_property, helper)
                } else if inner_property.owner_type_id == TypeId::of::<CapsuleDesc>() {
                    handle_capsule_desc_property_changed(handle, collider, inner_property, helper)
                } else if inner_property.owner_type_id == TypeId::of::<SegmentDesc>() {
                    handle_segment_desc_property_changed(handle, collider, inner_property, helper)
                } else if inner_property.owner_type_id == TypeId::of::<TriangleDesc>() {
                    handle_triangle_desc_property_changed(handle, collider, inner_property, helper)
                } else {
                    None
                }
            }
            _ => None,
        },
        _ => None,
    }
}

fn handle_ball_desc_property_changed(
    handle: Handle<Collider>,
    collider: &Collider,
    property_changed: &PropertyChanged,
    helper: &SenderHelper,
) -> Option<()> {
    if let ColliderShapeDesc::Ball(_) = collider.shape {
        match property_changed.value {
            FieldKind::Object(ref value) => match property_changed.name.as_ref() {
                BallDesc::RADIUS => helper.do_scene_command(SetBallRadiusCommand::new(
                    handle,
                    value.cast_value().cloned()?,
                )),
                _ => None,
            },
            _ => None,
        }
    } else {
        None
    }
}

fn handle_cuboid_desc_property_changed(
    handle: Handle<Collider>,
    collider: &Collider,
    property_changed: &PropertyChanged,
    helper: &SenderHelper,
) -> Option<()> {
    if let ColliderShapeDesc::Cuboid(_) = collider.shape {
        match property_changed.value {
            FieldKind::Object(ref value) => match property_changed.name.as_ref() {
                CuboidDesc::HALF_EXTENTS => helper.do_scene_command(
                    SetCuboidHalfExtentsCommand::new(handle, value.cast_value().cloned()?),
                ),
                _ => None,
            },
            _ => None,
        }
    } else {
        None
    }
}

fn handle_cylinder_desc_property_changed(
    handle: Handle<Collider>,
    collider: &Collider,
    property_changed: &PropertyChanged,
    helper: &SenderHelper,
) -> Option<()> {
    if let ColliderShapeDesc::Cylinder(_) = collider.shape {
        match property_changed.value {
            FieldKind::Object(ref value) => match property_changed.name.as_ref() {
                CylinderDesc::HALF_HEIGHT => helper.do_scene_command(
                    SetCylinderHalfHeightCommand::new(handle, value.cast_value().cloned()?),
                ),
                CylinderDesc::RADIUS => helper.do_scene_command(SetCylinderRadiusCommand::new(
                    handle,
                    value.cast_value().cloned()?,
                )),
                _ => None,
            },
            _ => None,
        }
    } else {
        None
    }
}

fn handle_round_cylinder_desc_property_changed(
    handle: Handle<Collider>,
    collider: &Collider,
    property_changed: &PropertyChanged,
    helper: &SenderHelper,
) -> Option<()> {
    if let ColliderShapeDesc::RoundCylinder(_) = collider.shape {
        match property_changed.value {
            FieldKind::Object(ref value) => match property_changed.name.as_ref() {
                RoundCylinderDesc::HALF_HEIGHT => helper.do_scene_command(
                    SetRoundCylinderHalfHeightCommand::new(handle, value.cast_value().cloned()?),
                ),
                RoundCylinderDesc::RADIUS => helper.do_scene_command(
                    SetRoundCylinderRadiusCommand::new(handle, value.cast_value().cloned()?),
                ),
                RoundCylinderDesc::BORDER_RADIUS => helper.do_scene_command(
                    SetRoundCylinderBorderRadiusCommand::new(handle, value.cast_value().cloned()?),
                ),
                _ => None,
            },
            _ => None,
        }
    } else {
        None
    }
}

fn handle_cone_desc_property_changed(
    handle: Handle<Collider>,
    collider: &Collider,
    property_changed: &PropertyChanged,
    helper: &SenderHelper,
) -> Option<()> {
    if let ColliderShapeDesc::Cone(_) = collider.shape {
        match property_changed.value {
            FieldKind::Object(ref value) => match property_changed.name.as_ref() {
                ConeDesc::HALF_HEIGHT => helper.do_scene_command(SetConeHalfHeightCommand::new(
                    handle,
                    value.cast_value().cloned()?,
                )),
                ConeDesc::RADIUS => helper.do_scene_command(SetConeRadiusCommand::new(
                    handle,
                    value.cast_value().cloned()?,
                )),
                _ => None,
            },
            _ => None,
        }
    } else {
        None
    }
}

fn handle_capsule_desc_property_changed(
    handle: Handle<Collider>,
    collider: &Collider,
    property_changed: &PropertyChanged,
    helper: &SenderHelper,
) -> Option<()> {
    if let ColliderShapeDesc::Capsule(_) = collider.shape {
        match property_changed.value {
            FieldKind::Object(ref value) => match property_changed.name.as_ref() {
                CapsuleDesc::BEGIN => helper.do_scene_command(SetCapsuleBeginCommand::new(
                    handle,
                    value.cast_value().cloned()?,
                )),
                CapsuleDesc::END => helper.do_scene_command(SetCapsuleEndCommand::new(
                    handle,
                    value.cast_value().cloned()?,
                )),
                CapsuleDesc::RADIUS => helper.do_scene_command(SetCapsuleRadiusCommand::new(
                    handle,
                    value.cast_value().cloned()?,
                )),
                _ => None,
            },
            _ => None,
        }
    } else {
        None
    }
}

fn handle_segment_desc_property_changed(
    handle: Handle<Collider>,
    collider: &Collider,
    property_changed: &PropertyChanged,
    helper: &SenderHelper,
) -> Option<()> {
    if let ColliderShapeDesc::Segment(_) = collider.shape {
        match property_changed.value {
            FieldKind::Object(ref value) => match property_changed.name.as_ref() {
                SegmentDesc::BEGIN => helper.do_scene_command(SetSegmentBeginCommand::new(
                    handle,
                    value.cast_value().cloned()?,
                )),
                SegmentDesc::END => helper.do_scene_command(SetSegmentEndCommand::new(
                    handle,
                    value.cast_value().cloned()?,
                )),
                _ => None,
            },
            _ => None,
        }
    } else {
        None
    }
}

fn handle_triangle_desc_property_changed(
    handle: Handle<Collider>,
    collider: &Collider,
    property_changed: &PropertyChanged,
    helper: &SenderHelper,
) -> Option<()> {
    if let ColliderShapeDesc::Triangle(_) = collider.shape {
        match property_changed.value {
            FieldKind::Object(ref value) => match property_changed.name.as_ref() {
                TriangleDesc::A => helper.do_scene_command(SetTriangleACommand::new(
                    handle,
                    value.cast_value().cloned()?,
                )),
                TriangleDesc::B => helper.do_scene_command(SetTriangleBCommand::new(
                    handle,
                    value.cast_value().cloned()?,
                )),
                TriangleDesc::C => helper.do_scene_command(SetTriangleCCommand::new(
                    handle,
                    value.cast_value().cloned()?,
                )),
                _ => None,
            },
            _ => None,
        }
    } else {
        None
    }
}
