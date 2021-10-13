use crate::gui::make_dropdown_list_option;
use crate::scene::commands::SceneCommand;
use crate::sidebar::make_section;
use crate::{
    physics::{Collider, Joint, RigidBody},
    scene::commands::CommandGroup,
    scene::{
        commands::physics::{
            AddColliderCommand, AddJointCommand, DeleteBodyCommand, DeleteColliderCommand,
            DeleteJointCommand, SetBallRadiusCommand, SetBodyCommand, SetColliderPositionCommand,
            SetCuboidHalfExtentsCommand, SetCylinderHalfHeightCommand, SetCylinderRadiusCommand,
        },
        EditorScene, Selection,
    },
    send_sync_message,
    sidebar::{
        make_text_mark,
        physics::{
            ball::BallSection, body::BodySection, capsule::CapsuleSection,
            collider::ColliderSection, cone::ConeSection, cuboid::CuboidSection,
            cylinder::CylinderSection, joint::JointSection,
        },
        COLUMN_WIDTH, ROW_HEIGHT,
    },
    GameEngine, Message,
};
use rg3d::gui::message::UiMessage;
use rg3d::gui::{BuildContext, UiNode, UserInterface};
use rg3d::{
    core::{
        algebra::Matrix4, algebra::Vector3, math::aabb::AxisAlignedBoundingBox, pool::Handle,
        scope_profile,
    },
    gui::{
        button::ButtonBuilder,
        dropdown_list::DropdownListBuilder,
        grid::{Column, GridBuilder, Row},
        message::{
            ButtonMessage, DropdownListMessage, MessageDirection, UiMessageData, WidgetMessage,
        },
        stack_panel::StackPanelBuilder,
        widget::WidgetBuilder,
        Orientation, Thickness,
    },
    physics3d::desc::{
        BallDesc, BallJointDesc, CapsuleDesc, ColliderShapeDesc, ConeDesc, CuboidDesc,
        CylinderDesc, FixedJointDesc, HeightfieldDesc, JointParamsDesc, PrismaticJointDesc,
        RevoluteJointDesc, RigidBodyTypeDesc, RoundCylinderDesc, SegmentDesc, TriangleDesc,
        TrimeshDesc,
    },
    scene::{graph::Graph, node::Node},
};
use std::sync::mpsc::Sender;

mod ball;
mod body;
mod capsule;
mod collider;
mod cone;
mod cuboid;
mod cylinder;
mod joint;
mod segment;
mod triangle;
mod trimesh;

pub struct PhysicsSection {
    pub section: Handle<UiNode>,
    body: Handle<UiNode>,
    collider: Handle<UiNode>,
    collider_text: Handle<UiNode>,
    joint: Handle<UiNode>,
    joint_text: Handle<UiNode>,
    fit: Handle<UiNode>,
    sender: Sender<Message>,
    pub body_section: BodySection,
    pub collider_section: ColliderSection,
    pub cylinder_section: CylinderSection,
    pub cone_section: ConeSection,
    pub cuboid_section: CuboidSection,
    pub capsule_section: CapsuleSection,
    pub ball_section: BallSection,
    pub joint_section: JointSection,
}

impl PhysicsSection {
    pub fn new(ctx: &mut BuildContext, sender: Sender<Message>) -> Self {
        let body;
        let collider;
        let collider_text;
        let joint;
        let joint_text;
        let fit;
        let body_section = BodySection::new(ctx, sender.clone());
        let collider_section = ColliderSection::new(ctx, sender.clone());
        let cylinder_section = CylinderSection::new(ctx, sender.clone());
        let cone_section = ConeSection::new(ctx, sender.clone());
        let cuboid_section = CuboidSection::new(ctx, sender.clone());
        let capsule_section = CapsuleSection::new(ctx, sender.clone());
        let ball_section = BallSection::new(ctx, sender.clone());
        let joint_section = JointSection::new(ctx, sender.clone());
        let section = make_section(
            "Physics Properties",
            StackPanelBuilder::new(
                WidgetBuilder::new()
                    .with_child(
                        GridBuilder::new(
                            WidgetBuilder::new()
                                .with_child(make_text_mark(ctx, "Body", 0))
                                .with_child({
                                    body = DropdownListBuilder::new(
                                        WidgetBuilder::new()
                                            .on_row(0)
                                            .on_column(1)
                                            .with_margin(Thickness::uniform(1.0)),
                                    )
                                    .with_close_on_selection(true)
                                    .with_items(vec![
                                        make_dropdown_list_option(ctx, "None"),
                                        make_dropdown_list_option(ctx, "Dynamic"),
                                        make_dropdown_list_option(ctx, "Static"),
                                        make_dropdown_list_option(ctx, "KinematicPositionBased"),
                                        make_dropdown_list_option(ctx, "KinematicVelocityBased"),
                                    ])
                                    .build(ctx);
                                    body
                                })
                                .with_child({
                                    collider_text = make_text_mark(ctx, "Collider", 1);
                                    collider_text
                                })
                                .with_child({
                                    collider = DropdownListBuilder::new(
                                        WidgetBuilder::new()
                                            .on_row(1)
                                            .on_column(1)
                                            .with_margin(Thickness::uniform(1.0)),
                                    )
                                    .with_close_on_selection(true)
                                    .with_items(vec![
                                        make_dropdown_list_option(ctx, "Ball"),
                                        make_dropdown_list_option(ctx, "Cylinder"),
                                        make_dropdown_list_option(ctx, "Round Cylinder"),
                                        make_dropdown_list_option(ctx, "Cone"),
                                        make_dropdown_list_option(ctx, "Cuboid"),
                                        make_dropdown_list_option(ctx, "Capsule"),
                                        make_dropdown_list_option(ctx, "Segment"),
                                        make_dropdown_list_option(ctx, "Triangle"),
                                        make_dropdown_list_option(ctx, "Trimesh"),
                                        make_dropdown_list_option(ctx, "Heightfield"),
                                    ])
                                    .build(ctx);
                                    collider
                                })
                                .with_child({
                                    joint_text = make_text_mark(ctx, "Joint", 2);
                                    joint_text
                                })
                                .with_child({
                                    joint = DropdownListBuilder::new(
                                        WidgetBuilder::new()
                                            .on_row(2)
                                            .on_column(1)
                                            .with_margin(Thickness::uniform(1.0)),
                                    )
                                    .with_close_on_selection(true)
                                    .with_items(vec![
                                        make_dropdown_list_option(ctx, "None"),
                                        make_dropdown_list_option(ctx, "Ball Joint"),
                                        make_dropdown_list_option(ctx, "Fixed Joint"),
                                        make_dropdown_list_option(ctx, "Prismatic Joint"),
                                        make_dropdown_list_option(ctx, "Revolute Joint"),
                                    ])
                                    .build(ctx);
                                    joint
                                })
                                .with_child({
                                    fit = ButtonBuilder::new(
                                        WidgetBuilder::new()
                                            .with_margin(Thickness::uniform(1.0))
                                            .on_row(3)
                                            .on_column(1),
                                    )
                                    .with_text("Fit Collider")
                                    .build(ctx);
                                    fit
                                }),
                        )
                        .add_column(Column::strict(COLUMN_WIDTH))
                        .add_column(Column::stretch())
                        .add_row(Row::strict(ROW_HEIGHT))
                        .add_row(Row::strict(ROW_HEIGHT))
                        .add_row(Row::strict(ROW_HEIGHT))
                        .add_row(Row::strict(ROW_HEIGHT))
                        .build(ctx),
                    )
                    .with_children([
                        body_section.section,
                        collider_section.section,
                        cylinder_section.section,
                        cone_section.section,
                        cuboid_section.section,
                        capsule_section.section,
                        ball_section.section,
                        joint_section.section,
                    ]),
            )
            .with_orientation(Orientation::Vertical)
            .build(ctx),
            ctx,
        );

        Self {
            body_section,
            collider_section,
            cylinder_section,
            cone_section,
            cuboid_section,
            capsule_section,
            ball_section,
            section,
            body,
            collider,
            collider_text,
            sender,
            joint_section,
            joint,
            joint_text,
            fit,
        }
    }

    pub fn sync_to_model(&mut self, editor_scene: &EditorScene, engine: &mut GameEngine) {
        if let Selection::Graph(selection) = &editor_scene.selection {
            let scene = &engine.scenes[editor_scene.scene];

            if selection.is_single_selection() {
                let node_handle = selection.nodes()[0];
                if scene.graph.is_valid_handle(node_handle) {
                    let ui = &mut engine.user_interface;

                    // Sync physical body info.
                    let mut body_index = 0;
                    let mut joint = Handle::NONE;
                    if let Some(&body_handle) = editor_scene.physics.binder.value_of(&node_handle) {
                        let body = &editor_scene.physics.bodies[body_handle];
                        body_index = match body.status {
                            RigidBodyTypeDesc::Dynamic => 1,
                            RigidBodyTypeDesc::Static => 2,
                            RigidBodyTypeDesc::KinematicPositionBased => 3,
                            RigidBodyTypeDesc::KinematicVelocityBased => 4,
                        };
                        for (h, j) in editor_scene.physics.joints.pair_iter() {
                            if j.body1 == body_handle.into() {
                                joint = h;
                                break;
                            }
                        }
                    }

                    send_sync_message(
                        ui,
                        DropdownListMessage::selection(
                            self.body,
                            MessageDirection::ToWidget,
                            Some(body_index),
                        ),
                    );

                    fn toggle_visibility(
                        ui: &mut UserInterface,
                        destination: Handle<UiNode>,
                        value: bool,
                    ) {
                        send_sync_message(
                            ui,
                            WidgetMessage::visibility(
                                destination,
                                MessageDirection::ToWidget,
                                value,
                            ),
                        );
                    }

                    toggle_visibility(ui, self.collider, body_index != 0);
                    toggle_visibility(ui, self.collider_text, body_index != 0);
                    toggle_visibility(ui, self.joint_text, body_index != 0);
                    toggle_visibility(ui, self.joint, body_index != 0);
                    toggle_visibility(ui, self.joint_section.section, joint.is_some());
                    toggle_visibility(ui, self.collider_section.section, false);
                    toggle_visibility(ui, self.cylinder_section.section, false);
                    toggle_visibility(ui, self.cone_section.section, false);
                    toggle_visibility(ui, self.cuboid_section.section, false);
                    toggle_visibility(ui, self.capsule_section.section, false);
                    toggle_visibility(ui, self.ball_section.section, false);
                    toggle_visibility(ui, self.body_section.section, false);
                    toggle_visibility(ui, self.fit, false);

                    if joint.is_some() {
                        let joint = &editor_scene.physics.joints[joint];

                        self.joint_section.sync_to_model(
                            joint,
                            &scene.graph,
                            &editor_scene.physics.binder,
                            ui,
                        );

                        let joint_index = match joint.params {
                            JointParamsDesc::BallJoint(_) => 1,
                            JointParamsDesc::FixedJoint(_) => 2,
                            JointParamsDesc::PrismaticJoint(_) => 3,
                            JointParamsDesc::RevoluteJoint(_) => 4,
                        };
                        send_sync_message(
                            ui,
                            DropdownListMessage::selection(
                                self.joint,
                                MessageDirection::ToWidget,
                                Some(joint_index),
                            ),
                        );
                    } else {
                        send_sync_message(
                            ui,
                            DropdownListMessage::selection(
                                self.joint,
                                MessageDirection::ToWidget,
                                Some(0),
                            ),
                        );
                    }

                    if let Some(&body_handle) = editor_scene.physics.binder.value_of(&node_handle) {
                        let body = &editor_scene.physics.bodies[body_handle];

                        if let Some(&collider_handle) = body.colliders.first() {
                            let collider = &editor_scene.physics.colliders[collider_handle.into()];
                            toggle_visibility(ui, self.collider_section.section, true);
                            toggle_visibility(ui, self.fit, true);
                            self.collider_section.sync_to_model(collider, ui);
                        }

                        self.body_section.sync_to_model(body, ui);
                        toggle_visibility(ui, self.body_section.section, true);

                        if let Some(&collider) = body.colliders.get(0) {
                            let collider_index =
                                match &editor_scene.physics.colliders[collider.into()].shape {
                                    ColliderShapeDesc::Ball(ball) => {
                                        toggle_visibility(ui, self.ball_section.section, true);
                                        self.ball_section.sync_to_model(ball, ui);
                                        0
                                    }
                                    ColliderShapeDesc::Cylinder(cylinder) => {
                                        toggle_visibility(ui, self.cylinder_section.section, true);
                                        self.cylinder_section.sync_to_model(cylinder, ui);
                                        1
                                    }
                                    ColliderShapeDesc::RoundCylinder(_) => 2,
                                    ColliderShapeDesc::Cone(cone) => {
                                        toggle_visibility(ui, self.cone_section.section, true);
                                        self.cone_section.sync_to_model(cone, ui);
                                        3
                                    }
                                    ColliderShapeDesc::Cuboid(cuboid) => {
                                        toggle_visibility(ui, self.cuboid_section.section, true);
                                        self.cuboid_section.sync_to_model(cuboid, ui);
                                        4
                                    }
                                    ColliderShapeDesc::Capsule(capsule) => {
                                        toggle_visibility(ui, self.capsule_section.section, true);
                                        self.capsule_section.sync_to_model(capsule, ui);
                                        5
                                    }
                                    ColliderShapeDesc::Segment(_) => {
                                        // TODO
                                        6
                                    }
                                    ColliderShapeDesc::Triangle(_) => {
                                        // TODO
                                        7
                                    }
                                    ColliderShapeDesc::Trimesh(_) => {
                                        // Nothing to edit.
                                        8
                                    }
                                    ColliderShapeDesc::Heightfield(_) => {
                                        // Nothing to edit.
                                        9
                                    }
                                };
                            send_sync_message(
                                ui,
                                DropdownListMessage::selection(
                                    self.collider,
                                    MessageDirection::ToWidget,
                                    Some(collider_index),
                                ),
                            );
                        } else {
                            send_sync_message(
                                ui,
                                DropdownListMessage::selection(
                                    self.collider,
                                    MessageDirection::ToWidget,
                                    None,
                                ),
                            );
                        }
                    }
                }
            }
        }
    }

    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        editor_scene: &EditorScene,
        engine: &GameEngine,
    ) {
        scope_profile!();

        if let Selection::Graph(selection) = &editor_scene.selection {
            let scene = &engine.scenes[editor_scene.scene];
            let graph = &scene.graph;

            if selection.is_single_selection() {
                let node_handle = selection.nodes()[0];

                if message.direction() == MessageDirection::FromWidget {
                    self.subsections_handle_ui_message(message, editor_scene, node_handle);
                }

                match message.data() {
                    UiMessageData::DropdownList(DropdownListMessage::SelectionChanged(index))
                        if message.direction() == MessageDirection::FromWidget =>
                    {
                        if message.destination() == self.collider {
                            self.select_collider(editor_scene, node_handle, *index);
                        }

                        if let Some(index) = index {
                            if message.destination() == self.body {
                                self.select_body(editor_scene, node_handle, graph, *index);
                            } else if message.destination() == self.joint {
                                self.select_joint(editor_scene, node_handle, *index);
                            }
                        }
                    }
                    UiMessageData::Button(ButtonMessage::Click)
                        if message.destination() == self.fit =>
                    {
                        self.fit_collider(editor_scene, node_handle, graph);
                    }
                    _ => {}
                }
            }
        }
    }

    fn subsections_handle_ui_message(
        &mut self,
        message: &UiMessage,
        editor_scene: &EditorScene,
        node_handle: Handle<Node>,
    ) {
        if let Some(&body_handle) = editor_scene.physics.binder.value_of(&node_handle) {
            let body = &editor_scene.physics.bodies[body_handle];
            self.body_section.handle_message(message, body, body_handle);

            if let Some(&collider_handle) = body.colliders.first() {
                let collider = &editor_scene.physics.colliders[collider_handle.into()];
                self.collider_section
                    .handle_message(message, collider, collider_handle.into());
            }

            if let Some(&collider) = body.colliders.get(0) {
                match &editor_scene.physics.colliders[collider.into()].shape {
                    ColliderShapeDesc::Ball(ball) => {
                        self.ball_section
                            .handle_message(message, ball, collider.into());
                    }
                    ColliderShapeDesc::Cylinder(cylinder) => {
                        self.cylinder_section
                            .handle_message(message, cylinder, collider.into());
                    }
                    ColliderShapeDesc::RoundCylinder(_) => {
                        // TODO
                    }
                    ColliderShapeDesc::Cone(cone) => {
                        self.cone_section
                            .handle_message(message, cone, collider.into());
                    }
                    ColliderShapeDesc::Cuboid(cuboid) => {
                        self.cuboid_section
                            .handle_message(message, cuboid, collider.into());
                    }
                    ColliderShapeDesc::Capsule(capsule) => {
                        self.capsule_section
                            .handle_message(message, capsule, collider.into());
                    }
                    ColliderShapeDesc::Segment(_) => {
                        // TODO
                    }
                    ColliderShapeDesc::Triangle(_) => {
                        // TODO
                    }
                    ColliderShapeDesc::Trimesh(_) => {
                        // Nothing to edit.
                    }
                    ColliderShapeDesc::Heightfield(_) => {
                        // Nothing to edit.
                    }
                };
            }

            let mut joint = Handle::NONE;
            for (h, j) in editor_scene.physics.joints.pair_iter() {
                if j.body1 == body_handle.into() {
                    joint = h;
                    break;
                }
            }

            if joint.is_some() {
                self.joint_section.handle_message(
                    message,
                    &editor_scene.physics.joints[joint],
                    joint,
                );
            }
        }
    }

    fn select_body(
        &self,
        editor_scene: &EditorScene,
        node_handle: Handle<Node>,
        graph: &Graph,
        index: usize,
    ) {
        match index {
            0 => {
                // Remove body.
                if let Some(&body_handle) = editor_scene.physics.binder.value_of(&node_handle) {
                    let mut commands = Vec::new();

                    for &collider in editor_scene.physics.bodies[body_handle].colliders.iter() {
                        commands.push(SceneCommand::new(DeleteColliderCommand::new(
                            collider.into(),
                        )))
                    }

                    commands.push(SceneCommand::new(DeleteBodyCommand::new(body_handle)));

                    self.sender
                        .send(Message::do_scene_command(CommandGroup::from(commands)))
                        .unwrap();
                }
            }
            1 | 2 | 3 | 4 => {
                let mut current_status = 0;
                if let Some(&body) = editor_scene.physics.binder.value_of(&node_handle) {
                    current_status = match editor_scene.physics.bodies[body].status {
                        RigidBodyTypeDesc::Dynamic => 1,
                        RigidBodyTypeDesc::Static => 2,
                        RigidBodyTypeDesc::KinematicPositionBased => 3,
                        RigidBodyTypeDesc::KinematicVelocityBased => 4,
                    };
                }

                if index != current_status {
                    // Create body.
                    let node = &graph[node_handle];
                    let body = RigidBody {
                        position: node.global_position(),
                        rotation: **node.local_transform().rotation(),
                        status: match index {
                            1 => RigidBodyTypeDesc::Dynamic,
                            2 => RigidBodyTypeDesc::Static,
                            3 => RigidBodyTypeDesc::KinematicPositionBased,
                            4 => RigidBodyTypeDesc::KinematicVelocityBased,
                            _ => unreachable!(),
                        },
                        ..Default::default()
                    };

                    let mut commands = Vec::new();

                    if let Some(&body) = editor_scene.physics.binder.value_of(&node_handle) {
                        for &collider in editor_scene.physics.bodies[body].colliders.iter() {
                            commands.push(SceneCommand::new(DeleteColliderCommand::new(
                                collider.into(),
                            )))
                        }

                        commands.push(SceneCommand::new(DeleteBodyCommand::new(body)));
                    }

                    commands.push(SceneCommand::new(SetBodyCommand::new(node_handle, body)));

                    self.sender
                        .send(Message::do_scene_command(CommandGroup::from(commands)))
                        .unwrap();
                }
            }
            _ => unreachable!(),
        };
    }

    fn select_collider(
        &self,
        editor_scene: &EditorScene,
        node_handle: Handle<Node>,
        index: Option<usize>,
    ) {
        if let Some(&body) = editor_scene.physics.binder.value_of(&node_handle) {
            let current_index =
                editor_scene.physics.bodies[body]
                    .colliders
                    .first()
                    .map(|&first_collider| {
                        editor_scene.physics.colliders[first_collider.into()]
                            .shape
                            .id() as usize
                    });

            let (can_switch, index) = match (current_index, index) {
                (Some(current_index), Some(index)) if current_index != index => (true, index),
                (None, Some(index)) => (true, index),
                _ => (false, 0),
            };

            if can_switch {
                let collider = match index {
                    0 => Collider {
                        shape: ColliderShapeDesc::Ball(BallDesc { radius: 0.5 }),
                        ..Default::default()
                    },
                    1 => Collider {
                        shape: ColliderShapeDesc::Cylinder(CylinderDesc {
                            half_height: 0.5,
                            radius: 0.5,
                        }),
                        ..Default::default()
                    },
                    2 => Collider {
                        shape: ColliderShapeDesc::RoundCylinder(RoundCylinderDesc {
                            half_height: 0.5,
                            radius: 0.5,
                            border_radius: 0.1,
                        }),
                        ..Default::default()
                    },
                    3 => Collider {
                        shape: ColliderShapeDesc::Cone(ConeDesc {
                            half_height: 0.5,
                            radius: 0.5,
                        }),
                        ..Default::default()
                    },
                    4 => Collider {
                        shape: ColliderShapeDesc::Cuboid(CuboidDesc {
                            half_extents: Vector3::new(0.5, 0.5, 0.5),
                        }),
                        ..Default::default()
                    },
                    5 => Collider {
                        shape: ColliderShapeDesc::Capsule(CapsuleDesc {
                            begin: Vector3::new(0.0, 0.0, 0.0),
                            end: Vector3::new(0.0, 1.0, 0.0),
                            radius: 0.5,
                        }),
                        ..Default::default()
                    },
                    6 => Collider {
                        shape: ColliderShapeDesc::Segment(SegmentDesc {
                            begin: Vector3::new(0.0, 0.0, 0.0),
                            end: Vector3::new(1.0, 0.0, 0.0),
                        }),
                        ..Default::default()
                    },
                    7 => Collider {
                        shape: ColliderShapeDesc::Triangle(TriangleDesc {
                            a: Vector3::new(0.0, 0.0, 0.0),
                            b: Vector3::new(1.0, 0.0, 0.0),
                            c: Vector3::new(1.0, 0.0, 1.0),
                        }),
                        ..Default::default()
                    },
                    8 => Collider {
                        shape: ColliderShapeDesc::Trimesh(TrimeshDesc),
                        ..Default::default()
                    },
                    9 => Collider {
                        shape: ColliderShapeDesc::Heightfield(HeightfieldDesc),
                        ..Default::default()
                    },
                    _ => unreachable!(),
                };
                let mut commands = Vec::new();
                // For now only one collider per body is supported.
                // It is easy to add more.
                if let Some(&first_collider) = editor_scene.physics.bodies[body].colliders.first() {
                    commands.push(SceneCommand::new(DeleteColliderCommand::new(
                        first_collider.into(),
                    )))
                }
                commands.push(SceneCommand::new(AddColliderCommand::new(body, collider)));
                self.sender
                    .send(Message::do_scene_command(CommandGroup::from(commands)))
                    .unwrap();
            }
        }
    }

    fn select_joint(&self, editor_scene: &EditorScene, node_handle: Handle<Node>, index: usize) {
        if let Some(&body_handle) = editor_scene.physics.binder.value_of(&node_handle) {
            let mut joint = Handle::NONE;
            for (h, j) in editor_scene.physics.joints.pair_iter() {
                if j.body1 == body_handle.into() {
                    joint = h;
                    break;
                }
            }

            let current_value = if joint.is_some() {
                let joint = &editor_scene.physics.joints[joint];
                match joint.params {
                    JointParamsDesc::BallJoint(_) => 1,
                    JointParamsDesc::FixedJoint(_) => 2,
                    JointParamsDesc::PrismaticJoint(_) => 3,
                    JointParamsDesc::RevoluteJoint(_) => 4,
                }
            } else {
                0
            };

            if current_value != index {
                let mut commands = Vec::new();

                if joint.is_some() {
                    commands.push(SceneCommand::new(DeleteJointCommand::new(joint)));
                }

                match index {
                    0 => {
                        // Do nothing
                    }
                    1 => commands.push(SceneCommand::new(AddJointCommand::new(Joint {
                        body1: body_handle.into(),
                        body2: Default::default(),
                        params: JointParamsDesc::BallJoint(BallJointDesc {
                            local_anchor1: Default::default(),
                            local_anchor2: Default::default(),
                        }),
                    }))),
                    2 => commands.push(SceneCommand::new(AddJointCommand::new(Joint {
                        body1: body_handle.into(),
                        body2: Default::default(),
                        params: JointParamsDesc::FixedJoint(FixedJointDesc {
                            local_anchor1_translation: Default::default(),
                            local_anchor1_rotation: Default::default(),
                            local_anchor2_translation: Default::default(),
                            local_anchor2_rotation: Default::default(),
                        }),
                    }))),
                    3 => commands.push(SceneCommand::new(AddJointCommand::new(Joint {
                        body1: body_handle.into(),
                        body2: Default::default(),
                        params: JointParamsDesc::PrismaticJoint(PrismaticJointDesc {
                            local_anchor1: Default::default(),
                            local_axis1: Vector3::y(),
                            local_anchor2: Default::default(),
                            local_axis2: Vector3::x(),
                        }),
                    }))),
                    4 => commands.push(SceneCommand::new(AddJointCommand::new(Joint {
                        body1: body_handle.into(),
                        body2: Default::default(),
                        params: JointParamsDesc::RevoluteJoint(RevoluteJointDesc {
                            local_anchor1: Default::default(),
                            local_axis1: Vector3::y(),
                            local_anchor2: Default::default(),
                            local_axis2: Vector3::x(),
                        }),
                    }))),
                    _ => unreachable!(),
                };

                self.sender
                    .send(Message::do_scene_command(CommandGroup::from(commands)))
                    .unwrap();
            }
        }
    }

    fn fit_collider(&self, editor_scene: &EditorScene, node_handle: Handle<Node>, graph: &Graph) {
        if let Some(&body_handle) = editor_scene.physics.binder.value_of(&node_handle) {
            let body = &editor_scene.physics.bodies[body_handle];
            if let Some(&collider_handle) = body.colliders.first() {
                let collider = &editor_scene.physics.colliders[collider_handle.into()];

                let mut bounding_box = AxisAlignedBoundingBox::default();

                for descendant in graph.traverse_handle_iter(node_handle) {
                    if let Node::Mesh(mesh) = &graph[descendant] {
                        let mut mesh_bb = mesh.bounding_box();
                        let scale = graph.global_scale_matrix(descendant);
                        let position = Matrix4::new_translation(&mesh.global_position());
                        mesh_bb.transform(position * scale);
                        bounding_box.add_box(mesh_bb);
                    }
                }

                let node_position = graph[node_handle].global_position();

                self.sender
                    .send(Message::do_scene_command(SetColliderPositionCommand::new(
                        collider_handle.into(),
                        bounding_box.center() - node_position,
                    )))
                    .unwrap();

                match &collider.shape {
                    ColliderShapeDesc::Ball(_) => {
                        let d = (bounding_box.max - bounding_box.min).scale(0.5);

                        let radius = d.x.max(d.y).max(d.z);

                        self.sender
                            .send(Message::do_scene_command(SetBallRadiusCommand::new(
                                collider_handle.into(),
                                radius,
                            )))
                            .unwrap();
                    }
                    ColliderShapeDesc::Cylinder(_) => {
                        let d = (bounding_box.max - bounding_box.min).scale(0.5);

                        let radius = d.x.max(d.z);
                        let height = bounding_box.max.y - bounding_box.min.y;

                        let commands = CommandGroup::from(vec![
                            SceneCommand::new(SetCylinderRadiusCommand::new(
                                collider_handle.into(),
                                radius,
                            )),
                            SceneCommand::new(SetCylinderHalfHeightCommand::new(
                                collider_handle.into(),
                                height * 0.5,
                            )),
                        ]);
                        self.sender
                            .send(Message::do_scene_command(commands))
                            .unwrap();
                    }
                    ColliderShapeDesc::Cone(_) => {
                        // TODO
                    }
                    ColliderShapeDesc::Cuboid(_) => {
                        self.sender
                            .send(Message::do_scene_command(SetCuboidHalfExtentsCommand::new(
                                collider_handle.into(),
                                (bounding_box.max - bounding_box.min).scale(0.5),
                            )))
                            .unwrap();
                    }
                    ColliderShapeDesc::Capsule(_) => {
                        // TODO
                    }
                    // Rest are not convex shapes, so there is no volume that can fit
                    // mesh's vertices.
                    _ => (),
                }
            }
        }
    }
}
