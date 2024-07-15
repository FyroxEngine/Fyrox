use crate::command::{Command, CommandGroup};
use crate::fyrox::graph::{BaseSceneGraph, SceneGraph};
use crate::fyrox::{
    core::{
        algebra::{UnitQuaternion, Vector3},
        log::Log,
        math::Matrix4Ext,
        pool::Handle,
        reflect::prelude::*,
    },
    gui::{
        button::{ButtonBuilder, ButtonMessage},
        grid::{Column, GridBuilder, Row},
        inspector::{InspectorBuilder, InspectorContext, InspectorMessage, PropertyAction},
        message::{MessageDirection, UiMessage},
        scroll_viewer::ScrollViewerBuilder,
        stack_panel::StackPanelBuilder,
        utils::make_simple_tooltip,
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowMessage, WindowTitle},
        BuildContext, HorizontalAlignment, Orientation, Thickness, UiNode, UserInterface,
    },
    scene::{
        base::BaseBuilder,
        collider::{ColliderBuilder, ColliderShape, InteractionGroups},
        graph::Graph,
        joint::{BallJoint, JointBuilder, JointParams, RevoluteJoint},
        node::Node,
        ragdoll::{Limb, RagdollBuilder},
        rigidbody::{RigidBodyBuilder, RigidBodyType},
        transform::TransformBuilder,
    },
};
use crate::{
    inspector::editors::make_property_editors_container,
    message::MessageSender,
    scene::{
        commands::{graph::AddModelCommand, ChangeSelectionCommand},
        GameScene, Selection,
    },
    world::graph::selection::GraphSelection,
    MSG_SYNC_FLAG,
};
use std::ops::Range;
use std::sync::Arc;

#[derive(Reflect, Debug)]
pub struct RagdollPreset {
    #[reflect(description = "A handle of a hips (pelvis) bone.")]
    hips: Handle<Node>,
    #[reflect(description = "A handle of a left upper leg (thigh) bone.")]
    left_up_leg: Handle<Node>,
    #[reflect(description = "A handle of a left leg bone.")]
    left_leg: Handle<Node>,
    #[reflect(description = "A handle of a left foot bone.")]
    left_foot: Handle<Node>,
    #[reflect(description = "A handle of a right upper leg (thigh) bone.")]
    right_up_leg: Handle<Node>,
    #[reflect(description = "A handle of a right leg bone.")]
    right_leg: Handle<Node>,
    #[reflect(description = "A handle of a right foot bone.")]
    right_foot: Handle<Node>,
    #[reflect(description = "A handle of a lower spine bone.")]
    spine: Handle<Node>,
    #[reflect(description = "A handle of a middle spine bone.")]
    spine1: Handle<Node>,
    #[reflect(description = "A handle of a upper spine bone.")]
    spine2: Handle<Node>,
    #[reflect(description = "A handle of a left shoulder bone.")]
    left_shoulder: Handle<Node>,
    #[reflect(description = "A handle of a left arm bone.")]
    left_arm: Handle<Node>,
    #[reflect(description = "A handle of a left fore arm bone.")]
    left_fore_arm: Handle<Node>,
    #[reflect(description = "A handle of a left hand bone.")]
    left_hand: Handle<Node>,
    #[reflect(description = "A handle of a right shoulder bone.")]
    right_shoulder: Handle<Node>,
    #[reflect(description = "A handle of a right arm bone.")]
    right_arm: Handle<Node>,
    #[reflect(description = "A handle of a right fore arm bone.")]
    right_fore_arm: Handle<Node>,
    #[reflect(description = "A handle of a right hand bone.")]
    right_hand: Handle<Node>,
    #[reflect(description = "A handle of a neck bone.")]
    neck: Handle<Node>,
    #[reflect(description = "A handle of a head bone.")]
    head: Handle<Node>,
    #[reflect(
        description = "Total mass of the rag doll. Masses of each body part will be calculated using average \
    human body weight proportions."
    )]
    total_mass: f32,
    #[reflect(
        description = "Friction coefficient of every collider of every body part of the rag doll.",
        min_value = 0.0,
        max_value = 1.0
    )]
    friction: f32,
    #[reflect(
        description = "A flag, that defines whether the rigid bodies of the ragdoll will use continuous \
    collision detection or not. This should be turned on, if your rag doll relatively small bones since they'll \
    most likely fall through floor without CCD."
    )]
    use_ccd: bool,
    #[reflect(
        description = "A flag, that defines whether the rigid bodies of the rag doll can sleep or not. \
    Sleeping rigid bodies won't consume any CPU resources while remain static."
    )]
    can_sleep: bool,
    #[reflect(
        description = "A pair of bit masks, that defines collision group and filter for every collider in the \
    rag doll. It could be used to filter out collisions between character capsule and any part of the rag doll."
    )]
    collision_groups: InteractionGroups,
    #[reflect(
        description = "A pair of bit masks, that defines solver group and filter for every collider in the \
    rag doll. It could be used to filter out interactions between character capsule and any part of the rag doll."
    )]
    solver_groups: InteractionGroups,
}

impl Default for RagdollPreset {
    fn default() -> Self {
        Self {
            hips: Default::default(),
            left_up_leg: Default::default(),
            left_leg: Default::default(),
            left_foot: Default::default(),
            right_up_leg: Default::default(),
            right_leg: Default::default(),
            right_foot: Default::default(),
            spine: Default::default(),
            spine1: Default::default(),
            spine2: Default::default(),
            left_shoulder: Default::default(),
            left_arm: Default::default(),
            left_fore_arm: Default::default(),
            left_hand: Default::default(),
            right_shoulder: Default::default(),
            right_arm: Default::default(),
            right_fore_arm: Default::default(),
            right_hand: Default::default(),
            neck: Default::default(),
            head: Default::default(),
            total_mass: 70.0,
            friction: 0.5,
            use_ccd: true,
            can_sleep: true,
            collision_groups: Default::default(),
            solver_groups: Default::default(),
        }
    }
}

#[allow(dead_code)]
enum AxisOffset {
    None,
    X(f32),
    Y(f32),
    Z(f32),
}

struct BallJointLimits {
    x: Range<f32>,
    y: Range<f32>,
    z: Range<f32>,
}

fn try_make_ball_joint(
    body1: Handle<Node>,
    body2: Handle<Node>,
    name: &str,
    limits: Option<BallJointLimits>,
    offset_radius: AxisOffset,
    ragdoll: Handle<Node>,
    graph: &mut Graph,
) -> Handle<Node> {
    if body1.is_some() && body2.is_some() {
        let mut joint = BallJoint::default();

        if let Some(limits) = limits {
            joint.x_limits_enabled = true;
            joint.y_limits_enabled = true;
            joint.z_limits_enabled = true;

            joint.x_limits_angles = limits.x;
            joint.y_limits_angles = limits.y;
            joint.z_limits_angles = limits.z;
        }

        let body1_ref = &graph[body1];

        let offset = match offset_radius {
            AxisOffset::None => Default::default(),
            AxisOffset::X(offset) => body1_ref
                .side_vector()
                .try_normalize(f32::EPSILON)
                .unwrap_or_default()
                .scale(offset),
            AxisOffset::Y(offset) => body1_ref
                .up_vector()
                .try_normalize(f32::EPSILON)
                .unwrap_or_default()
                .scale(offset),
            AxisOffset::Z(offset) => body1_ref
                .look_vector()
                .try_normalize(f32::EPSILON)
                .unwrap_or_default()
                .scale(offset),
        };

        let ball_joint = JointBuilder::new(
            BaseBuilder::new().with_name(name).with_local_transform(
                TransformBuilder::new()
                    .with_local_position(body1_ref.global_position() - offset)
                    .with_local_rotation(UnitQuaternion::from_matrix_eps(
                        &graph[body1].global_transform().basis(),
                        f32::EPSILON,
                        16,
                        Default::default(),
                    ))
                    .build(),
            ),
        )
        .with_params(JointParams::BallJoint(joint))
        .with_body1(body1)
        .with_body2(body2)
        .with_auto_rebinding_enabled(false)
        .with_contacts_enabled(false)
        .build(graph);

        graph.link_nodes(ball_joint, ragdoll);

        ball_joint
    } else {
        Default::default()
    }
}

fn try_make_hinge_joint(
    body1: Handle<Node>,
    body2: Handle<Node>,
    name: &str,
    limits: Option<Range<f32>>,
    ragdoll: Handle<Node>,
    graph: &mut Graph,
) -> Handle<Node> {
    if body1.is_some() && body2.is_some() {
        let mut joint = RevoluteJoint::default();

        if let Some(limits) = limits {
            joint.limits_enabled = true;
            joint.limits = limits;
        }

        let hinge_joint = JointBuilder::new(
            BaseBuilder::new().with_name(name).with_local_transform(
                TransformBuilder::new()
                    .with_local_position(graph[body1].global_position())
                    .with_local_rotation(UnitQuaternion::from_matrix_eps(
                        &graph[body1].global_transform().basis(),
                        f32::EPSILON,
                        16,
                        Default::default(),
                    ))
                    .build(),
            ),
        )
        .with_params(JointParams::RevoluteJoint(joint))
        .with_body1(body1)
        .with_body2(body2)
        .with_auto_rebinding_enabled(false)
        .with_contacts_enabled(false)
        .build(graph);

        graph.link_nodes(hinge_joint, ragdoll);

        hinge_joint
    } else {
        Default::default()
    }
}

impl RagdollPreset {
    fn make_sphere(
        &self,
        from: Handle<Node>,
        radius: f32,
        mass: f32,
        name: &str,
        ragdoll: Handle<Node>,
        apply_offset: bool,
        graph: &mut Graph,
    ) -> Handle<Node> {
        if let Some(from_ref) = graph.try_get(from) {
            let offset = if apply_offset {
                from_ref
                    .up_vector()
                    .try_normalize(f32::EPSILON)
                    .unwrap_or_default()
                    .scale(radius)
            } else {
                Default::default()
            };

            let sphere = RigidBodyBuilder::new(
                BaseBuilder::new()
                    .with_name(name)
                    .with_local_transform(
                        TransformBuilder::new()
                            .with_local_position(from_ref.global_position() + offset)
                            .with_local_rotation(UnitQuaternion::from_matrix_eps(
                                &from_ref.global_transform().basis(),
                                f32::EPSILON,
                                16,
                                Default::default(),
                            ))
                            .build(),
                    )
                    .with_children(&[ColliderBuilder::new(
                        BaseBuilder::new().with_name("SphereCollider"),
                    )
                    .with_collision_groups(self.collision_groups)
                    .with_solver_groups(self.solver_groups)
                    .with_friction(self.friction)
                    .with_shape(ColliderShape::ball(radius))
                    .build(graph)]),
            )
            .with_mass(mass)
            .with_can_sleep(self.can_sleep)
            .with_ccd_enabled(self.use_ccd)
            .with_body_type(RigidBodyType::KinematicPositionBased)
            .build(graph);

            graph.link_nodes(sphere, ragdoll);

            sphere
        } else {
            Default::default()
        }
    }

    fn make_oriented_capsule(
        &self,
        from: Handle<Node>,
        to: Handle<Node>,
        radius: f32,
        mass: f32,
        name: &str,
        ragdoll: Handle<Node>,
        graph: &mut Graph,
    ) -> Handle<Node> {
        if let (Some(from_ref), Some(to_ref)) = (graph.try_get(from), graph.try_get(to)) {
            let pos_from = from_ref.global_position();
            let pos_to = to_ref.global_position();

            let capsule = RigidBodyBuilder::new(
                BaseBuilder::new()
                    .with_name(name)
                    .with_local_transform(
                        TransformBuilder::new()
                            .with_local_position(pos_from)
                            .with_local_rotation(UnitQuaternion::from_matrix_eps(
                                &from_ref.global_transform().basis(),
                                f32::EPSILON,
                                16,
                                Default::default(),
                            ))
                            .build(),
                    )
                    .with_children(&[ColliderBuilder::new(
                        BaseBuilder::new().with_name("CapsuleCollider"),
                    )
                    .with_shape(ColliderShape::capsule(
                        Vector3::default(),
                        Vector3::new(0.0, (pos_to - pos_from).norm() - 2.0 * radius, 0.0),
                        radius,
                    ))
                    .with_collision_groups(self.collision_groups)
                    .with_solver_groups(self.solver_groups)
                    .with_friction(self.friction)
                    .build(graph)]),
            )
            .with_mass(mass)
            .with_can_sleep(self.can_sleep)
            .with_ccd_enabled(self.use_ccd)
            .with_body_type(RigidBodyType::KinematicPositionBased)
            .build(graph);

            graph.link_nodes(capsule, ragdoll);

            capsule
        } else {
            Default::default()
        }
    }

    fn make_cuboid(
        &self,
        from: Handle<Node>,
        half_size: Vector3<f32>,
        mass: f32,
        name: &str,
        ragdoll: Handle<Node>,
        graph: &mut Graph,
    ) -> Handle<Node> {
        if let Some(from_ref) = graph.try_get(from) {
            let cuboid = RigidBodyBuilder::new(
                BaseBuilder::new()
                    .with_name(name)
                    .with_local_transform(
                        TransformBuilder::new()
                            .with_local_position(from_ref.global_position())
                            .build(),
                    )
                    .with_children(&[ColliderBuilder::new(
                        BaseBuilder::new().with_name("CuboidCollider"),
                    )
                    .with_collision_groups(self.collision_groups)
                    .with_solver_groups(self.solver_groups)
                    .with_shape(ColliderShape::cuboid(half_size.x, half_size.y, half_size.z))
                    .with_friction(self.friction)
                    .build(graph)]),
            )
            .with_mass(mass)
            .with_can_sleep(self.can_sleep)
            .with_ccd_enabled(self.use_ccd)
            .with_body_type(RigidBodyType::KinematicPositionBased)
            .build(graph);

            graph.link_nodes(cuboid, ragdoll);

            cuboid
        } else {
            Default::default()
        }
    }

    /// Calculates base size (size of the head) using common human body proportions. It uses distance between hand and elbow as a
    /// head size (it matches 1:1).
    fn measure_base_size(&self, graph: &Graph) -> f32 {
        let mut base_size = 0.2;
        for (upper, lower) in [
            (self.left_fore_arm, self.left_hand),
            (self.left_fore_arm, self.right_hand),
        ] {
            if let (Some(upper_ref), Some(lower_ref)) = (graph.try_get(upper), graph.try_get(lower))
            {
                base_size = (upper_ref.global_position() - lower_ref.global_position()).norm();
                break;
            }
        }
        base_size
    }

    pub fn create_and_send_command(
        &self,
        graph: &mut Graph,
        game_scene: &GameScene,
        sender: &MessageSender,
    ) {
        let base_size = self.measure_base_size(graph);
        let hand_radius = 0.3 * base_size;
        let head_radius = 0.5 * base_size;
        let foot_radius = 0.2 * base_size;

        let head_mass = 0.0823 * self.total_mass;
        let thorax_mass = 0.1856 * self.total_mass;
        let abdomen_mass = 0.1265 * self.total_mass;
        let pelvis_mass = 0.1481 * self.total_mass;
        let upper_arm_mass = 0.03075 * self.total_mass / 2.0;
        let fore_arm_mass = 0.0172 * self.total_mass / 2.0;
        let hand_mass = 0.00575 * self.total_mass / 2.0;
        let thigh_mass = 0.11125 * self.total_mass / 2.0;
        let leg_mass = 0.0505 * self.total_mass / 2.0;
        let foot_mass = 0.0138 * self.total_mass / 2.0;

        let ragdoll = RagdollBuilder::new(BaseBuilder::new().with_name("Ragdoll"))
            .with_active(true)
            .build(graph);

        graph.link_nodes(ragdoll, game_scene.scene_content_root);

        let left_up_leg = self.make_oriented_capsule(
            self.left_up_leg,
            self.left_leg,
            0.35 * base_size,
            thigh_mass,
            "RagdollLeftUpLeg",
            ragdoll,
            graph,
        );

        let left_leg = self.make_oriented_capsule(
            self.left_leg,
            self.left_foot,
            0.3 * base_size,
            leg_mass,
            "RagdollLeftLeg",
            ragdoll,
            graph,
        );

        let left_foot = self.make_sphere(
            self.left_foot,
            0.2 * base_size,
            foot_mass,
            "RagdollLeftFoot",
            ragdoll,
            false,
            graph,
        );

        let right_up_leg = self.make_oriented_capsule(
            self.right_up_leg,
            self.right_leg,
            0.35 * base_size,
            thigh_mass,
            "RagdollRightUpLeg",
            ragdoll,
            graph,
        );

        let right_leg = self.make_oriented_capsule(
            self.right_leg,
            self.right_foot,
            0.3 * base_size,
            leg_mass,
            "RagdollRightLeg",
            ragdoll,
            graph,
        );

        let right_foot = self.make_sphere(
            self.right_foot,
            foot_radius,
            foot_mass,
            "RagdollRightFoot",
            ragdoll,
            false,
            graph,
        );

        let hips = self.make_cuboid(
            self.hips,
            Vector3::new(base_size * 0.5, base_size * 0.2, base_size * 0.4),
            pelvis_mass,
            "RagdollHips",
            ragdoll,
            graph,
        );

        let spine = self.make_cuboid(
            self.spine,
            Vector3::new(base_size * 0.45, base_size * 0.2, base_size * 0.4),
            abdomen_mass,
            "RagdollSpine",
            ragdoll,
            graph,
        );

        let spine1 = self.make_cuboid(
            self.spine1,
            Vector3::new(base_size * 0.45, base_size * 0.2, base_size * 0.4),
            thorax_mass / 2.0,
            "RagdollSpine1",
            ragdoll,
            graph,
        );

        let spine2 = self.make_cuboid(
            self.spine2,
            Vector3::new(base_size * 0.45, base_size * 0.2, base_size * 0.4),
            thorax_mass / 2.0,
            "RagdollSpine2",
            ragdoll,
            graph,
        );

        // Left arm.
        let left_shoulder = self.make_oriented_capsule(
            self.left_shoulder,
            self.left_arm,
            0.2 * base_size,
            upper_arm_mass / 2.0,
            "RagdollLeftShoulder",
            ragdoll,
            graph,
        );

        let left_arm = self.make_oriented_capsule(
            self.left_arm,
            self.left_fore_arm,
            0.2 * base_size,
            upper_arm_mass / 2.0,
            "RagdollLeftArm",
            ragdoll,
            graph,
        );

        let left_fore_arm = self.make_oriented_capsule(
            self.left_fore_arm,
            self.left_hand,
            0.2 * base_size,
            fore_arm_mass,
            "RagdollLeftForeArm",
            ragdoll,
            graph,
        );

        let left_hand = self.make_sphere(
            self.left_hand,
            hand_radius,
            hand_mass,
            "LeftHand",
            ragdoll,
            false,
            graph,
        );

        // Right arm.
        let right_shoulder = self.make_oriented_capsule(
            self.right_shoulder,
            self.right_arm,
            0.2 * base_size,
            upper_arm_mass / 2.0,
            "RagdollRightShoulder",
            ragdoll,
            graph,
        );

        let right_arm = self.make_oriented_capsule(
            self.right_arm,
            self.right_fore_arm,
            0.2 * base_size,
            upper_arm_mass / 2.0,
            "RagdollRightArm",
            ragdoll,
            graph,
        );

        let right_fore_arm = self.make_oriented_capsule(
            self.right_fore_arm,
            self.right_hand,
            0.2 * base_size,
            fore_arm_mass,
            "RagdollRightForeArm",
            ragdoll,
            graph,
        );

        let right_hand = self.make_sphere(
            self.right_hand,
            hand_radius,
            hand_mass,
            "RightHand",
            ragdoll,
            false,
            graph,
        );

        let neck = self.make_oriented_capsule(
            self.neck,
            self.head,
            0.2 * base_size,
            0.3 * head_mass,
            "RagdollNeck",
            ragdoll,
            graph,
        );

        let head = self.make_sphere(
            self.head,
            0.5 * base_size,
            0.7 * head_mass,
            "RadgollHead",
            ragdoll,
            true,
            graph,
        );

        // Link limbs with joints.
        graph.update_hierarchical_data();

        // Left leg.
        try_make_ball_joint(
            left_up_leg,
            hips,
            "RagdollLeftUpLegHipsBallJoint",
            Some(BallJointLimits {
                x: -80.0f32.to_radians()..80.0f32.to_radians(),
                y: -80.0f32.to_radians()..80.0f32.to_radians(),
                z: -80.0f32.to_radians()..80.0f32.to_radians(),
            }),
            AxisOffset::None,
            ragdoll,
            graph,
        );
        try_make_hinge_joint(
            left_leg,
            left_up_leg,
            "RagdollLeftLegLeftUpLegHingeJoint",
            None,
            ragdoll,
            graph,
        );
        try_make_ball_joint(
            left_foot,
            left_leg,
            "RagdollLeftFootLeftLegBallJoint",
            Some(BallJointLimits {
                x: -45.0f32.to_radians()..45.0f32.to_radians(),
                y: -45.0f32.to_radians()..45.0f32.to_radians(),
                z: -45.0f32.to_radians()..45.0f32.to_radians(),
            }),
            AxisOffset::Y(-foot_radius),
            ragdoll,
            graph,
        );

        // Right leg.
        try_make_ball_joint(
            right_up_leg,
            hips,
            "RagdollLeftUpLegHipsBallJoint",
            Some(BallJointLimits {
                x: -80.0f32.to_radians()..80.0f32.to_radians(),
                y: -80.0f32.to_radians()..80.0f32.to_radians(),
                z: -80.0f32.to_radians()..80.0f32.to_radians(),
            }),
            AxisOffset::None,
            ragdoll,
            graph,
        );
        try_make_hinge_joint(
            right_leg,
            right_up_leg,
            "RagdollRightLegRightUpLegHingeJoint",
            None,
            ragdoll,
            graph,
        );
        try_make_ball_joint(
            right_foot,
            right_leg,
            "RagdollRightFootRightLegBallJoint",
            Some(BallJointLimits {
                x: -45.0f32.to_radians()..45.0f32.to_radians(),
                y: -45.0f32.to_radians()..45.0f32.to_radians(),
                z: -45.0f32.to_radians()..45.0f32.to_radians(),
            }),
            AxisOffset::Y(-foot_radius),
            ragdoll,
            graph,
        );

        try_make_hinge_joint(
            spine,
            hips,
            "RagdollSpineHipsHingeJoint",
            None,
            ragdoll,
            graph,
        );

        try_make_hinge_joint(
            spine1,
            spine,
            "RagdollSpine1SpineHingeJoint",
            None,
            ragdoll,
            graph,
        );

        try_make_hinge_joint(
            spine2,
            spine1,
            "RagdollSpine2Spine1HingeJoint",
            None,
            ragdoll,
            graph,
        );

        try_make_hinge_joint(
            left_shoulder,
            spine2,
            "RagdollSpine2LeftShoulderBallJoint",
            None,
            ragdoll,
            graph,
        );
        try_make_ball_joint(
            left_arm,
            left_shoulder,
            "RagdollLeftShoulderLeftArmBallJoint",
            None,
            AxisOffset::None,
            ragdoll,
            graph,
        );
        try_make_hinge_joint(
            left_fore_arm,
            left_arm,
            "RagdollLeftArmLeftForeArmBallJoint",
            None,
            ragdoll,
            graph,
        );
        try_make_ball_joint(
            left_hand,
            left_fore_arm,
            "RagdollLeftForeArmLeftHandBallJoint",
            Some(BallJointLimits {
                x: -45.0f32.to_radians()..45.0f32.to_radians(),
                y: -45.0f32.to_radians()..45.0f32.to_radians(),
                z: -45.0f32.to_radians()..45.0f32.to_radians(),
            }),
            AxisOffset::X(hand_radius),
            ragdoll,
            graph,
        );

        try_make_hinge_joint(
            right_shoulder,
            spine2,
            "RagdollSpine2RightShoulderBallJoint",
            None,
            ragdoll,
            graph,
        );
        try_make_ball_joint(
            right_arm,
            right_shoulder,
            "RagdollRightShoulderRightArmBallJoint",
            None,
            AxisOffset::None,
            ragdoll,
            graph,
        );
        try_make_hinge_joint(
            right_fore_arm,
            right_arm,
            "RagdollRightArmRightForeArmHingeJoint",
            None,
            ragdoll,
            graph,
        );
        try_make_ball_joint(
            right_hand,
            right_fore_arm,
            "RagdollRightForeArmRightHandBallJoint",
            Some(BallJointLimits {
                x: -45.0f32.to_radians()..45.0f32.to_radians(),
                y: -45.0f32.to_radians()..45.0f32.to_radians(),
                z: -45.0f32.to_radians()..45.0f32.to_radians(),
            }),
            AxisOffset::X(-hand_radius),
            ragdoll,
            graph,
        );

        try_make_ball_joint(
            neck,
            spine2,
            "RagdollNeckSpine2BallJoint",
            None,
            AxisOffset::None,
            ragdoll,
            graph,
        );
        try_make_ball_joint(
            head,
            neck,
            "RagdollHeadNeckBallJoint",
            None,
            AxisOffset::Y(head_radius),
            ragdoll,
            graph,
        );

        graph[ragdoll]
            .as_ragdoll_mut()
            .root_limb
            .set_value_and_mark_modified(Limb {
                bone: self.hips,
                physical_bone: hips,
                children: vec![
                    Limb {
                        bone: self.spine,
                        physical_bone: spine,
                        children: vec![Limb {
                            bone: self.spine1,
                            physical_bone: spine1,
                            children: vec![Limb {
                                bone: self.spine2,
                                physical_bone: spine2,
                                children: vec![
                                    Limb {
                                        bone: self.left_shoulder,
                                        physical_bone: left_shoulder,
                                        children: vec![Limb {
                                            bone: self.left_arm,
                                            physical_bone: left_arm,
                                            children: vec![Limb {
                                                bone: self.left_fore_arm,
                                                physical_bone: left_fore_arm,
                                                children: vec![Limb {
                                                    bone: self.left_hand,
                                                    physical_bone: left_hand,
                                                    children: vec![],
                                                }],
                                            }],
                                        }],
                                    },
                                    Limb {
                                        bone: self.right_shoulder,
                                        physical_bone: right_shoulder,
                                        children: vec![Limb {
                                            bone: self.right_arm,
                                            physical_bone: right_arm,
                                            children: vec![Limb {
                                                bone: self.right_fore_arm,
                                                physical_bone: right_fore_arm,
                                                children: vec![Limb {
                                                    bone: self.right_hand,
                                                    physical_bone: right_hand,
                                                    children: vec![],
                                                }],
                                            }],
                                        }],
                                    },
                                    Limb {
                                        bone: self.neck,
                                        physical_bone: neck,
                                        children: vec![Limb {
                                            bone: self.head,
                                            physical_bone: head,
                                            children: vec![],
                                        }],
                                    },
                                ],
                            }],
                        }],
                    },
                    Limb {
                        bone: self.left_up_leg,
                        physical_bone: left_up_leg,
                        children: vec![Limb {
                            bone: self.left_leg,
                            physical_bone: left_leg,
                            children: vec![Limb {
                                bone: self.left_foot,
                                physical_bone: left_foot,
                                children: vec![],
                            }],
                        }],
                    },
                    Limb {
                        bone: self.right_up_leg,
                        physical_bone: right_up_leg,
                        children: vec![Limb {
                            bone: self.right_leg,
                            physical_bone: right_leg,
                            children: vec![Limb {
                                bone: self.right_foot,
                                physical_bone: right_foot,
                                children: vec![],
                            }],
                        }],
                    },
                ],
            });

        // Immediately after extract if from the scene to subgraph. This is required to not violate
        // the rule of one place of execution, only commands allowed to modify the scene.
        let sub_graph = graph.take_reserve_sub_graph(ragdoll);

        let group = vec![
            Command::new(AddModelCommand::new(sub_graph)),
            // We also want to select newly instantiated model.
            Command::new(ChangeSelectionCommand::new(Selection::new(
                GraphSelection::single_or_empty(ragdoll),
            ))),
        ];

        sender.do_command(CommandGroup::from(group).with_custom_name("Generate Ragdoll"));
    }
}

pub struct RagdollWizard {
    pub window: Handle<UiNode>,
    pub preset: RagdollPreset,
    inspector: Handle<UiNode>,
    ok: Handle<UiNode>,
    cancel: Handle<UiNode>,
    autofill: Handle<UiNode>,
}

impl RagdollWizard {
    pub fn new(ctx: &mut BuildContext, sender: MessageSender) -> Self {
        let preset = RagdollPreset::default();
        let container = Arc::new(make_property_editors_container(sender));

        let inspector;
        let ok;
        let cancel;
        let autofill;
        let window = WindowBuilder::new(
            WidgetBuilder::new()
                .with_width(350.0)
                .with_height(550.0)
                .with_name("RagdollWizard"),
        )
        .open(false)
        .with_title(WindowTitle::text("Ragdoll Wizard"))
        .with_content(
            GridBuilder::new(
                WidgetBuilder::new()
                    .with_child(
                        ScrollViewerBuilder::new(
                            WidgetBuilder::new().with_margin(Thickness::uniform(1.0)),
                        )
                        .with_content({
                            inspector = InspectorBuilder::new(
                                WidgetBuilder::new().with_margin(Thickness::uniform(1.0)),
                            )
                            .with_context(InspectorContext::from_object(
                                &preset,
                                ctx,
                                container,
                                None,
                                MSG_SYNC_FLAG,
                                0,
                                true,
                                Default::default(),
                                150.0,
                            ))
                            .build(ctx);
                            inspector
                        })
                        .build(ctx),
                    )
                    .with_child(
                        StackPanelBuilder::new(
                            WidgetBuilder::new()
                                .with_horizontal_alignment(HorizontalAlignment::Right)
                                .on_row(1)
                                .with_margin(Thickness::uniform(1.0))
                                .with_child({
                                    autofill = ButtonBuilder::new(
                                        WidgetBuilder::new()
                                            .with_width(100.0)
                                            .with_margin(Thickness::uniform(1.0))
                                            .with_tooltip(make_simple_tooltip(
                                                ctx,
                                                "Tries to fill in bone handles of every body part \
                                                by using a fixed set of commonly used bone names. \
                                                Tested only on Mixamo skeletons.",
                                            )),
                                    )
                                    .with_text("Autofill")
                                    .build(ctx);
                                    autofill
                                })
                                .with_child({
                                    ok = ButtonBuilder::new(
                                        WidgetBuilder::new()
                                            .with_width(100.0)
                                            .with_margin(Thickness::uniform(1.0)),
                                    )
                                    .with_text("OK")
                                    .build(ctx);
                                    ok
                                })
                                .with_child({
                                    cancel = ButtonBuilder::new(
                                        WidgetBuilder::new()
                                            .with_width(100.0)
                                            .with_margin(Thickness::uniform(1.0)),
                                    )
                                    .with_text("Cancel")
                                    .build(ctx);
                                    cancel
                                }),
                        )
                        .with_orientation(Orientation::Horizontal)
                        .build(ctx),
                    ),
            )
            .add_row(Row::stretch())
            .add_row(Row::strict(24.0))
            .add_column(Column::stretch())
            .build(ctx),
        )
        .build(ctx);

        Self {
            window,
            preset,
            inspector,
            ok,
            cancel,
            autofill,
        }
    }

    pub fn open(&self, ui: &UserInterface) {
        ui.send_message(WindowMessage::open(
            self.window,
            MessageDirection::ToWidget,
            true,
            true,
        ));
    }

    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        ui: &mut UserInterface,
        graph: &mut Graph,
        game_scene: &GameScene,
        sender: &MessageSender,
    ) {
        if let Some(InspectorMessage::PropertyChanged(args)) = message.data() {
            if message.destination() == self.inspector
                && message.direction() == MessageDirection::FromWidget
            {
                PropertyAction::from_field_kind(&args.value).apply(
                    &args.path(),
                    &mut self.preset,
                    &mut |result| {
                        Log::verify(result);
                    },
                );
            }
        } else if let Some(ButtonMessage::Click) = message.data() {
            if message.destination() == self.ok {
                self.preset
                    .create_and_send_command(graph, game_scene, sender);

                ui.send_message(WindowMessage::close(
                    self.window,
                    MessageDirection::ToWidget,
                ));
            } else if message.destination() == self.cancel {
                ui.send_message(WindowMessage::close(
                    self.window,
                    MessageDirection::ToWidget,
                ));
            } else if message.destination() == self.autofill {
                fn find_by_pattern(graph: &Graph, pattern: &str) -> Handle<Node> {
                    graph
                        .find(graph.get_root(), &mut |n| n.name().contains(pattern))
                        .map(|(h, _)| h)
                        .unwrap_or_default()
                }

                self.preset.hips = find_by_pattern(graph, "Hips");

                self.preset.spine = find_by_pattern(graph, "Spine");
                self.preset.spine1 = find_by_pattern(graph, "Spine1");
                self.preset.spine2 = find_by_pattern(graph, "Spine2");

                self.preset.right_up_leg = find_by_pattern(graph, "RightUpLeg");
                self.preset.right_leg = find_by_pattern(graph, "RightLeg");
                self.preset.right_foot = find_by_pattern(graph, "RightFoot");

                self.preset.left_up_leg = find_by_pattern(graph, "LeftUpLeg");
                self.preset.left_leg = find_by_pattern(graph, "LeftLeg");
                self.preset.left_foot = find_by_pattern(graph, "LeftFoot");

                self.preset.right_hand = find_by_pattern(graph, "RightHand");
                self.preset.right_arm = find_by_pattern(graph, "RightArm");
                self.preset.right_fore_arm = find_by_pattern(graph, "RightForeArm");
                self.preset.right_shoulder = find_by_pattern(graph, "RightShoulder");

                self.preset.left_hand = find_by_pattern(graph, "LeftHand");
                self.preset.left_arm = find_by_pattern(graph, "LeftArm");
                self.preset.left_fore_arm = find_by_pattern(graph, "LeftForeArm");
                self.preset.left_shoulder = find_by_pattern(graph, "LeftShoulder");

                self.preset.neck = find_by_pattern(graph, "Neck");
                self.preset.head = find_by_pattern(graph, "Head");

                let ctx = ui
                    .node(self.inspector)
                    .cast::<fyrox::gui::inspector::Inspector>()
                    .unwrap()
                    .context()
                    .clone();

                if let Err(sync_errors) = ctx.sync(&self.preset, ui, 0, true, Default::default()) {
                    for error in sync_errors {
                        Log::err(format!("Failed to sync property. Reason: {:?}", error))
                    }
                }
            }
        }
    }
}
