use crate::inspector::handlers::node::base::handle_base_property_changed;
use crate::{make_command, scene::commands::collider::*, SceneCommand};
use rg3d::gui::inspector::CollectionChanged;
use rg3d::scene::collider::Collider;
use rg3d::scene::node::Node;
use rg3d::{
    core::pool::Handle,
    gui::inspector::{FieldKind, PropertyChanged},
    scene::collider::*,
};
use std::any::TypeId;

pub fn handle_collider_property_changed(
    args: &PropertyChanged,
    handle: Handle<Node>,
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
            Collider::SHAPE => {
                make_command!(SetColliderShapeCommand, handle, value)
            }
            _ => None,
        },
        FieldKind::Inspectable(ref inner_property) => match args.name.as_ref() {
            Collider::COLLISION_GROUPS => match inner_property.value {
                FieldKind::Object(ref value) => match inner_property.name.as_ref() {
                    InteractionGroupsDesc::MEMBERSHIPS => {
                        let mut new_value = collider.collision_groups();
                        new_value.memberships = value.cast_clone()?;
                        Some(SceneCommand::new(SetColliderCollisionGroupsCommand::new(
                            handle, new_value,
                        )))
                    }
                    InteractionGroupsDesc::FILTER => {
                        let mut new_value = collider.collision_groups();
                        new_value.filter = value.cast_clone()?;
                        Some(SceneCommand::new(SetColliderCollisionGroupsCommand::new(
                            handle, new_value,
                        )))
                    }
                    _ => None,
                },
                _ => None,
            },
            Collider::SOLVER_GROUPS => match inner_property.value {
                FieldKind::Object(ref value) => match inner_property.name.as_ref() {
                    InteractionGroupsDesc::MEMBERSHIPS => {
                        let mut new_value = collider.collision_groups();
                        new_value.memberships = value.cast_clone()?;
                        Some(SceneCommand::new(SetColliderSolverGroupsCommand::new(
                            handle, new_value,
                        )))
                    }
                    InteractionGroupsDesc::FILTER => {
                        let mut new_value = collider.collision_groups();
                        new_value.filter = value.cast_clone()?;
                        Some(SceneCommand::new(SetColliderSolverGroupsCommand::new(
                            handle, new_value,
                        )))
                    }
                    _ => None,
                },
                _ => None,
            },
            Collider::SHAPE => {
                if inner_property.owner_type_id == TypeId::of::<CuboidShape>() {
                    handle_cuboid_desc_property_changed(handle, collider, inner_property)
                } else if inner_property.owner_type_id == TypeId::of::<BallShape>() {
                    handle_ball_desc_property_changed(handle, collider, inner_property)
                } else if inner_property.owner_type_id == TypeId::of::<CylinderShape>() {
                    handle_cylinder_desc_property_changed(handle, collider, inner_property)
                } else if inner_property.owner_type_id == TypeId::of::<RoundCylinderShape>() {
                    handle_round_cylinder_desc_property_changed(handle, collider, inner_property)
                } else if inner_property.owner_type_id == TypeId::of::<ConeShape>() {
                    handle_cone_desc_property_changed(handle, collider, inner_property)
                } else if inner_property.owner_type_id == TypeId::of::<CapsuleShape>() {
                    handle_capsule_desc_property_changed(handle, collider, inner_property)
                } else if inner_property.owner_type_id == TypeId::of::<SegmentShape>() {
                    handle_segment_desc_property_changed(handle, collider, inner_property)
                } else if inner_property.owner_type_id == TypeId::of::<TriangleShape>() {
                    handle_triangle_desc_property_changed(handle, collider, inner_property)
                } else if inner_property.owner_type_id == TypeId::of::<TrimeshShape>() {
                    handle_trimesh_desc_property_changed(handle, collider, inner_property)
                } else {
                    None
                }
            }
            Collider::BASE => handle_base_property_changed(inner_property, handle, collider),
            _ => None,
        },
        _ => None,
    }
}

fn handle_ball_desc_property_changed(
    handle: Handle<Node>,
    collider: &Collider,
    property_changed: &PropertyChanged,
) -> Option<SceneCommand> {
    if let ColliderShapeDesc::Ball(_) = collider.shape() {
        match property_changed.value {
            FieldKind::Object(ref value) => match property_changed.name.as_ref() {
                BallShape::RADIUS => make_command!(SetBallRadiusCommand, handle, value),
                _ => None,
            },
            _ => None,
        }
    } else {
        None
    }
}

fn handle_cuboid_desc_property_changed(
    handle: Handle<Node>,
    collider: &Collider,
    property_changed: &PropertyChanged,
) -> Option<SceneCommand> {
    if let ColliderShapeDesc::Cuboid(_) = collider.shape() {
        match property_changed.value {
            FieldKind::Object(ref value) => match property_changed.name.as_ref() {
                CuboidShape::HALF_EXTENTS => {
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
    handle: Handle<Node>,
    collider: &Collider,
    property_changed: &PropertyChanged,
) -> Option<SceneCommand> {
    if let ColliderShapeDesc::Cylinder(_) = collider.shape() {
        match property_changed.value {
            FieldKind::Object(ref value) => match property_changed.name.as_ref() {
                CylinderShape::HALF_HEIGHT => {
                    make_command!(SetCylinderHalfHeightCommand, handle, value)
                }
                CylinderShape::RADIUS => {
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
    handle: Handle<Node>,
    collider: &Collider,
    property_changed: &PropertyChanged,
) -> Option<SceneCommand> {
    if let ColliderShapeDesc::RoundCylinder(_) = collider.shape() {
        match property_changed.value {
            FieldKind::Object(ref value) => match property_changed.name.as_ref() {
                RoundCylinderShape::HALF_HEIGHT => {
                    make_command!(SetRoundCylinderHalfHeightCommand, handle, value)
                }
                RoundCylinderShape::RADIUS => {
                    make_command!(SetRoundCylinderRadiusCommand, handle, value)
                }
                RoundCylinderShape::BORDER_RADIUS => {
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
    handle: Handle<Node>,
    collider: &Collider,
    property_changed: &PropertyChanged,
) -> Option<SceneCommand> {
    if let ColliderShapeDesc::Cone(_) = collider.shape() {
        match property_changed.value {
            FieldKind::Object(ref value) => match property_changed.name.as_ref() {
                ConeShape::HALF_HEIGHT => {
                    make_command!(SetConeHalfHeightCommand, handle, value)
                }
                ConeShape::RADIUS => make_command!(SetConeRadiusCommand, handle, value),
                _ => None,
            },
            _ => None,
        }
    } else {
        None
    }
}

fn handle_capsule_desc_property_changed(
    handle: Handle<Node>,
    collider: &Collider,
    property_changed: &PropertyChanged,
) -> Option<SceneCommand> {
    if let ColliderShapeDesc::Capsule(_) = collider.shape() {
        match property_changed.value {
            FieldKind::Object(ref value) => match property_changed.name.as_ref() {
                CapsuleShape::BEGIN => make_command!(SetCapsuleBeginCommand, handle, value),
                CapsuleShape::END => make_command!(SetCapsuleEndCommand, handle, value),
                CapsuleShape::RADIUS => {
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
    handle: Handle<Node>,
    collider: &Collider,
    property_changed: &PropertyChanged,
) -> Option<SceneCommand> {
    if let ColliderShapeDesc::Segment(_) = collider.shape() {
        match property_changed.value {
            FieldKind::Object(ref value) => match property_changed.name.as_ref() {
                SegmentShape::BEGIN => make_command!(SetSegmentBeginCommand, handle, value),
                SegmentShape::END => make_command!(SetSegmentEndCommand, handle, value),
                _ => None,
            },
            _ => None,
        }
    } else {
        None
    }
}

fn handle_triangle_desc_property_changed(
    handle: Handle<Node>,
    collider: &Collider,
    property_changed: &PropertyChanged,
) -> Option<SceneCommand> {
    if let ColliderShapeDesc::Triangle(_) = collider.shape() {
        match property_changed.value {
            FieldKind::Object(ref value) => match property_changed.name.as_ref() {
                TriangleShape::A => make_command!(SetTriangleACommand, handle, value),
                TriangleShape::B => make_command!(SetTriangleBCommand, handle, value),
                TriangleShape::C => make_command!(SetTriangleCCommand, handle, value),
                _ => None,
            },
            _ => None,
        }
    } else {
        None
    }
}

fn handle_trimesh_desc_property_changed(
    handle: Handle<Node>,
    collider: &Collider,
    property_changed: &PropertyChanged,
) -> Option<SceneCommand> {
    if let ColliderShapeDesc::Trimesh(_) = collider.shape() {
        match property_changed.name.as_ref() {
            TrimeshShape::SOURCES => match property_changed.value {
                FieldKind::Collection(ref collection_changed) => match **collection_changed {
                    CollectionChanged::Add => {
                        Some(SceneCommand::new(AddTrimeshGeometrySourceCommand {
                            node: handle,
                            source: Default::default(),
                        }))
                    }
                    CollectionChanged::Remove(_) => None,
                    CollectionChanged::ItemChanged {
                        index,
                        ref property,
                    } => {
                        if let FieldKind::Object(ref value) = property.value {
                            Some(SceneCommand::new(
                                SetTrimeshColliderGeometrySourceValueCommand {
                                    node: handle,
                                    index,
                                    value: GeometrySource(value.cast_clone()?),
                                },
                            ))
                        } else {
                            None
                        }
                    }
                },
                _ => None,
            },
            _ => None,
        }
    } else {
        None
    }
}
