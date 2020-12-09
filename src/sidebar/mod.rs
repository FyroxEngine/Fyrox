use crate::{
    gui::{BuildContext, UiMessage, UiNode},
    scene::{
        EditorScene, MoveNodeCommand, RotateNodeCommand, ScaleNodeCommand, SceneCommand,
        SetNameCommand,
    },
    sidebar::{
        camera::CameraSection, light::LightSection, particle::ParticleSystemSection,
        physics::PhysicsSection, sprite::SpriteSection,
    },
    GameEngine, Message,
};
use rg3d::{
    core::{
        algebra::Vector3,
        math::{quat_from_euler, RotationOrder, UnitQuaternionExt},
        pool::Handle,
    },
    gui::{
        border::BorderBuilder,
        check_box::CheckBoxBuilder,
        color::ColorFieldBuilder,
        decorator::DecoratorBuilder,
        grid::{Column, GridBuilder, Row},
        message::{
            MessageDirection, TextBoxMessage, UiMessageData, Vec3EditorMessage, WidgetMessage,
        },
        numeric::NumericUpDownBuilder,
        scroll_viewer::ScrollViewerBuilder,
        stack_panel::StackPanelBuilder,
        text::TextBuilder,
        text_box::TextBoxBuilder,
        vec::Vec3EditorBuilder,
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowTitle},
        HorizontalAlignment, Thickness, VerticalAlignment,
    },
};
use std::sync::mpsc::Sender;

mod camera;
mod light;
mod particle;
mod physics;
mod sprite;

const ROW_HEIGHT: f32 = 25.0;
const COLUMN_WIDTH: f32 = 120.0;

pub struct SideBar {
    pub window: Handle<UiNode>,
    scroll_viewer: Handle<UiNode>,
    node_name: Handle<UiNode>,
    position: Handle<UiNode>,
    rotation: Handle<UiNode>,
    scale: Handle<UiNode>,
    sender: Sender<Message>,
    light_section: LightSection,
    camera_section: CameraSection,
    particle_system_section: ParticleSystemSection,
    sprite_section: SpriteSection,
    physics_section: PhysicsSection,
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

fn make_f32_input_field(ctx: &mut BuildContext, row: usize) -> Handle<UiNode> {
    NumericUpDownBuilder::new(
        WidgetBuilder::new()
            .on_row(row)
            .with_margin(Thickness::uniform(1.0))
            .on_column(1),
    )
    .build(ctx)
}

fn make_color_input_field(ctx: &mut BuildContext, row: usize) -> Handle<UiNode> {
    ColorFieldBuilder::new(
        WidgetBuilder::new()
            .on_row(row)
            .with_margin(Thickness::uniform(1.0))
            .on_column(1),
    )
    .build(ctx)
}

fn make_bool_input_field(ctx: &mut BuildContext, row: usize) -> Handle<UiNode> {
    CheckBoxBuilder::new(
        WidgetBuilder::new()
            .on_row(row)
            .with_margin(Thickness::uniform(1.0))
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
        let scroll_viewer;
        let node_name;
        let position;
        let rotation;
        let scale;

        let light_section = LightSection::new(ctx, sender.clone());
        let camera_section = CameraSection::new(ctx, sender.clone());
        let particle_system_section = ParticleSystemSection::new(ctx, sender.clone());
        let sprite_section = SpriteSection::new(ctx, sender.clone());
        let physics_section = PhysicsSection::new(ctx, sender.clone());

        let window = WindowBuilder::new(WidgetBuilder::new())
            .can_minimize(false)
            .with_content({
                scroll_viewer =
                    ScrollViewerBuilder::new(WidgetBuilder::new().with_visibility(false))
                        .with_content(
                            StackPanelBuilder::new(
                                WidgetBuilder::new().with_children(&[
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
                                    .add_column(Column::strict(COLUMN_WIDTH))
                                    .add_column(Column::stretch())
                                    .add_row(Row::strict(ROW_HEIGHT))
                                    .add_row(Row::strict(ROW_HEIGHT))
                                    .add_row(Row::strict(ROW_HEIGHT))
                                    .add_row(Row::strict(ROW_HEIGHT))
                                    .add_row(Row::stretch())
                                    .build(ctx),
                                    light_section.section,
                                    light_section.point_light_section.section,
                                    light_section.spot_light_section.section,
                                    camera_section.section,
                                    particle_system_section.section,
                                    sprite_section.section,
                                    physics_section.section,
                                    physics_section.body_section.section,
                                    physics_section.cylinder_section.section,
                                    physics_section.cone_section.section,
                                    physics_section.cuboid_section.section,
                                ]),
                            )
                            .build(ctx),
                        )
                        .build(ctx);
                scroll_viewer
            })
            .with_title(WindowTitle::text("Node Properties"))
            .build(ctx);

        Self {
            scroll_viewer,
            window,
            node_name,
            position,
            rotation,
            sender,
            scale,
            light_section,
            camera_section,
            particle_system_section,
            sprite_section,
            physics_section,
        }
    }

    pub fn sync_to_model(&mut self, editor_scene: &EditorScene, engine: &mut GameEngine) {
        let scene = &engine.scenes[editor_scene.scene];
        engine
            .user_interface
            .send_message(WidgetMessage::visibility(
                self.scroll_viewer,
                MessageDirection::ToWidget,
                editor_scene.selection.is_single_selection(),
            ));
        if editor_scene.selection.is_single_selection() {
            let node_handle = editor_scene.selection.nodes()[0];
            if scene.graph.is_valid_handle(node_handle) {
                let node = &scene.graph[node_handle];

                let ui = &mut engine.user_interface;

                ui.send_message(TextBoxMessage::text(
                    self.node_name,
                    MessageDirection::ToWidget,
                    node.name().to_owned(),
                ));

                ui.send_message(Vec3EditorMessage::value(
                    self.position,
                    MessageDirection::ToWidget,
                    node.local_transform().position(),
                ));

                let euler = node.local_transform().rotation().to_euler();
                let euler_degrees = Vector3::new(
                    euler.x.to_degrees(),
                    euler.y.to_degrees(),
                    euler.z.to_degrees(),
                );
                ui.send_message(Vec3EditorMessage::value(
                    self.rotation,
                    MessageDirection::ToWidget,
                    euler_degrees,
                ));

                ui.send_message(Vec3EditorMessage::value(
                    self.scale,
                    MessageDirection::ToWidget,
                    node.local_transform().scale(),
                ));

                self.light_section.sync_to_model(node, ui);
                self.camera_section.sync_to_model(node, ui);
                self.particle_system_section.sync_to_model(node, ui);
                self.sprite_section.sync_to_model(node, ui);
                self.physics_section.sync_to_model(editor_scene, engine);
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
            let node = &graph[node_handle];

            self.light_section
                .handle_message(message, node, node_handle);
            self.camera_section
                .handle_message(message, node, node_handle);
            self.particle_system_section
                .handle_message(message, node, node_handle);
            self.sprite_section
                .handle_message(message, node, node_handle);
            self.physics_section
                .handle_ui_message(message, editor_scene, engine);

            match &message.data() {
                UiMessageData::Vec3Editor(msg) => {
                    if let Vec3EditorMessage::Value(value) = *msg {
                        let transform = graph[node_handle].local_transform();
                        if message.destination() == self.rotation {
                            let old_rotation = transform.rotation();
                            let euler = Vector3::new(
                                value.x.to_radians(),
                                value.y.to_radians(),
                                value.z.to_radians(),
                            );
                            let new_rotation = quat_from_euler(euler, RotationOrder::XYZ);
                            if !old_rotation.approx_eq(&new_rotation, 0.00001) {
                                self.sender
                                    .send(Message::DoSceneCommand(SceneCommand::RotateNode(
                                        RotateNodeCommand::new(
                                            node_handle,
                                            old_rotation,
                                            new_rotation,
                                        ),
                                    )))
                                    .unwrap();
                            }
                        } else if message.destination() == self.position {
                            let old_position = transform.position();
                            if old_position != value {
                                self.sender
                                    .send(Message::DoSceneCommand(SceneCommand::MoveNode(
                                        MoveNodeCommand::new(node_handle, old_position, value),
                                    )))
                                    .unwrap();
                            }
                        } else if message.destination() == self.scale {
                            let old_scale = transform.scale();
                            if old_scale != value {
                                self.sender
                                    .send(Message::DoSceneCommand(SceneCommand::ScaleNode(
                                        ScaleNodeCommand::new(node_handle, old_scale, value),
                                    )))
                                    .unwrap();
                            }
                        }
                    }
                }
                UiMessageData::TextBox(msg) => {
                    if message.destination() == self.node_name {
                        if let TextBoxMessage::Text(new_name) = msg {
                            let old_name = graph[node_handle].name();
                            if old_name != new_name {
                                self.sender
                                    .send(Message::DoSceneCommand(SceneCommand::SetName(
                                        SetNameCommand::new(node_handle, new_name.to_owned()),
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
