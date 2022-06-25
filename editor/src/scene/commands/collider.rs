use crate::{
    command::Command, define_node_command, define_swap_command, scene::commands::SceneContext,
};
use fyrox::{
    core::{algebra::Vector3, pool::Handle},
    scene::{collider::*, graph::physics::CoefficientCombineRule, graph::Graph, node::Node},
};

macro_rules! define_collider_shape_variant_command {
    ($($ty_name:ident($value_ty:ty): $variant:ident, $field:ident, $name:expr;)*) => {
        $(
            define_swap_command! {
                $ty_name($value_ty): $name, |me: &mut $ty_name, graph: &mut Graph| {
                    let node = &mut graph[me.handle];
                    let variant = match *node.as_collider_mut().shape_mut() {
                        ColliderShape::$variant(ref mut x) => x,
                        _ => unreachable!()
                    };
                    ::core::mem::swap(&mut variant.$field, &mut me.value);
                }
            }
        )*
    };
}

macro_rules! define_collider_variant_command {
    ($($name:ident($human_readable_name:expr, $value_type:ty) where fn swap($self:ident, $node:ident, $variant:ident, $var:ident) $apply_method:block )*) => {
        $(
            define_node_command!($name($human_readable_name, $value_type) where fn swap($self, $node) {
                if let ColliderShape::$variant(ref mut $var) = *$node.as_collider_mut().shape_mut() {
                    $apply_method
                } else {
                    unreachable!();
                }
            });
        )*
    };
}

define_swap_command! {
    Node::as_collider_mut,
    SetColliderShapeCommand(ColliderShape): shape_value, set_shape, "Set Collider Shape";
    SetColliderFrictionCommand(f32): friction, set_friction, "Set Collider Friction";
    SetColliderRestitutionCommand(f32): restitution, set_restitution, "Set Collider Restitution";
    SetColliderIsSensorCommand(bool): is_sensor, set_is_sensor, "Set Collider Is Sensor";
    SetColliderDensityCommand(Option<f32>): density, set_density, "Set Collider Density";
    SetColliderFrictionCombineRule(CoefficientCombineRule): friction_combine_rule, set_friction_combine_rule, "Set Collider Friction Combine Rule";
    SetColliderRestitutionCombineRule(CoefficientCombineRule): restitution_combine_rule, set_restitution_combine_rule, "Set Collider Restitution Combine Rule";
    SetColliderCollisionGroupsCommand(InteractionGroups): collision_groups, set_collision_groups, "Set Collider Collision Groups";
    SetColliderSolverGroupsCommand(InteractionGroups): solver_groups, set_solver_groups, "Set Collider Solver Groups";
}

define_collider_shape_variant_command! {
    SetCylinderHalfHeightCommand(f32): Cylinder, half_height, "Set Cylinder Half Height";
    SetCylinderRadiusCommand(f32): Cylinder, radius, "Set Cylinder Radius";
    SetConeHalfHeightCommand(f32): Cone, half_height, "Set Cone Half Height";
    SetConeRadiusCommand(f32): Cone, radius, "Set Cone Radius";
    SetCuboidHalfExtentsCommand(Vector3<f32>): Cuboid, half_extents, "Set Cuboid Half Extents";
    SetCapsuleRadiusCommand(f32): Capsule, radius, "Set Capsule Radius";
    SetCapsuleBeginCommand(Vector3<f32>): Capsule, begin, "Set Capsule Begin";
    SetCapsuleEndCommand(Vector3<f32>): Capsule, end, "Set Capsule End";
    SetSegmentBeginCommand(Vector3<f32>): Segment, begin, "Set Segment Begin";
    SetSegmentEndCommand(Vector3<f32>): Segment, end, "Set Segment End";
    SetTriangleACommand(Vector3<f32>): Triangle, a, "Set Triangle A";
    SetTriangleBCommand(Vector3<f32>): Triangle, b, "Set Triangle B";
    SetTriangleCCommand(Vector3<f32>): Triangle, c, "Set Triangle C";
    SetBallRadiusCommand(f32): Ball, radius, "Set Ball Radius";
}

define_collider_variant_command! {
    SetHeightfieldSourceCommand("Set Heightfield Polyhedron Source", Handle<Node>) where fn swap(self, physics, Heightfield, hf) {
        std::mem::swap(&mut hf.geometry_source.0, &mut self.value);
    }

    SetPolyhedronSourceCommand("Set Polyhedron Source", Handle<Node>) where fn swap(self, physics, Polyhedron, ph) {
        std::mem::swap(&mut ph.geometry_source.0, &mut self.value);
    }
}

#[derive(Debug)]
pub struct AddTrimeshGeometrySourceCommand {
    pub node: Handle<Node>,
    pub source: GeometrySource,
}

impl Command for AddTrimeshGeometrySourceCommand {
    fn name(&mut self, _: &SceneContext) -> String {
        "Add Trimesh Geometry Source".to_string()
    }

    fn execute(&mut self, context: &mut SceneContext) {
        if let ColliderShape::Trimesh(trimesh) =
            context.scene.graph[self.node].as_collider_mut().shape_mut()
        {
            trimesh.sources.push(self.source)
        } else {
            unreachable!()
        }
    }

    fn revert(&mut self, context: &mut SceneContext) {
        if let ColliderShape::Trimesh(trimesh) =
            context.scene.graph[self.node].as_collider_mut().shape_mut()
        {
            trimesh.sources.pop();
        } else {
            unreachable!()
        }
    }
}

#[derive(Debug)]
pub struct SetTrimeshColliderGeometrySourceValueCommand {
    pub node: Handle<Node>,
    pub index: usize,
    pub value: GeometrySource,
}

impl SetTrimeshColliderGeometrySourceValueCommand {
    fn swap(&mut self, context: &mut SceneContext) {
        if let ColliderShape::Trimesh(trimesh) =
            context.scene.graph[self.node].as_collider_mut().shape_mut()
        {
            std::mem::swap(&mut trimesh.sources[self.index], &mut self.value)
        } else {
            unreachable!()
        }
    }
}

impl Command for SetTrimeshColliderGeometrySourceValueCommand {
    fn name(&mut self, _: &SceneContext) -> String {
        "Set Trimesh Collider Geometry Source".to_string()
    }

    fn execute(&mut self, context: &mut SceneContext) {
        self.swap(context)
    }

    fn revert(&mut self, context: &mut SceneContext) {
        self.swap(context)
    }
}
