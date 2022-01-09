use crate::{command::Command, define_node_command, get_set_swap, scene::commands::SceneContext};
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

define_node_command!(SetColliderShapeCommand("Set 2D Collider Shape", ColliderShape) where fn swap(self, node) {
    get_set_swap!(self, node.as_collider2d_mut(), shape_value, set_shape)
});

define_node_command!(SetColliderFrictionCommand("Set 2D Collider Friction", f32) where fn swap(self, node) {
    get_set_swap!(self, node.as_collider2d_mut(), friction, set_friction)
});

define_node_command!(SetColliderRestitutionCommand("Set 2D Collider Restitution", f32) where fn swap(self, node) {
    get_set_swap!(self, node.as_collider2d_mut(), restitution, set_restitution)
});

define_node_command!(SetColliderIsSensorCommand("Set 2D Collider Is Sensor", bool) where fn swap(self, node) {
    get_set_swap!(self, node.as_collider2d_mut(), is_sensor, set_is_sensor)
});

define_node_command!(SetColliderDensityCommand("Set 2D Collider Density", Option<f32>) where fn swap(self,node) {
    get_set_swap!(self, node.as_collider2d_mut(), density, set_density)
});

define_node_command!(SetColliderCollisionGroupsCommand("Set 2D Collider Collision Groups", InteractionGroups) where fn swap(self, node) {
    get_set_swap!(self, node.as_collider2d_mut(), collision_groups, set_collision_groups)
});

define_node_command!(SetColliderSolverGroupsCommand("Set 2D Collider Solver Groups", InteractionGroups) where fn swap(self, node) {
    get_set_swap!(self, node.as_collider2d_mut(), solver_groups, set_solver_groups)
});

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
