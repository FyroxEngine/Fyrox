use crate::{command::Command, define_swap_command, scene::commands::SceneContext};
use fyrox::{
    core::algebra::Vector2,
    scene::{collider::InteractionGroups, dim2::collider::*, graph::Graph, node::Node},
};

macro_rules! define_collider_variant_command {
    ($($ty_name:ident($value_ty:ty): $variant:ident, $field:ident, $name:expr;)*) => {
        $(
            define_swap_command! {
                $ty_name($value_ty): $name, |me: &mut $ty_name, graph: &mut Graph| {
                    let node = &mut graph[me.handle];
                    let variant = match *node.as_collider2d_mut().shape_mut() {
                        ColliderShape::$variant(ref mut x) => x,
                        _ => unreachable!()
                    };
                    ::core::mem::swap(&mut variant.$field, &mut me.value);
                }
            }
        )*
    };
}

define_swap_command! {
    Node::as_collider2d_mut,
    SetColliderShapeCommand(ColliderShape): shape_value, set_shape, "Set 2D Collider Shape";
    SetColliderFrictionCommand(f32): friction, set_friction, "Set 2D Collider Friction";
    SetColliderRestitutionCommand(f32): restitution, set_restitution, "Set 2D Collider Restitution";
    SetColliderIsSensorCommand(bool): is_sensor, set_is_sensor, "Set 2D Collider Is Sensor";
    SetColliderDensityCommand(Option<f32>): density, set_density, "Set 2D Collider Density";
    SetColliderCollisionGroupsCommand(InteractionGroups): collision_groups, set_collision_groups, "Set 2D Collider Collision Groups";
    SetColliderSolverGroupsCommand(InteractionGroups): solver_groups, set_solver_groups, "Set 2D Collider Solver Groups";
}

define_collider_variant_command! {
    SetCuboidHalfExtentsCommand(Vector2<f32>): Cuboid, half_extents, "Set 2D Cuboid Half Extents";
    SetCapsuleRadiusCommand(f32): Capsule, radius, "Set 2D Capsule Radius";
    SetCapsuleBeginCommand(Vector2<f32>): Capsule, begin, "Set 2D Capsule Begin";
    SetCapsuleEndCommand(Vector2<f32>): Capsule, end, "Set 2D Capsule End";
    SetSegmentBeginCommand(Vector2<f32>): Segment, begin, "Set 2D Segment Begin";
    SetSegmentEndCommand(Vector2<f32>): Segment, end, "Set 2D Segment End";
    SetTriangleACommand(Vector2<f32>): Triangle, a, "Set 2D Triangle A";
    SetTriangleBCommand(Vector2<f32>): Triangle, b, "Set 2D Triangle B";
    SetTriangleCCommand(Vector2<f32>): Triangle, c, "Set 2D Triangle C";
    SetBallRadiusCommand(f32): Ball, radius, "Set 2D Ball Radius";
}
