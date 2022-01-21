use crate::{
    command::Command, define_node_command, define_swap_command, scene::commands::SceneContext,
};
use fyrox::{
    core::{algebra::Vector3, pool::Handle},
    scene::{collider::*, graph::Graph, node::Node},
};

macro_rules! define_collider_variant_command {
    ($name:ident($human_readable_name:expr, $value_type:ty) where fn swap($self:ident, $node:ident, $variant:ident, $var:ident) $apply_method:block ) => {
        define_node_command!($name($human_readable_name, $value_type) where fn swap($self, $node) {
            if let ColliderShape::$variant(ref mut $var) = *$node.as_collider_mut().shape_mut() {
                $apply_method
            } else {
                unreachable!();
            }
        });
    };
}

define_swap_command! {
    Node::as_collider_mut,
    SetColliderShapeCommand(ColliderShape): shape_value, set_shape, "Set Collider Shape";
    SetColliderFrictionCommand(f32): friction, set_friction, "Set Collider Friction";
    SetColliderRestitutionCommand(f32): restitution, set_restitution, "Set Collider Restitution";
    SetColliderIsSensorCommand(bool): is_sensor, set_is_sensor, "Set Collider Is Sensor";
    SetColliderDensityCommand(Option<f32>): density, set_density, "Set Collider Density";
    SetColliderCollisionGroupsCommand(InteractionGroups): collision_groups, set_collision_groups, "Set Collider Collision Groups";
    SetColliderSolverGroupsCommand(InteractionGroups): solver_groups, set_solver_groups, "Set Collider Solver Groups";
}

define_collider_variant_command!(SetCylinderHalfHeightCommand("Set Cylinder Half Height", f32) where fn swap(self, node, Cylinder, cylinder) {
    std::mem::swap(&mut cylinder.half_height, &mut self.value);
});

define_collider_variant_command!(SetCylinderRadiusCommand("Set Cylinder Radius", f32) where fn swap(self, physics, Cylinder, cylinder) {
    std::mem::swap(&mut cylinder.radius, &mut self.value);
});

define_collider_variant_command!(SetConeHalfHeightCommand("Set Cone Half Height", f32) where fn swap(self, physics, Cone, cone) {
    std::mem::swap(&mut cone.half_height, &mut self.value);
});

define_collider_variant_command!(SetConeRadiusCommand("Set Cone Radius", f32) where fn swap(self, physics, Cone, cone) {
    std::mem::swap(&mut cone.radius, &mut self.value);
});

define_collider_variant_command!(SetCuboidHalfExtentsCommand("Set Cuboid Half Extents", Vector3<f32>) where fn swap(self, physics, Cuboid, cuboid) {
    std::mem::swap(&mut cuboid.half_extents, &mut self.value);
});

define_collider_variant_command!(SetCapsuleRadiusCommand("Set Capsule Radius", f32) where fn swap(self, physics, Capsule, capsule) {
    std::mem::swap(&mut capsule.radius, &mut self.value);
});

define_collider_variant_command!(SetCapsuleBeginCommand("Set Capsule Begin", Vector3<f32>) where fn swap(self, physics, Capsule, capsule) {
    std::mem::swap(&mut capsule.begin, &mut self.value);
});

define_collider_variant_command!(SetCapsuleEndCommand("Set Capsule End", Vector3<f32>) where fn swap(self, physics, Capsule, capsule) {
    std::mem::swap(&mut capsule.end, &mut self.value);
});

define_collider_variant_command!(SetSegmentBeginCommand("Set Segment Begin", Vector3<f32>) where fn swap(self, physics, Segment, segment) {
    std::mem::swap(&mut segment.begin, &mut self.value);
});

define_collider_variant_command!(SetSegmentEndCommand("Set Segment End", Vector3<f32>) where fn swap(self, physics, Segment, segment) {
    std::mem::swap(&mut segment.end, &mut self.value);
});

define_collider_variant_command!(SetTriangleACommand("Set Triangle A", Vector3<f32>) where fn swap(self, physics, Triangle, triangle) {
    std::mem::swap(&mut triangle.a, &mut self.value);
});

define_collider_variant_command!(SetTriangleBCommand("Set Triangle B", Vector3<f32>) where fn swap(self, physics, Triangle, triangle) {
    std::mem::swap(&mut triangle.b, &mut self.value);
});

define_collider_variant_command!(SetTriangleCCommand("Set Triangle C", Vector3<f32>) where fn swap(self, physics, Triangle, triangle) {
    std::mem::swap(&mut triangle.c, &mut self.value);
});

define_collider_variant_command!(SetBallRadiusCommand("Set Ball Radius", f32) where fn swap(self, physics, Ball, ball) {
    std::mem::swap(&mut ball.radius, &mut self.value);
});

define_collider_variant_command!(SetHeightfieldSourceCommand("Set Heightfield Polyhedron Source", Handle<Node>) where fn swap(self, physics, Heightfield, hf) {
    std::mem::swap(&mut hf.geometry_source.0, &mut self.value);
});

define_collider_variant_command!(SetPolyhedronSourceCommand("Set Polyhedron Source", Handle<Node>) where fn swap(self, physics, Polyhedron, ph) {
    std::mem::swap(&mut ph.geometry_source.0, &mut self.value);
});

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
