use crate::{
    handle_properties, handle_property_changed,
    inspector::handlers::node::base::handle_base_property_changed, scene::commands::collider2d::*,
    SceneCommand,
};
use fyrox::{
    core::pool::Handle,
    gui::inspector::{FieldKind, PropertyChanged},
    scene::{
        collider::InteractionGroups,
        dim2::collider::{Collider, *},
        node::Node,
    },
};
use std::any::TypeId;

pub fn handle_collider2d_property_changed(
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
                Collider::SHAPE => SetColliderShapeCommand
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
                } else if inner_property.owner_type_id == TypeId::of::<CapsuleShape>() {
                    handle_capsule(handle, inner_property)
                } else if inner_property.owner_type_id == TypeId::of::<SegmentShape>() {
                    handle_segment(handle, inner_property)
                } else if inner_property.owner_type_id == TypeId::of::<TriangleShape>() {
                    handle_triangle(handle, inner_property)
                } else if inner_property.owner_type_id == TypeId::of::<TrimeshShape>() {
                    handle_trimesh(handle, inner_property)
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
    _handle: Handle<Node>,
    _property_changed: &PropertyChanged,
) -> Option<SceneCommand> {
    // TODO
    None
}
