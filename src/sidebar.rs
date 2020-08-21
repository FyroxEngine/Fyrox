use crate::{
    gui::{BuildContext, UiMessage, UiNode},
    scene::{
        EditorScene, MoveNodeCommand, RotateNodeCommand, ScaleNodeCommand, SceneCommand,
        SetNameCommand,
    },
    GameEngine, Message,
};
use rg3d::{
    core::{
        math::{
            quat::{Quat, RotationOrder},
            vec3::Vec3,
        },
        pool::Handle,
    },
    gui::{
        grid::{Column, GridBuilder, Row},
        message::{TextBoxMessage, UiMessageData, Vec3EditorMessage},
        text::TextBuilder,
        text_box::TextBoxBuilder,
        vec::Vec3EditorBuilder,
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowTitle},
        Thickness, VerticalAlignment,
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

impl SideBar {
    pub fn new(ctx: &mut BuildContext, sender: Sender<Message>) -> Self {
        let node_name;
        let position;
        let rotation;
        let scale;
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
                        }),
                )
                .add_column(Column::strict(70.0))
                .add_column(Column::stretch())
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
        }
    }

    pub fn sync_to_model(&mut self, editor_scene: &EditorScene, engine: &mut GameEngine) {
        let scene = &engine.scenes[editor_scene.scene];
        if editor_scene.selection.is_single_selection() {
            let node = editor_scene.selection.nodes()[0];
            if scene.graph.is_valid_handle(node) {
                let node = &scene.graph[node];

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
            }
        }
    }

    pub fn handle_message(
        &mut self,
        message: &UiMessage,
        editor_scene: &EditorScene,
        engine: &GameEngine,
    ) {
        let graph = &engine.scenes[editor_scene.scene].graph;

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
