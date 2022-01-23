use crate::{
    command::Command, define_node_command, define_swap_command, scene::commands::SceneContext,
};
use fyrox::{
    core::{
        algebra::{UnitQuaternion, Vector3},
        pool::Handle,
    },
    scene::{graph::Graph, joint::*, node::Node},
};

macro_rules! define_joint_variant_command {
    ($($name:ident($human_readable_name:expr, $value_type:ty) where fn swap($self:ident, $node:ident, $variant:ident, $var:ident) $apply_method:block )*) => {
        $(
            define_node_command!($name($human_readable_name, $value_type) where fn swap($self, $node) {
                if let JointParams::$variant(ref mut $var) = *$node.as_joint_mut().params_mut() {
                    $apply_method
                } else {
                    unreachable!();
                }
            });
        )*
    };
}

define_joint_variant_command! {
    SetBallJointAnchor1Command("Set 2D Ball Joint Anchor 1", Vector3<f32>) where fn swap(self, physics, BallJoint, ball) {
        std::mem::swap(&mut ball.local_anchor1, &mut self.value);
    }

    SetBallJointAnchor2Command("Set 2D Ball Joint Anchor 2", Vector3<f32>) where fn swap(self, physics, BallJoint, ball) {
        std::mem::swap(&mut ball.local_anchor2, &mut self.value);
    }

    SetFixedJointAnchor1TranslationCommand("Set 2D Fixed Joint Anchor 1 Translation", Vector3<f32>) where fn swap(self, physics, FixedJoint, fixed) {
        std::mem::swap(&mut fixed.local_anchor1_translation, &mut self.value);
    }

    SetFixedJointAnchor2TranslationCommand("Set 2D Fixed Joint Anchor 2 Translation", Vector3<f32>) where fn swap(self, physics, FixedJoint, fixed) {
        std::mem::swap(&mut fixed.local_anchor2_translation, &mut self.value);
    }

    SetFixedJointAnchor1RotationCommand("Set 2D Fixed Joint Anchor 1 Rotation", UnitQuaternion<f32>) where fn swap(self, node, FixedJoint, fixed) {
        std::mem::swap(&mut fixed.local_anchor1_rotation, &mut self.value);
    }

    SetFixedJointAnchor2RotationCommand("Set 2D Fixed Joint Anchor 2 Rotation", UnitQuaternion<f32>) where fn swap(self, node, FixedJoint, fixed) {
        std::mem::swap(&mut fixed.local_anchor2_rotation, &mut self.value);
    }

    SetPrismaticJointAnchor1Command("Set 2D Prismatic Joint Anchor 1", Vector3<f32>) where fn swap(self, node, PrismaticJoint, prismatic) {
        std::mem::swap(&mut prismatic.local_anchor1, &mut self.value);
    }

    SetPrismaticJointAxis1Command("Set 2D Prismatic Joint Axis 1", Vector3<f32>) where fn swap(self, node, PrismaticJoint, prismatic) {
        std::mem::swap(&mut prismatic.local_axis1, &mut self.value);
    }

    SetPrismaticJointAnchor2Command("Set 2D Prismatic Joint Anchor 2", Vector3<f32>) where fn swap(self, node, PrismaticJoint, prismatic) {
        std::mem::swap(&mut prismatic.local_anchor2, &mut self.value);
    }

    SetPrismaticJointAxis2Command("Set 2D Prismatic Joint Axis 2", Vector3<f32>) where fn swap(self, node, PrismaticJoint, prismatic) {
        std::mem::swap(&mut prismatic.local_axis2, &mut self.value);
    }
}

define_swap_command! {
    Node::as_joint2d_mut,
    SetJointBody1Command(Handle<Node>): body1, set_body1, "Set 2D Joint Body 1";
    SetJointBody2Command(Handle<Node>): body2, set_body2, "Set 2D Joint Body 2";
}
