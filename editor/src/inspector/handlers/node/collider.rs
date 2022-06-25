use crate::{
    handle_properties, handle_property_changed,
    inspector::handlers::node::base::handle_base_property_changed, scene::commands::collider::*,
    SceneCommand,
};
use fyrox::{
    core::pool::Handle,
    gui::inspector::{CollectionChanged, FieldKind, PropertyChanged},
    scene::{
        collider::{Collider, *},
        node::Node,
    },
};
use std::any::TypeId;

pub fn handle_collider_property_changed(
    args: &PropertyChanged,
    handle: Handle<Node>,
    collider: &mut Collider,
) -> Option<SceneCommand> {
    match args.value {
        FieldKind::Object(ref value) => {
            handle_properties!(args.name.as_ref(), handle, value,
                Collider::FRICTION => SetColliderFrictionCommand,
                Collider::RESTITUTION => SetColliderRestitutionCommand,
                Collider::IS_SENSOR => SetColliderIsSensorCommand,
                Collider::DENSITY => SetColliderDensityCommand,
                Collider::SHAPE => SetColliderShapeCommand,
                Collider::FRICTION_COMBINE_RULE => SetColliderFrictionCombineRule,
                Collider::RESTITUTION_COMBINE_RULE => SetColliderRestitutionCombineRule
            )
        }
        FieldKind::Inspectable(ref inner_property) => match args.name.as_ref() {
            Collider::COLLISION_GROUPS => match inner_property.value {
                FieldKind::Object(ref value) => match inner_property.name.as_ref() {
                    InteractionGroups::MEMBERSHIPS => {
                        let mut new_value = collider.collision_groups();
                        new_value.memberships = value.cast_clone()?;
                        Some(SceneCommand::new(SetColliderCollisionGroupsCommand::new(
                            handle, new_value,
                        )))
                    }
                    InteractionGroups::FILTER => {
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
                    InteractionGroups::MEMBERSHIPS => {
                        let mut new_value = collider.collision_groups();
                        new_value.memberships = value.cast_clone()?;
                        Some(SceneCommand::new(SetColliderSolverGroupsCommand::new(
                            handle, new_value,
                        )))
                    }
                    InteractionGroups::FILTER => {
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
                    handle_cuboid(handle, inner_property)
                } else if inner_property.owner_type_id == TypeId::of::<BallShape>() {
                    handle_ball(handle, inner_property)
                } else if inner_property.owner_type_id == TypeId::of::<CylinderShape>() {
                    handle_cylinder(handle, inner_property)
                } else if inner_property.owner_type_id == TypeId::of::<ConeShape>() {
                    handle_cone(handle, inner_property)
                } else if inner_property.owner_type_id == TypeId::of::<CapsuleShape>() {
                    handle_capsule(handle, inner_property)
                } else if inner_property.owner_type_id == TypeId::of::<SegmentShape>() {
                    handle_segment(handle, inner_property)
                } else if inner_property.owner_type_id == TypeId::of::<TriangleShape>() {
                    handle_triangle(handle, inner_property)
                } else if inner_property.owner_type_id == TypeId::of::<TrimeshShape>() {
                    handle_trimesh(handle, inner_property)
                } else if inner_property.owner_type_id == TypeId::of::<HeightfieldShape>() {
                    handle_heightfield(handle, inner_property)
                } else if inner_property.owner_type_id == TypeId::of::<ConvexPolyhedronShape>() {
                    handle_convex_polyhedron(handle, inner_property)
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

fn handle_ball(handle: Handle<Node>, args: &PropertyChanged) -> Option<SceneCommand> {
    handle_property_changed!(args, handle,
        BallShape::RADIUS => SetBallRadiusCommand
    )
}

fn handle_cuboid(handle: Handle<Node>, args: &PropertyChanged) -> Option<SceneCommand> {
    handle_property_changed!(args, handle,
         CuboidShape::HALF_EXTENTS => SetCuboidHalfExtentsCommand
    )
}

fn handle_cylinder(handle: Handle<Node>, args: &PropertyChanged) -> Option<SceneCommand> {
    handle_property_changed!(args, handle,
         CylinderShape::HALF_HEIGHT => SetCylinderHalfHeightCommand,
         CylinderShape::RADIUS => SetCylinderRadiusCommand
    )
}

fn handle_cone(handle: Handle<Node>, args: &PropertyChanged) -> Option<SceneCommand> {
    handle_property_changed!(args, handle,
        ConeShape::HALF_HEIGHT => SetConeHalfHeightCommand,
        ConeShape::RADIUS => SetConeRadiusCommand
    )
}

fn handle_capsule(handle: Handle<Node>, args: &PropertyChanged) -> Option<SceneCommand> {
    handle_property_changed!(args, handle,
        CapsuleShape::BEGIN => SetCapsuleBeginCommand,
        CapsuleShape::END => SetCapsuleEndCommand,
        CapsuleShape::RADIUS => SetCapsuleRadiusCommand
    )
}

fn handle_segment(handle: Handle<Node>, args: &PropertyChanged) -> Option<SceneCommand> {
    handle_property_changed!(args, handle,
        SegmentShape::BEGIN => SetSegmentBeginCommand,
        SegmentShape::END => SetSegmentEndCommand
    )
}

fn handle_triangle(handle: Handle<Node>, args: &PropertyChanged) -> Option<SceneCommand> {
    handle_property_changed!(args, handle,
        TriangleShape::A => SetTriangleACommand,
        TriangleShape::B => SetTriangleBCommand,
        TriangleShape::C => SetTriangleCCommand
    )
}

fn handle_trimesh(
    handle: Handle<Node>,
    property_changed: &PropertyChanged,
) -> Option<SceneCommand> {
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
}

fn handle_heightfield(handle: Handle<Node>, args: &PropertyChanged) -> Option<SceneCommand> {
    if args.name == HeightfieldShape::GEOMETRY_SOURCE {
        if let FieldKind::Inspectable(ref inner) = args.value {
            if inner.name == GeometrySource::F_0 {
                if let FieldKind::Object(ref val) = inner.value {
                    return Some(SceneCommand::new(SetHeightfieldSourceCommand::new(
                        handle,
                        val.cast_clone()?,
                    )));
                }
            }
        }
    }
    None
}

fn handle_convex_polyhedron(handle: Handle<Node>, args: &PropertyChanged) -> Option<SceneCommand> {
    if args.name == ConvexPolyhedronShape::GEOMETRY_SOURCE {
        if let FieldKind::Inspectable(ref inner) = args.value {
            if inner.name == GeometrySource::F_0 {
                if let FieldKind::Object(ref val) = inner.value {
                    return Some(SceneCommand::new(SetPolyhedronSourceCommand::new(
                        handle,
                        val.cast_clone()?,
                    )));
                }
            }
        }
    }
    None
}
