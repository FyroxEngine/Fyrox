use crate::{
    command::Command, define_node_command, define_swap_command, scene::commands::SceneContext,
};
use fyrox::{
    core::{algebra::Vector2, pool::Handle},
    scene::{collider::InteractionGroups, dim2::collider::*, graph::Graph, node::Node},
};

macro_rules! define_collider_variant_command {
    ($name:ident($human_readable_name:expr, $value_type:ty) where fn swap($self:ident, $node:ident, $variant:ident, $var:ident) $apply_method:block ) => {
        define_node_command!($name($human_readable_name, $value_type) where fn swap($self, $node) {
            if let ColliderShape::$variant(ref mut $var) = *$node.as_collider2d_mut().shape_mut() {
                $apply_method
            } else {
                unreachable!();
            }
        });
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

define_collider_variant_command!(SetCuboidHalfExtentsCommand("Set 2D Cuboid Half Extents", Vector2<f32>) where fn swap(self, physics, Cuboid, cuboid) {
    std::mem::swap(&mut cuboid.half_extents, &mut self.value);
});

define_collider_variant_command!(SetCapsuleRadiusCommand("Set 2D Capsule Radius", f32) where fn swap(self, physics, Capsule, capsule) {
    std::mem::swap(&mut capsule.radius, &mut self.value);
});

define_collider_variant_command!(SetCapsuleBeginCommand("Set 2D Capsule Begin", Vector2<f32>) where fn swap(self, physics, Capsule, capsule) {
    std::mem::swap(&mut capsule.begin, &mut self.value);
});

define_collider_variant_command!(SetCapsuleEndCommand("Set 2D Capsule End", Vector2<f32>) where fn swap(self, physics, Capsule, capsule) {
    std::mem::swap(&mut capsule.end, &mut self.value);
});

define_collider_variant_command!(SetSegmentBeginCommand("Set 2D Segment Begin", Vector2<f32>) where fn swap(self, physics, Segment, segment) {
    std::mem::swap(&mut segment.begin, &mut self.value);
});

define_collider_variant_command!(SetSegmentEndCommand("Set 2D Segment End", Vector2<f32>) where fn swap(self, physics, Segment, segment) {
    std::mem::swap(&mut segment.end, &mut self.value);
});

define_collider_variant_command!(SetTriangleACommand("Set 2D Triangle A", Vector2<f32>) where fn swap(self, physics, Triangle, triangle) {
    std::mem::swap(&mut triangle.a, &mut self.value);
});

define_collider_variant_command!(SetTriangleBCommand("Set 2D Triangle B", Vector2<f32>) where fn swap(self, physics, Triangle, triangle) {
    std::mem::swap(&mut triangle.b, &mut self.value);
});

define_collider_variant_command!(SetTriangleCCommand("Set 2D Triangle C", Vector2<f32>) where fn swap(self, physics, Triangle, triangle) {
    std::mem::swap(&mut triangle.c, &mut self.value);
});

define_collider_variant_command!(SetBallRadiusCommand("Set 2D Ball Radius", f32) where fn swap(self, physics, Ball, ball) {
    std::mem::swap(&mut ball.radius, &mut self.value);
});
