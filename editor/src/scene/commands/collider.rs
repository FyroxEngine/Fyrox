use crate::{command::Command, define_node_command, get_set_swap, scene::commands::SceneContext};
use rg3d::{
    core::{algebra::Vector3, pool::Handle},
    scene::{collider::*, graph::Graph, node::Node},
};

macro_rules! define_collider_variant_command {
    ($name:ident($human_readable_name:expr, $value_type:ty) where fn swap($self:ident, $node:ident, $variant:ident, $var:ident) $apply_method:block ) => {
        define_node_command!($name($human_readable_name, $value_type) where fn swap($self, $node) {
            if let ColliderShapeDesc::$variant(ref mut $var) = *$node.as_collider_mut().shape_mut() {
                $apply_method
            } else {
                unreachable!();
            }
        });
    };
}

define_node_command!(SetColliderShapeCommand("Set Collider Shape", ColliderShapeDesc) where fn swap(self, node) {
    get_set_swap!(self, node.as_collider_mut(), shape_value, set_shape)
});

define_node_command!(SetColliderFrictionCommand("Set Collider Friction", f32) where fn swap(self, node) {
    get_set_swap!(self, node.as_collider_mut(), friction, set_friction)
});

define_node_command!(SetColliderRestitutionCommand("Set Collider Restitution", f32) where fn swap(self, node) {
    get_set_swap!(self, node.as_collider_mut(), restitution, set_restitution)
});

define_node_command!(SetColliderIsSensorCommand("Set Collider Is Sensor", bool) where fn swap(self, node) {
    get_set_swap!(self, node.as_collider_mut(), is_sensor, set_is_sensor)
});

define_node_command!(SetColliderDensityCommand("Set Collider Density", Option<f32>) where fn swap(self,node) {
    get_set_swap!(self, node.as_collider_mut(), density, set_density)
});

define_node_command!(SetColliderCollisionGroupsCommand("Set Collider Collision Groups", InteractionGroupsDesc) where fn swap(self, node) {
    get_set_swap!(self, node.as_collider_mut(), collision_groups, set_collision_groups)
});

define_node_command!(SetColliderSolverGroupsCommand("Set Collider Solver Groups", InteractionGroupsDesc) where fn swap(self, node) {
    get_set_swap!(self, node.as_collider_mut(), solver_groups, set_solver_groups)
});

define_collider_variant_command!(SetCylinderHalfHeightCommand("Set Cylinder Half Height", f32) where fn swap(self, node, Cylinder, cylinder) {
    std::mem::swap(&mut cylinder.half_height, &mut self.value);
});

define_collider_variant_command!(SetCylinderRadiusCommand("Set Cylinder Radius", f32) where fn swap(self, physics, Cylinder, cylinder) {
    std::mem::swap(&mut cylinder.radius, &mut self.value);
});

define_collider_variant_command!(SetRoundCylinderHalfHeightCommand("Set Cylinder Half Height", f32) where fn swap(self, physics, RoundCylinder, round_cylinder) {
    std::mem::swap(&mut round_cylinder.half_height, &mut self.value);
});

define_collider_variant_command!(SetRoundCylinderRadiusCommand("Set Round Cylinder Radius", f32) where fn swap(self, physics, RoundCylinder, round_cylinder) {
    std::mem::swap(&mut round_cylinder.radius, &mut self.value);
});

define_collider_variant_command!(SetRoundCylinderBorderRadiusCommand("Set Round Cylinder Border Radius", f32) where fn swap(self, physics, RoundCylinder, round_cylinder) {
    std::mem::swap(&mut round_cylinder.border_radius, &mut self.value);
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
        let mut ref_mut = context.scene.graph[self.node].as_collider_mut().shape_mut();
        if let ColliderShapeDesc::Trimesh(trimesh) = &mut *ref_mut {
            trimesh.sources.push(self.source)
        } else {
            unreachable!()
        }
    }

    fn revert(&mut self, context: &mut SceneContext) {
        let mut ref_mut = context.scene.graph[self.node].as_collider_mut().shape_mut();
        if let ColliderShapeDesc::Trimesh(trimesh) = &mut *ref_mut {
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
        let mut ref_mut = context.scene.graph[self.node].as_collider_mut().shape_mut();
        if let ColliderShapeDesc::Trimesh(trimesh) = &mut *ref_mut {
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
