use crate::{make_command, physics::Collider, scene::commands::physics::*, SceneCommand};
use rg3d::{
    core::pool::Handle,
    gui::inspector::{FieldKind, PropertyChanged},
    physics3d::desc::*,
};
use std::any::TypeId;

pub fn handle_collider_property_changed(
    args: &PropertyChanged,
    handle: Handle<Collider>,
    collider: &Collider,
) -> Option<SceneCommand> {
    match args.value {
        FieldKind::Object(ref value) => match args.name.as_ref() {
            Collider::FRICTION => {
                make_command!(SetColliderFrictionCommand, handle, value)
            }
            Collider::RESTITUTION => {
                make_command!(SetColliderRestitutionCommand, handle, value)
            }
            Collider::IS_SENSOR => {
                make_command!(SetColliderIsSensorCommand, handle, value)
            }
            Collider::DENSITY => {
                make_command!(SetColliderDensityCommand, handle, value)
            }
            Collider::TRANSLATION => {
                make_command!(SetColliderPositionCommand, handle, value)
            }
            Collider::ROTATION => {
                make_command!(SetColliderRotationCommand, handle, value)
            }
            _ => None,
        },
        FieldKind::Inspectable(ref inner_property) => match args.name.as_ref() {
            Collider::COLLISION_GROUPS => match inner_property.value {
                FieldKind::Object(ref value) => match inner_property.name.as_ref() {
                    InteractionGroupsDesc::MEMBERSHIPS => {
                        make_command!(SetColliderCollisionGroupsMembershipsCommand, handle, value)
                    }
                    InteractionGroupsDesc::FILTER => {
                        make_command!(SetColliderCollisionGroupsFilterCommand, handle, value)
                    }
                    _ => None,
                },
                _ => None,
            },
            Collider::SOLVER_GROUPS => match inner_property.value {
                FieldKind::Object(ref value) => match inner_property.name.as_ref() {
                    InteractionGroupsDesc::MEMBERSHIPS => {
                        make_command!(SetColliderSolverGroupsMembershipsCommand, handle, value)
                    }
                    InteractionGroupsDesc::FILTER => {
                        make_command!(SetColliderSolverGroupsFilterCommand, handle, value)
                    }
                    _ => None,
                },
                _ => None,
            },
            Collider::SHAPE => {
                if inner_property.owner_type_id == TypeId::of::<CuboidDesc>() {
                    handle_cuboid_desc_property_changed(handle, collider, inner_property)
                } else if inner_property.owner_type_id == TypeId::of::<BallDesc>() {
                    handle_ball_desc_property_changed(handle, collider, inner_property)
                } else if inner_property.owner_type_id == TypeId::of::<CylinderDesc>() {
                    handle_cylinder_desc_property_changed(handle, collider, inner_property)
                } else if inner_property.owner_type_id == TypeId::of::<RoundCylinderDesc>() {
                    handle_round_cylinder_desc_property_changed(handle, collider, inner_property)
                } else if inner_property.owner_type_id == TypeId::of::<ConeDesc>() {
                    handle_cone_desc_property_changed(handle, collider, inner_property)
                } else if inner_property.owner_type_id == TypeId::of::<CapsuleDesc>() {
                    handle_capsule_desc_property_changed(handle, collider, inner_property)
                } else if inner_property.owner_type_id == TypeId::of::<SegmentDesc>() {
                    handle_segment_desc_property_changed(handle, collider, inner_property)
                } else if inner_property.owner_type_id == TypeId::of::<TriangleDesc>() {
                    handle_triangle_desc_property_changed(handle, collider, inner_property)
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
) -> Option<SceneCommand> {
    if let ColliderShapeDesc::Ball(_) = collider.shape {
        match property_changed.value {
            FieldKind::Object(ref value) => match property_changed.name.as_ref() {
                BallDesc::RADIUS => make_command!(SetBallRadiusCommand, handle, value),
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
) -> Option<SceneCommand> {
    if let ColliderShapeDesc::Cuboid(_) = collider.shape {
        match property_changed.value {
            FieldKind::Object(ref value) => match property_changed.name.as_ref() {
                CuboidDesc::HALF_EXTENTS => {
                    make_command!(SetCuboidHalfExtentsCommand, handle, value)
                }
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
) -> Option<SceneCommand> {
    if let ColliderShapeDesc::Cylinder(_) = collider.shape {
        match property_changed.value {
            FieldKind::Object(ref value) => match property_changed.name.as_ref() {
                CylinderDesc::HALF_HEIGHT => {
                    make_command!(SetCylinderHalfHeightCommand, handle, value)
                }
                CylinderDesc::RADIUS => {
                    make_command!(SetCylinderRadiusCommand, handle, value)
                }
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
) -> Option<SceneCommand> {
    if let ColliderShapeDesc::RoundCylinder(_) = collider.shape {
        match property_changed.value {
            FieldKind::Object(ref value) => match property_changed.name.as_ref() {
                RoundCylinderDesc::HALF_HEIGHT => {
                    make_command!(SetRoundCylinderHalfHeightCommand, handle, value)
                }
                RoundCylinderDesc::RADIUS => {
                    make_command!(SetRoundCylinderRadiusCommand, handle, value)
                }
                RoundCylinderDesc::BORDER_RADIUS => {
                    make_command!(SetRoundCylinderBorderRadiusCommand, handle, value)
                }
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
) -> Option<SceneCommand> {
    if let ColliderShapeDesc::Cone(_) = collider.shape {
        match property_changed.value {
            FieldKind::Object(ref value) => match property_changed.name.as_ref() {
                ConeDesc::HALF_HEIGHT => {
                    make_command!(SetConeHalfHeightCommand, handle, value)
                }
                ConeDesc::RADIUS => make_command!(SetConeRadiusCommand, handle, value),
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
) -> Option<SceneCommand> {
    if let ColliderShapeDesc::Capsule(_) = collider.shape {
        match property_changed.value {
            FieldKind::Object(ref value) => match property_changed.name.as_ref() {
                CapsuleDesc::BEGIN => make_command!(SetCapsuleBeginCommand, handle, value),
                CapsuleDesc::END => make_command!(SetCapsuleEndCommand, handle, value),
                CapsuleDesc::RADIUS => {
                    make_command!(SetCapsuleRadiusCommand, handle, value)
                }
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
) -> Option<SceneCommand> {
    if let ColliderShapeDesc::Segment(_) = collider.shape {
        match property_changed.value {
            FieldKind::Object(ref value) => match property_changed.name.as_ref() {
                SegmentDesc::BEGIN => make_command!(SetSegmentBeginCommand, handle, value),
                SegmentDesc::END => make_command!(SetSegmentEndCommand, handle, value),
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
) -> Option<SceneCommand> {
    if let ColliderShapeDesc::Triangle(_) = collider.shape {
        match property_changed.value {
            FieldKind::Object(ref value) => match property_changed.name.as_ref() {
                TriangleDesc::A => make_command!(SetTriangleACommand, handle, value),
                TriangleDesc::B => make_command!(SetTriangleBCommand, handle, value),
                TriangleDesc::C => make_command!(SetTriangleCCommand, handle, value),
                _ => None,
            },
            _ => None,
        }
    } else {
        None
    }
}
