use crate::{
    inspector::editors::make_property_editors_container,
    message::MessageSender,
    scene::{
        commands::{graph::AddModelCommand, ChangeSelectionCommand, CommandGroup, SceneCommand},
        EditorScene, Selection,
    },
    world::graph::selection::GraphSelection,
    MSG_SYNC_FLAG,
};
use fyrox::{
    core::{algebra::Vector3, log::Log, pool::Handle, reflect::prelude::*},
    gui::{
        button::{ButtonBuilder, ButtonMessage},
        grid::{Column, GridBuilder, Row},
        inspector::{InspectorBuilder, InspectorContext, InspectorMessage, PropertyAction},
        message::{MessageDirection, UiMessage},
        stack_panel::StackPanelBuilder,
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowMessage, WindowTitle},
        BuildContext, HorizontalAlignment, Orientation, Thickness, UiNode, UserInterface,
    },
    scene::{
        base::BaseBuilder,
        collider::{ColliderBuilder, ColliderShape},
        graph::Graph,
        joint::JointBuilder,
        node::Node,
        rigidbody::{RigidBodyBuilder, RigidBodyType},
        transform::TransformBuilder,
    },
};
use std::rc::Rc;

#[derive(Reflect, Debug)]
pub struct RagdollPreset {
    hips: Handle<Node>,
    left_up_leg: Handle<Node>,
    left_leg: Handle<Node>,
    left_foot: Handle<Node>,
    right_up_leg: Handle<Node>,
    right_leg: Handle<Node>,
    right_foot: Handle<Node>,
    spine: Handle<Node>,
    spine1: Handle<Node>,
    spine2: Handle<Node>,
    left_shoulder: Handle<Node>,
    left_arm: Handle<Node>,
    left_fore_arm: Handle<Node>,
    left_hand: Handle<Node>,
    right_shoulder: Handle<Node>,
    right_arm: Handle<Node>,
    right_fore_arm: Handle<Node>,
    right_hand: Handle<Node>,
    total_mass: f32,
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
            total_mass: 20.0,
        }
    }
}

fn make_oriented_capsule(
    from: Handle<Node>,
    to: Handle<Node>,
    radius: f32,
    name: &str,
    graph: &mut Graph,
) -> Handle<Node> {
    let pos_from = graph[from].global_position();
    let pos_to = graph[to].global_position();

    RigidBodyBuilder::new(
        BaseBuilder::new()
            .with_name(name)
            .with_children(&[ColliderBuilder::new(
                BaseBuilder::new()
                    .with_name("CapsuleCollider")
                    .with_local_transform(
                        TransformBuilder::new()
                            .with_local_position(pos_from)
                            .build(),
                    ),
            )
            .with_shape(ColliderShape::capsule(
                Vector3::default(),
                pos_to - pos_from,
                radius,
            ))
            .build(graph)]),
    )
    .with_body_type(RigidBodyType::KinematicPositionBased)
    .build(graph)
}

fn make_cuboid(
    from: Handle<Node>,
    half_size: Vector3<f32>,
    name: &str,
    graph: &mut Graph,
) -> Handle<Node> {
    RigidBodyBuilder::new(
        BaseBuilder::new()
            .with_name(name)
            .with_children(&[ColliderBuilder::new(
                BaseBuilder::new()
                    .with_name("CuboidCollider")
                    .with_local_transform(
                        TransformBuilder::new()
                            .with_local_position(graph[from].global_position())
                            .build(),
                    ),
            )
            .with_shape(ColliderShape::cuboid(half_size.x, half_size.y, half_size.z))
            .build(graph)]),
    )
    .with_body_type(RigidBodyType::KinematicPositionBased)
    .build(graph)
}

fn make_sphere(from: Handle<Node>, radius: f32, name: &str, graph: &mut Graph) -> Handle<Node> {
    RigidBodyBuilder::new(
        BaseBuilder::new()
            .with_name(name)
            .with_children(&[ColliderBuilder::new(
                BaseBuilder::new()
                    .with_name("SphereCollider")
                    .with_local_transform(
                        TransformBuilder::new()
                            .with_local_position(graph[from].global_position())
                            .build(),
                    ),
            )
            .with_shape(ColliderShape::ball(radius))
            .build(graph)]),
    )
    .with_body_type(RigidBodyType::KinematicPositionBased)
    .build(graph)
}

fn make_ball_joint(
    body1: Handle<Node>,
    body2: Handle<Node>,
    name: &str,
    graph: &mut Graph,
) -> Handle<Node> {
    JointBuilder::new(
        BaseBuilder::new().with_name(name).with_local_transform(
            TransformBuilder::new()
                .with_local_position(graph[body1].global_position())
                .build(),
        ),
    )
    .with_body1(body1)
    .with_body2(body2)
    .build(graph)
}

impl RagdollPreset {
    pub fn create_and_send_command(
        &self,
        graph: &mut Graph,
        editor_scene: &EditorScene,
        sender: &MessageSender,
    ) {
        let left_up_leg = if self.left_up_leg.is_some() && self.left_leg.is_some() {
            make_oriented_capsule(
                self.left_up_leg,
                self.left_leg,
                0.25,
                "RagdollLeftUpLeg",
                graph,
            )
        } else {
            Default::default()
        };

        let left_leg = if self.left_leg.is_some() && self.left_foot.is_some() {
            make_oriented_capsule(self.left_leg, self.left_foot, 0.25, "RagdollLeftLeg", graph)
        } else {
            Default::default()
        };

        let left_foot = if self.left_foot.is_some() {
            make_sphere(self.left_foot, 0.5, "RagdollLeftFoot", graph)
        } else {
            Default::default()
        };

        let right_up_leg = if self.right_up_leg.is_some() && self.right_leg.is_some() {
            make_oriented_capsule(
                self.right_up_leg,
                self.right_leg,
                0.25,
                "RagdollLeftUpLeg",
                graph,
            )
        } else {
            Default::default()
        };

        let right_leg = if self.right_leg.is_some() && self.right_foot.is_some() {
            make_oriented_capsule(
                self.right_leg,
                self.right_foot,
                0.25,
                "RagdollLeftLeg",
                graph,
            )
        } else {
            Default::default()
        };

        let right_foot = if self.right_foot.is_some() {
            make_sphere(self.right_foot, 0.5, "RagdollRightFoot", graph)
        } else {
            Default::default()
        };

        let hips = if self.hips.is_some() {
            make_cuboid(self.hips, Vector3::new(0.5, 0.5, 0.5), "RagdollHips", graph)
        } else {
            Default::default()
        };

        if left_up_leg.is_some() && hips.is_some() {
            make_ball_joint(left_up_leg, hips, "RagdollLeftUpLegHipsBallJoint", graph);
        }

        if left_leg.is_some() && left_up_leg.is_some() {
            make_ball_joint(
                left_leg,
                left_up_leg,
                "RagdollLeftLegLeftUpLegBallJoint",
                graph,
            );
        }

        if left_foot.is_some() && left_leg.is_some() {
            make_ball_joint(
                left_foot,
                left_leg,
                "RagdollLeftFootLeftLegBallJoint",
                graph,
            );
        }

        if right_up_leg.is_some() && hips.is_some() {
            make_ball_joint(right_up_leg, hips, "RagdollLeftUpLegHipsBallJoint", graph);
        }

        if right_leg.is_some() && right_up_leg.is_some() {
            make_ball_joint(
                right_leg,
                right_up_leg,
                "RagdollRightLegRightUpLegBallJoint",
                graph,
            );
        }

        if right_foot.is_some() && right_leg.is_some() {
            make_ball_joint(
                right_foot,
                right_leg,
                "RagdollRightFootRightLegBallJoint",
                graph,
            );
        }

        // Immediately after extract if from the scene to subgraph. This is required to not violate
        // the rule of one place of execution, only commands allowed to modify the scene.
        let sub_graph = graph.take_reserve_sub_graph(hips);

        let group = vec![
            SceneCommand::new(AddModelCommand::new(sub_graph)),
            // We also want to select newly instantiated model.
            SceneCommand::new(ChangeSelectionCommand::new(
                Selection::Graph(GraphSelection::single_or_empty(hips)),
                editor_scene.selection.clone(),
            )),
        ];

        sender.do_scene_command(CommandGroup::from(group));
    }
}

pub struct RagdollWizard {
    pub window: Handle<UiNode>,
    pub preset: RagdollPreset,
    inspector: Handle<UiNode>,
    ok: Handle<UiNode>,
    cancel: Handle<UiNode>,
}

impl RagdollWizard {
    pub fn new(ctx: &mut BuildContext, sender: MessageSender) -> Self {
        let preset = RagdollPreset::default();
        let container = Rc::new(make_property_editors_container(sender));

        let inspector;
        let ok;
        let cancel;
        let window = WindowBuilder::new(
            WidgetBuilder::new()
                .with_width(350.0)
                .with_height(500.0)
                .with_name("RagdollWizard"),
        )
        .open(false)
        .with_title(WindowTitle::text("Ragdoll Wizard"))
        .with_content(
            GridBuilder::new(
                WidgetBuilder::new()
                    .with_child({
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
                        ))
                        .build(ctx);
                        inspector
                    })
                    .with_child(
                        StackPanelBuilder::new(
                            WidgetBuilder::new()
                                .with_horizontal_alignment(HorizontalAlignment::Right)
                                .on_row(1)
                                .with_margin(Thickness::uniform(1.0))
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
        }
    }

    pub fn open(&self, ui: &UserInterface) {
        ui.send_message(WindowMessage::open(
            self.window,
            MessageDirection::ToWidget,
            true,
        ));
    }

    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        ui: &UserInterface,
        graph: &mut Graph,
        editor_scene: &EditorScene,
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
                    .create_and_send_command(graph, editor_scene, sender);

                ui.send_message(WindowMessage::close(
                    self.window,
                    MessageDirection::ToWidget,
                ));
            } else if message.destination() == self.cancel {
                ui.send_message(WindowMessage::close(
                    self.window,
                    MessageDirection::ToWidget,
                ));
            }
        }
    }
}
