use crate::scene::{DeleteBodyCommand, SetBodyCommand};
use crate::{
    gui::{BuildContext, UiMessage, UiNode},
    scene::{
        EditorScene, MoveNodeCommand, RotateNodeCommand, ScaleNodeCommand, SceneCommand,
        SetNameCommand,
    },
    GameEngine, Message,
};
use rg3d::gui::message::DropdownListMessage;
use rg3d::physics::convex_shape::{BoxShape, CapsuleShape, ConvexShape, SphereShape};
use rg3d::physics::rigid_body::RigidBody;
use rg3d::{
    core::{
        math::{
            quat::{Quat, RotationOrder},
            vec3::Vec3,
        },
        pool::Handle,
    },
    gui::{
        border::BorderBuilder,
        decorator::DecoratorBuilder,
        dropdown_list::DropdownListBuilder,
        grid::{Column, GridBuilder, Row},
        message::{TextBoxMessage, UiMessageData, Vec3EditorMessage},
        text::TextBuilder,
        text_box::TextBoxBuilder,
        vec::Vec3EditorBuilder,
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowTitle},
        HorizontalAlignment, Thickness, VerticalAlignment,
    },
};
use std::sync::mpsc::Sender;

pub struct SideBar {
    pub window: Handle<UiNode>,
    node_name: Handle<UiNode>,
    position: Handle<UiNode>,
    rotation: Handle<UiNode>,
    scale: Handle<UiNode>,
    sender: Sender<Message>,
    body: Handle<UiNode>,
}

fn make_text_mark(ctx: &mut BuildContext, text: &str, row: usize) -> Handle<UiNode> {
    TextBuilder::new(
        WidgetBuilder::new()
            .with_vertical_alignment(VerticalAlignment::Center)
            .with_margin(Thickness::left(4.0))
            .on_row(row)
            .on_column(0),
    )
    .with_text(text)
    .build(ctx)
}

fn make_vec3_input_field(ctx: &mut BuildContext, row: usize) -> Handle<UiNode> {
    Vec3EditorBuilder::new(
        WidgetBuilder::new()
            .with_margin(Thickness::uniform(1.0))
            .on_row(row)
            .on_column(1),
    )
    .build(ctx)
}

fn make_dropdown_list_option(ctx: &mut BuildContext, name: &str) -> Handle<UiNode> {
    DecoratorBuilder::new(BorderBuilder::new(
        WidgetBuilder::new().with_height(26.0).with_child(
            TextBuilder::new(WidgetBuilder::new())
                .with_vertical_text_alignment(VerticalAlignment::Center)
                .with_horizontal_text_alignment(HorizontalAlignment::Center)
                .with_text(name)
                .build(ctx),
        ),
    ))
    .build(ctx)
}

impl SideBar {
    pub fn new(ctx: &mut BuildContext, sender: Sender<Message>) -> Self {
        let node_name;
        let position;
        let rotation;
        let scale;
        let body;
        let window = WindowBuilder::new(WidgetBuilder::new())
            .with_content(
                GridBuilder::new(
                    WidgetBuilder::new()
                        .with_child(make_text_mark(ctx, "Name", 0))
                        .with_child({
                            node_name = TextBoxBuilder::new(
                                WidgetBuilder::new()
                                    .on_row(0)
                                    .on_column(1)
                                    .with_margin(Thickness::uniform(1.0)),
                            )
                            .build(ctx);
                            node_name
                        })
                        .with_child(make_text_mark(ctx, "Position", 1))
                        .with_child({
                            position = make_vec3_input_field(ctx, 1);
                            position
                        })
                        .with_child(make_text_mark(ctx, "Rotation", 2))
                        .with_child({
                            rotation = make_vec3_input_field(ctx, 2);
                            rotation
                        })
                        .with_child(make_text_mark(ctx, "Scale", 3))
                        .with_child({
                            scale = make_vec3_input_field(ctx, 3);
                            scale
                        })
                        .with_child(make_text_mark(ctx, "Body", 4))
                        .with_child({
                            body = DropdownListBuilder::new(
                                WidgetBuilder::new()
                                    .on_row(4)
                                    .on_column(1)
                                    .with_margin(Thickness::uniform(1.0)),
                            )
                            .with_items(vec![
                                make_dropdown_list_option(ctx, "None"),
                                make_dropdown_list_option(ctx, "Sphere"),
                                make_dropdown_list_option(ctx, "Cube"),
                                make_dropdown_list_option(ctx, "Capsule"),
                                make_dropdown_list_option(ctx, "Static Mesh"),
                            ])
                            .build(ctx);
                            body
                        }),
                )
                .add_column(Column::strict(70.0))
                .add_column(Column::stretch())
                .add_row(Row::strict(25.0))
                .add_row(Row::strict(25.0))
                .add_row(Row::strict(25.0))
                .add_row(Row::strict(25.0))
                .add_row(Row::strict(25.0))
                .add_row(Row::stretch())
                .build(ctx),
            )
            .with_title(WindowTitle::text("Node Properties"))
            .build(ctx);

        Self {
            window,
            node_name,
            position,
            rotation,
            sender,
            scale,
            body,
        }
    }

    pub fn sync_to_model(&mut self, editor_scene: &EditorScene, engine: &mut GameEngine) {
        let scene = &engine.scenes[editor_scene.scene];
        if editor_scene.selection.is_single_selection() {
            let node_handle = editor_scene.selection.nodes()[0];
            if scene.graph.is_valid_handle(node_handle) {
                let node = &scene.graph[node_handle];

                let ui = &mut engine.user_interface;

                // These messages created with `handled=true` flag to be able to filter such messages
                // in `handle_message` method. Otherwise each syncing would create command, which is
                // not what we want - we want to create command only when user types something in
                // fields, and such messages comes from ui library and they're not handled by default.
                ui.send_message(UiMessage {
                    handled: true,
                    data: UiMessageData::TextBox(TextBoxMessage::Text(node.name().to_owned())),
                    destination: self.node_name,
                });
                ui.send_message(UiMessage {
                    handled: true,
                    data: UiMessageData::Vec3Editor(Vec3EditorMessage::Value(
                        node.local_transform().position(),
                    )),
                    destination: self.position,
                });

                let euler = node.local_transform().rotation().to_euler();
                let euler_degrees = Vec3::new(
                    euler.x.to_degrees(),
                    euler.y.to_degrees(),
                    euler.z.to_degrees(),
                );
                ui.send_message(UiMessage {
                    handled: true,
                    data: UiMessageData::Vec3Editor(Vec3EditorMessage::Value(euler_degrees)),
                    destination: self.rotation,
                });

                ui.send_message(UiMessage {
                    handled: true,
                    data: UiMessageData::Vec3Editor(Vec3EditorMessage::Value(
                        node.local_transform().scale(),
                    )),
                    destination: self.scale,
                });

                // Sync physical body info.
                let body_handle = scene.physics_binder.body_of(node_handle);
                let index = if body_handle.is_some() {
                    let body = scene.physics.borrow_body(body_handle);
                    match body.get_shape() {
                        ConvexShape::Sphere(_) => 1,
                        ConvexShape::Box(_) => 2,
                        ConvexShape::Capsule(_) => 3,
                        _ => 0,
                    }
                } else {
                    0
                };

                ui.send_message(UiMessage {
                    handled: true,
                    data: UiMessageData::DropdownList(DropdownListMessage::SelectionChanged(Some(
                        index,
                    ))),
                    destination: self.body,
                });
            }
        }
    }

    pub fn handle_message(
        &mut self,
        message: &UiMessage,
        editor_scene: &EditorScene,
        engine: &GameEngine,
    ) {
        let scene = &engine.scenes[editor_scene.scene];
        let graph = &scene.graph;

        if editor_scene.selection.is_single_selection() && !message.handled {
            let node = editor_scene.selection.nodes()[0];
            match &message.data {
                UiMessageData::Vec3Editor(msg) => {
                    if let &Vec3EditorMessage::Value(value) = msg {
                        let transform = graph[node].local_transform();
                        if message.destination == self.rotation {
                            let old_rotation = transform.rotation();
                            let euler = Vec3::new(
                                value.x.to_radians(),
                                value.y.to_radians(),
                                value.z.to_radians(),
                            );
                            let new_rotation = Quat::from_euler(euler, RotationOrder::XYZ);
                            if !old_rotation.approx_eq(new_rotation, 0.001) {
                                self.sender
                                    .send(Message::DoSceneCommand(SceneCommand::RotateNode(
                                        RotateNodeCommand::new(node, old_rotation, new_rotation),
                                    )))
                                    .unwrap();
                            }
                        } else if message.destination == self.position {
                            let old_position = transform.position();
                            if old_position != value {
                                self.sender
                                    .send(Message::DoSceneCommand(SceneCommand::MoveNode(
                                        MoveNodeCommand::new(node, old_position, value),
                                    )))
                                    .unwrap();
                            }
                        } else if message.destination == self.scale {
                            let old_scale = transform.scale();
                            if old_scale != value {
                                self.sender
                                    .send(Message::DoSceneCommand(SceneCommand::ScaleNode(
                                        ScaleNodeCommand::new(node, old_scale, value),
                                    )))
                                    .unwrap();
                            }
                        }
                    }
                }
                UiMessageData::DropdownList(msg) => {
                    if message.destination == self.body {
                        if let DropdownListMessage::SelectionChanged(index) = msg {
                            if let Some(index) = index {
                                match index {
                                    0 => {
                                        let body_handle = scene.physics_binder.body_of(node);
                                        if body_handle.is_some() {
                                            self.sender
                                                .send(Message::DoSceneCommand(
                                                    SceneCommand::DeleteBody(
                                                        DeleteBodyCommand::new(body_handle),
                                                    ),
                                                ))
                                                .unwrap();
                                        }
                                    }
                                    1 | 2 | 3 => {
                                        let mut body = match index {
                                            1 => RigidBody::new(ConvexShape::Sphere(
                                                SphereShape::default(),
                                            )),
                                            2 => RigidBody::new(ConvexShape::Box(
                                                BoxShape::default(),
                                            )),
                                            3 => RigidBody::new(ConvexShape::Capsule(
                                                CapsuleShape::default(),
                                            )),
                                            _ => unreachable!(),
                                        };
                                        body.set_position(graph[node].global_position());
                                        self.sender
                                            .send(Message::DoSceneCommand(SceneCommand::SetBody(
                                                SetBodyCommand::new(node, body),
                                            )))
                                            .unwrap();
                                    }
                                    4 => {
                                        println!("implement me!");
                                    }
                                    _ => unreachable!(),
                                };
                            }
                        }
                    }
                }
                UiMessageData::TextBox(msg) => {
                    if message.destination == self.node_name {
                        if let TextBoxMessage::Text(new_name) = msg {
                            let old_name = graph[node].name();
                            if old_name != new_name {
                                self.sender
                                    .send(Message::DoSceneCommand(SceneCommand::SetName(
                                        SetNameCommand::new(node, new_name.to_owned()),
                                    )))
                                    .unwrap();
                            }
                        }
                    }
                }
                _ => (),
            }
        }
    }
}
