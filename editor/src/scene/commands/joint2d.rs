use crate::{command::Command, define_swap_command, scene::commands::SceneContext};
use fyrox::{
    core::{
        algebra::{UnitQuaternion, Vector3},
        pool::Handle,
    },
    scene::{graph::Graph, joint::*, node::Node},
};

macro_rules! define_joint_variant_command {
    ($($ty_name:ident($value_ty:ty): $variant:ident, $field:ident, $name:expr;)*) => {
        $(
            define_swap_command! {
                $ty_name($value_ty): $name, |me: &mut $ty_name, graph: &mut Graph| {
                    let node = &mut graph[me.handle];
                    let variant = match *node.as_joint_mut().params_mut() {
                        JointParams::$variant(ref mut x) => x,
                        _ => unreachable!()
                    };
                    ::core::mem::swap(&mut variant.$field, &mut me.value);
                }
            }
        )*
    };
}

define_joint_variant_command! {
    SetBallJointAnchor1Command(Vector3<f32>): BallJoint, local_anchor1, "Set 2D Ball Joint Anchor 1";
    SetBallJointAnchor2Command(Vector3<f32>): BallJoint, local_anchor2, "Set 2D Ball Joint Anchor 2";
    SetFixedJointAnchor1TranslationCommand(Vector3<f32>): FixedJoint, local_anchor1_translation, "Set 2D Fixed Joint Anchor 1 Translation";
    SetFixedJointAnchor2TranslationCommand(Vector3<f32>): FixedJoint, local_anchor2_translation, "Set 2D Fixed Joint Anchor 2 Translation";
    SetFixedJointAnchor1RotationCommand(UnitQuaternion<f32>): FixedJoint, local_anchor1_rotation, "Set 2D Fixed Joint Anchor 1 Rotation";
    SetFixedJointAnchor2RotationCommand(UnitQuaternion<f32>): FixedJoint, local_anchor2_rotation, "Set 2D Fixed Joint Anchor 2 Rotation";
    SetPrismaticJointAnchor1Command(Vector3<f32>): PrismaticJoint, local_anchor1, "Set 2D Prismatic Joint Anchor 1";
    SetPrismaticJointAxis1Command(Vector3<f32>): PrismaticJoint, local_axis1, "Set 2D Prismatic Joint Axis 1";
    SetPrismaticJointAnchor2Command(Vector3<f32>): PrismaticJoint, local_anchor2, "Set 2D Prismatic Joint Anchor 2";
    SetPrismaticJointAxis2Command(Vector3<f32>): PrismaticJoint, local_axis2, "Set 2D Prismatic Joint Axis 2";
}

define_swap_command! {
    Node::as_joint2d_mut,
    SetJointBody1Command(Handle<Node>): body1, set_body1, "Set 2D Joint Body 1";
    SetJointBody2Command(Handle<Node>): body2, set_body2, "Set 2D Joint Body 2";
}
