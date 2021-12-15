use crate::{command::Command, define_node_command, get_set_swap, scene::commands::SceneContext};
use rg3d::{
    core::{
        algebra::{UnitQuaternion, Vector3},
        pool::Handle,
    },
    physics3d::desc::JointParamsDesc,
    scene::collider::*,
    scene::graph::Graph,
    scene::node::Node,
    scene::rigidbody::*,
};

macro_rules! define_joint_variant_command {
    ($name:ident($human_readable_name:expr, $value_type:ty) where fn swap($self:ident, $node:ident, $variant:ident, $var:ident) $apply_method:block ) => {
        define_node_command!($name($human_readable_name, $value_type) where fn swap($self, $node) {
            if let JointParamsDesc::$variant(ref mut $var) = *$node.as_joint_mut().params_mut() {
                $apply_method
            } else {
                unreachable!();
            }
        });
    };
}

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

define_node_command!(SetBodyMassCommand("Set Body Mass", f32) where fn swap(self, node) {
    get_set_swap!(self, node.as_rigid_body_mut(), mass, set_mass)
});

define_node_command!(SetBodyLinVelCommand("Set Body Linear Velocity", Vector3<f32>) where fn swap(self, node) {
    get_set_swap!(self, node.as_rigid_body_mut(), lin_vel, set_lin_vel)
});

define_node_command!(SetBodyAngVelCommand("Set Body Angular Velocity", Vector3<f32>) where fn swap(self, node) {
    get_set_swap!(self, node.as_rigid_body_mut(), ang_vel, set_ang_vel)
});

define_node_command!(SetBodyStatusCommand("Set Body Status", RigidBodyTypeDesc) where fn swap(self, node) {
    get_set_swap!(self, node.as_rigid_body_mut(), body_type, set_body_type)
});

define_node_command!(SetBodyXRotationLockedCommand("Set Body X Rotation Locked", bool) where fn swap(self, node) {
    get_set_swap!(self, node.as_rigid_body_mut(), is_x_rotation_locked, lock_x_rotations)
});

define_node_command!(SetBodyYRotationLockedCommand("Set Body Y Rotation Locked", bool) where fn swap(self, node) {
    get_set_swap!(self, node.as_rigid_body_mut(), is_y_rotation_locked, lock_y_rotations)
});

define_node_command!(SetBodyZRotationLockedCommand("Set Body Z Rotation Locked", bool) where fn swap(self, node) {
    get_set_swap!(self, node.as_rigid_body_mut(), is_z_rotation_locked, lock_z_rotations)
});

define_node_command!(SetBodyTranslationLockedCommand("Set Body Translation Locked", bool) where fn swap(self, node) {
    get_set_swap!(self, node.as_rigid_body_mut(), is_translation_locked, lock_translation)
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

define_joint_variant_command!(SetBallJointAnchor1Command("Set Ball Joint Anchor 1", Vector3<f32>) where fn swap(self, physics, BallJoint, ball) {
    std::mem::swap(&mut ball.local_anchor1, &mut self.value);
});

define_joint_variant_command!(SetBallJointAnchor2Command("Set Ball Joint Anchor 2", Vector3<f32>) where fn swap(self, physics, BallJoint, ball) {
    std::mem::swap(&mut ball.local_anchor2, &mut self.value);
});

define_joint_variant_command!(SetFixedJointAnchor1TranslationCommand("Set Fixed Joint Anchor 1 Translation", Vector3<f32>) where fn swap(self, physics, FixedJoint, fixed) {
    std::mem::swap(&mut fixed.local_anchor1_translation, &mut self.value);
});

define_joint_variant_command!(SetFixedJointAnchor2TranslationCommand("Set Fixed Joint Anchor 2 Translation", Vector3<f32>) where fn swap(self, physics, FixedJoint, fixed) {
    std::mem::swap(&mut fixed.local_anchor2_translation, &mut self.value);
});

define_joint_variant_command!(SetFixedJointAnchor1RotationCommand("Set Fixed Joint Anchor 1 Rotation", UnitQuaternion<f32>) where fn swap(self, node, FixedJoint, fixed) {
    std::mem::swap(&mut fixed.local_anchor1_rotation, &mut self.value);
});

define_joint_variant_command!(SetFixedJointAnchor2RotationCommand("Set Fixed Joint Anchor 2 Rotation", UnitQuaternion<f32>) where fn swap(self, node, FixedJoint, fixed) {
    std::mem::swap(&mut fixed.local_anchor2_rotation, &mut self.value);
});

define_joint_variant_command!(SetRevoluteJointAnchor1Command("Set Revolute Joint Anchor 1", Vector3<f32>) where fn swap(self, node, RevoluteJoint, revolute) {
    std::mem::swap(&mut revolute.local_anchor1, &mut self.value);
});

define_joint_variant_command!(SetRevoluteJointAxis1Command("Set Revolute Joint Axis 1", Vector3<f32>) where fn swap(self, node, RevoluteJoint, revolute) {
    std::mem::swap(&mut revolute.local_axis1, &mut self.value);
});

define_joint_variant_command!(SetRevoluteJointAnchor2Command("Set Revolute Joint Anchor 2", Vector3<f32>) where fn swap(self, node, RevoluteJoint, revolute) {
    std::mem::swap(&mut revolute.local_anchor2, &mut self.value);
});

define_joint_variant_command!(SetRevoluteJointAxis2Command("Set Prismatic Joint Axis 2", Vector3<f32>) where fn swap(self, node, RevoluteJoint, revolute) {
    std::mem::swap(&mut revolute.local_axis2, &mut self.value);
});

define_joint_variant_command!(SetPrismaticJointAnchor1Command("Set Prismatic Joint Anchor 1", Vector3<f32>) where fn swap(self, node, PrismaticJoint, prismatic) {
    std::mem::swap(&mut prismatic.local_anchor1, &mut self.value);
});

define_joint_variant_command!(SetPrismaticJointAxis1Command("Set Prismatic Joint Axis 1", Vector3<f32>) where fn swap(self, node, PrismaticJoint, prismatic) {
    std::mem::swap(&mut prismatic.local_axis1, &mut self.value);
});

define_joint_variant_command!(SetPrismaticJointAnchor2Command("Set Prismatic Joint Anchor 2", Vector3<f32>) where fn swap(self, node, PrismaticJoint, prismatic) {
    std::mem::swap(&mut prismatic.local_anchor2, &mut self.value);
});

define_joint_variant_command!(SetPrismaticJointAxis2Command("Set Prismatic Joint Axis 2", Vector3<f32>) where fn swap(self, node, PrismaticJoint, prismatic) {
    std::mem::swap(&mut prismatic.local_axis2, &mut self.value);
});

define_node_command!(SetJointBody1Command("Set Joint Body 1", Handle<Node>) where fn swap(self, node) {
    get_set_swap!(self, node.as_joint_mut(), body1, set_body1)
});

define_node_command!(SetJointBody2Command("Set Joint Body 2", Handle<Node>) where fn swap(self, node) {
    get_set_swap!(self, node.as_joint_mut(), body2, set_body2)
});
