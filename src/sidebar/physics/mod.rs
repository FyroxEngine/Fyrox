use crate::gui::Ui;
use crate::sidebar::physics::body::BodySection;
use crate::sidebar::physics::cylinder::CylinderSection;
use crate::{
    gui::{BuildContext, UiMessage, UiNode},
    physics::{Collider, RigidBody},
    scene::{
        CommandGroup, DeleteBodyCommand, DeleteColliderCommand, EditorScene, SceneCommand,
        SetBodyCommand, SetColliderCommand,
    },
    sidebar::{make_dropdown_list_option, make_text_mark, COLUMN_WIDTH, ROW_HEIGHT},
    GameEngine, Message,
};
use rg3d::core::algebra::Vector3;
use rg3d::{
    core::pool::Handle,
    gui::{
        dropdown_list::DropdownListBuilder,
        grid::{Column, GridBuilder, Row},
        message::{DropdownListMessage, MessageDirection, UiMessageData, WidgetMessage},
        widget::WidgetBuilder,
        Thickness,
    },
    scene::physics::{
        BallDesc, BodyStatusDesc, CapsuleDesc, ColliderShapeDesc, ConeDesc, CuboidDesc,
        CylinderDesc, HeightfieldDesc, RoundCylinderDesc, SegmentDesc, TriangleDesc, TrimeshDesc,
    },
};
use std::sync::mpsc::Sender;

mod body;
mod capsule;
mod cone;
mod cuboid;
mod cylinder;
mod segment;
mod triangle;
mod trimesh;

pub struct PhysicsSection {
    pub section: Handle<UiNode>,
    body: Handle<UiNode>,
    collider: Handle<UiNode>,
    collider_text: Handle<UiNode>,
    sender: Sender<Message>,
    pub body_section: BodySection,
    pub cylinder_section: CylinderSection,
}

impl PhysicsSection {
    pub fn new(ctx: &mut BuildContext, sender: Sender<Message>) -> Self {
        let body;
        let collider;
        let collider_text;

        let section = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(make_text_mark(ctx, "Body", 0))
                .with_child({
                    body = DropdownListBuilder::new(
                        WidgetBuilder::new()
                            .on_row(0)
                            .on_column(1)
                            .with_margin(Thickness::uniform(1.0)),
                    )
                    .with_items(vec![
                        make_dropdown_list_option(ctx, "None"),
                        make_dropdown_list_option(ctx, "Dynamic"),
                        make_dropdown_list_option(ctx, "Static"),
                        make_dropdown_list_option(ctx, "Kinematic"),
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
                }),
        )
        .add_column(Column::strict(COLUMN_WIDTH))
        .add_column(Column::stretch())
        .add_row(Row::strict(ROW_HEIGHT))
        .add_row(Row::strict(ROW_HEIGHT))
        .build(ctx);

        Self {
            body_section: BodySection::new(ctx, sender.clone()),
            cylinder_section: CylinderSection::new(ctx, sender.clone()),
            section,
            body,
            collider,
            collider_text,
            sender,
        }
    }

    pub fn sync_to_model(&mut self, editor_scene: &EditorScene, engine: &mut GameEngine) {
        let scene = &engine.scenes[editor_scene.scene];

        if editor_scene.selection.is_single_selection() {
            let node_handle = editor_scene.selection.nodes()[0];
            if scene.graph.is_valid_handle(node_handle) {
                let ui = &mut engine.user_interface;

                // Sync physical body info.
                let body_index =
                    if let Some(&body_handle) = editor_scene.physics.binder.get(&node_handle) {
                        let body = &editor_scene.physics.bodies[body_handle];
                        match body.status {
                            BodyStatusDesc::Dynamic => 1,
                            BodyStatusDesc::Static => 2,
                            BodyStatusDesc::Kinematic => 3,
                        }
                    } else {
                        0
                    };

                fn toggle_visibility(ui: &mut Ui, destination: Handle<UiNode>, value: bool) {
                    ui.send_message(WidgetMessage::visibility(
                        destination,
                        MessageDirection::ToWidget,
                        value,
                    ));
                };

                toggle_visibility(ui, self.collider, body_index != 0);
                toggle_visibility(ui, self.collider_text, body_index != 0);

                ui.send_message(DropdownListMessage::selection(
                    self.body,
                    MessageDirection::ToWidget,
                    Some(body_index),
                ));

                toggle_visibility(ui, self.cylinder_section.section, false);
                toggle_visibility(ui, self.body_section.section, false);

                if let Some(&body_handle) = editor_scene.physics.binder.get(&node_handle) {
                    let body = &editor_scene.physics.bodies[body_handle];

                    self.body_section.sync_to_model(body, ui);
                    toggle_visibility(ui, self.body_section.section, true);

                    if let Some(&collider) = body.colliders.get(0) {
                        let collider_index =
                            match &editor_scene.physics.colliders[collider.into()].shape {
                                ColliderShapeDesc::Ball(_) => 0,
                                ColliderShapeDesc::Cylinder(cylinder) => {
                                    toggle_visibility(ui, self.cylinder_section.section, true);
                                    self.cylinder_section.sync_to_model(cylinder, ui);
                                    1
                                }
                                ColliderShapeDesc::RoundCylinder(_) => 2,
                                ColliderShapeDesc::Cone(_) => 3,
                                ColliderShapeDesc::Cuboid(_) => 4,
                                ColliderShapeDesc::Capsule(_) => 5,
                                ColliderShapeDesc::Segment(_) => 6,
                                ColliderShapeDesc::Triangle(_) => 7,
                                ColliderShapeDesc::Trimesh(_) => 8,
                                ColliderShapeDesc::Heightfield(_) => 9,
                            };
                        ui.send_message(DropdownListMessage::selection(
                            self.collider,
                            MessageDirection::ToWidget,
                            Some(collider_index),
                        ));
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
        let scene = &engine.scenes[editor_scene.scene];
        let graph = &scene.graph;

        if editor_scene.selection.is_single_selection()
            && message.direction() == MessageDirection::FromWidget
        {
            let node_handle = editor_scene.selection.nodes()[0];

            if let Some(&body_handle) = editor_scene.physics.binder.get(&node_handle) {
                let body = &editor_scene.physics.bodies[body_handle];
                self.body_section.handle_message(message, body, body_handle);

                if let Some(&collider) = body.colliders.get(0) {
                    match &editor_scene.physics.colliders[collider.into()].shape {
                        ColliderShapeDesc::Ball(_) => {}
                        ColliderShapeDesc::Cylinder(cylinder) => {
                            self.cylinder_section.handle_message(
                                message,
                                cylinder,
                                collider.into(),
                            );
                        }
                        ColliderShapeDesc::RoundCylinder(_) => {}
                        ColliderShapeDesc::Cone(_) => {}
                        ColliderShapeDesc::Cuboid(_) => {}
                        ColliderShapeDesc::Capsule(_) => {}
                        ColliderShapeDesc::Segment(_) => {}
                        ColliderShapeDesc::Triangle(_) => {}
                        ColliderShapeDesc::Trimesh(_) => {}
                        ColliderShapeDesc::Heightfield(_) => {}
                    };
                }
            }

            if let UiMessageData::DropdownList(msg) = &message.data() {
                if let DropdownListMessage::SelectionChanged(index) = msg {
                    if let Some(index) = index {
                        if message.destination() == self.body {
                            match index {
                                0 => {
                                    // Remove body.
                                    if let Some(&body_handle) =
                                        editor_scene.physics.binder.get(&node_handle)
                                    {
                                        let mut commands = Vec::new();

                                        for &collider in editor_scene.physics.bodies[body_handle]
                                            .colliders
                                            .iter()
                                        {
                                            commands.push(SceneCommand::DeleteCollider(
                                                DeleteColliderCommand::new(collider.into()),
                                            ))
                                        }

                                        commands.push(SceneCommand::DeleteBody(
                                            DeleteBodyCommand::new(body_handle),
                                        ));

                                        self.sender
                                            .send(Message::DoSceneCommand(
                                                SceneCommand::CommandGroup(CommandGroup::from(
                                                    commands,
                                                )),
                                            ))
                                            .unwrap();
                                    }
                                }
                                1 | 2 | 3 => {
                                    let mut current_status = 0;
                                    if let Some(&body) =
                                        editor_scene.physics.binder.get(&node_handle)
                                    {
                                        current_status =
                                            match editor_scene.physics.bodies[body].status {
                                                BodyStatusDesc::Dynamic => 1,
                                                BodyStatusDesc::Static => 2,
                                                BodyStatusDesc::Kinematic => 3,
                                            };
                                    }

                                    if *index != current_status {
                                        // Create body.
                                        let node = &graph[node_handle];
                                        let body = RigidBody {
                                            position: node.global_position(),
                                            rotation: node.local_transform().rotation(),
                                            status: match index {
                                                1 => BodyStatusDesc::Dynamic,
                                                2 => BodyStatusDesc::Static,
                                                3 => BodyStatusDesc::Kinematic,
                                                _ => unreachable!(),
                                            },
                                            ..Default::default()
                                        };

                                        let mut commands = Vec::new();

                                        if let Some(&body) =
                                            editor_scene.physics.binder.get(&node_handle)
                                        {
                                            for &collider in
                                                editor_scene.physics.bodies[body].colliders.iter()
                                            {
                                                commands.push(SceneCommand::DeleteCollider(
                                                    DeleteColliderCommand::new(collider.into()),
                                                ))
                                            }

                                            commands.push(SceneCommand::DeleteBody(
                                                DeleteBodyCommand::new(body),
                                            ));
                                        }

                                        commands.push(SceneCommand::SetBody(SetBodyCommand::new(
                                            node_handle,
                                            body,
                                        )));

                                        self.sender
                                            .send(Message::DoSceneCommand(
                                                SceneCommand::CommandGroup(CommandGroup::from(
                                                    commands,
                                                )),
                                            ))
                                            .unwrap();
                                    }
                                }
                                _ => unreachable!(),
                            };
                        } else if message.destination() == self.collider {
                            if let Some(&body) = editor_scene.physics.binder.get(&node_handle) {
                                let mut current_index = 0;
                                if let Some(&first_collider) =
                                    editor_scene.physics.bodies[body].colliders.first()
                                {
                                    current_index = editor_scene.physics.colliders
                                        [first_collider.into()]
                                    .shape
                                    .id();
                                }

                                if current_index != *index as u32 {
                                    let collider = match index {
                                        0 => Collider {
                                            shape: ColliderShapeDesc::Ball(BallDesc {
                                                radius: 0.5,
                                            }),
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
                                            shape: ColliderShapeDesc::RoundCylinder(
                                                RoundCylinderDesc {
                                                    half_height: 0.5,
                                                    radius: 0.5,
                                                    border_radius: 0.1,
                                                },
                                            ),
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
                                    if let Some(&first_collider) =
                                        editor_scene.physics.bodies[body].colliders.first()
                                    {
                                        commands.push(SceneCommand::DeleteCollider(
                                            DeleteColliderCommand::new(first_collider.into()),
                                        ))
                                    }
                                    commands.push(SceneCommand::SetCollider(
                                        SetColliderCommand::new(body, collider),
                                    ));
                                    self.sender
                                        .send(Message::DoSceneCommand(SceneCommand::CommandGroup(
                                            CommandGroup::from(commands),
                                        )))
                                        .unwrap();
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
