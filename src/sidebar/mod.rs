use crate::{
    gui::{BuildContext, Ui, UiMessage, UiNode},
    scene::{
        commands::{
            graph::{
                MoveNodeCommand, RotateNodeCommand, ScaleNodeCommand, SetNameCommand,
                SetPhysicsBindingCommand, SetTagCommand,
            },
            lod::SetLodGroupCommand,
            SceneCommand,
        },
        EditorScene, Selection,
    },
    send_sync_message,
    sidebar::{
        camera::CameraSection, light::LightSection, lod::LodGroupEditor, mesh::MeshSection,
        particle::ParticleSystemSection, physics::PhysicsSection, sound::SoundSection,
        sprite::SpriteSection, terrain::TerrainSection,
    },
    GameEngine, Message,
};
use rg3d::{
    core::{
        algebra::Vector3,
        math::{quat_from_euler, RotationOrder, UnitQuaternionExt},
        pool::Handle,
        scope_profile,
    },
    engine::resource_manager::ResourceManager,
    gui::{
        border::BorderBuilder,
        button::ButtonBuilder,
        check_box::CheckBoxBuilder,
        color::ColorFieldBuilder,
        decorator::DecoratorBuilder,
        dropdown_list::DropdownListBuilder,
        grid::{Column, GridBuilder, Row},
        message::{
            ButtonMessage, DropdownListMessage, MessageDirection, TextBoxMessage, TextMessage,
            UiMessageData, Vec3EditorMessage, WidgetMessage,
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
    scene::{base::PhysicsBinding, node::Node},
};
use std::sync::mpsc::Sender;

mod camera;
mod light;
mod lod;
mod mesh;
mod particle;
mod physics;
mod sound;
mod sprite;
mod terrain;

const ROW_HEIGHT: f32 = 25.0;
const COLUMN_WIDTH: f32 = 140.0;

pub struct SideBar {
    pub window: Handle<UiNode>,
    scroll_viewer: Handle<UiNode>,
    base_section: BaseSection,
    lod_editor: LodGroupEditor,
    sender: Sender<Message>,
    light_section: LightSection,
    camera_section: CameraSection,
    particle_system_section: ParticleSystemSection,
    sprite_section: SpriteSection,
    mesh_section: MeshSection,
    physics_section: PhysicsSection,
    sound_section: SoundSection,
    pub terrain_section: TerrainSection,
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

fn make_f32_input_field(
    ctx: &mut BuildContext,
    row: usize,
    min: f32,
    max: f32,
    step: f32,
) -> Handle<UiNode> {
    NumericUpDownBuilder::new(
        WidgetBuilder::new()
            .on_row(row)
            .with_margin(Thickness::uniform(1.0))
            .on_column(1),
    )
    .with_min_value(min)
    .with_max_value(max)
    .with_step(step)
    .build(ctx)
}

fn make_int_input_field(
    ctx: &mut BuildContext,
    row: usize,
    min: i32,
    max: i32,
    step: i32,
) -> Handle<UiNode> {
    NumericUpDownBuilder::new(
        WidgetBuilder::new()
            .on_row(row)
            .with_margin(Thickness::uniform(1.0))
            .on_column(1),
    )
    .with_min_value(min as f32)
    .with_max_value(max as f32)
    .with_step(step as f32)
    .with_precision(0)
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
            .with_horizontal_alignment(HorizontalAlignment::Left)
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

pub struct BaseSection {
    pub section: Handle<UiNode>,
    node_name: Handle<UiNode>,
    position: Handle<UiNode>,
    rotation: Handle<UiNode>,
    scale: Handle<UiNode>,
    resource: Handle<UiNode>,
    tag: Handle<UiNode>,
    create_lod_group: Handle<UiNode>,
    remove_lod_group: Handle<UiNode>,
    edit_lod_group: Handle<UiNode>,
    physics_binding: Handle<UiNode>,
}

impl BaseSection {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let node_name;
        let position;
        let rotation;
        let scale;
        let resource;
        let tag;
        let physics_binding;
        let create_lod_group;
        let remove_lod_group;
        let edit_lod_group;
        let section = GridBuilder::new(
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
                .with_child(make_text_mark(ctx, "Resource", 4))
                .with_child({
                    resource =
                        TextBuilder::new(WidgetBuilder::new().on_column(1).on_row(4)).build(ctx);
                    resource
                })
                .with_child(make_text_mark(ctx, "Tag", 5))
                .with_child({
                    tag =
                        TextBoxBuilder::new(WidgetBuilder::new().on_column(1).on_row(5)).build(ctx);
                    tag
                })
                .with_child(make_text_mark(ctx, "Physics Binding", 6))
                .with_child({
                    physics_binding = DropdownListBuilder::new(
                        WidgetBuilder::new()
                            .on_row(6)
                            .on_column(1)
                            .with_margin(Thickness::uniform(1.0)),
                    )
                    .with_close_on_selection(true)
                    .with_items(vec![
                        make_dropdown_list_option(ctx, "Node With Body"),
                        make_dropdown_list_option(ctx, "Body With Node"),
                    ])
                    .build(ctx);
                    physics_binding
                })
                .with_child(make_text_mark(ctx, "LOD Group", 7))
                .with_child(
                    GridBuilder::new(
                        WidgetBuilder::new()
                            .on_row(7)
                            .on_column(1)
                            .with_child({
                                create_lod_group = ButtonBuilder::new(
                                    WidgetBuilder::new()
                                        .with_margin(Thickness::uniform(1.0))
                                        .on_column(0),
                                )
                                .with_text("Create Group")
                                .build(ctx);
                                create_lod_group
                            })
                            .with_child({
                                remove_lod_group = ButtonBuilder::new(
                                    WidgetBuilder::new()
                                        .with_enabled(false)
                                        .with_margin(Thickness::uniform(1.0))
                                        .on_column(1),
                                )
                                .with_text("Remove Group")
                                .build(ctx);
                                remove_lod_group
                            })
                            .with_child({
                                edit_lod_group = ButtonBuilder::new(
                                    WidgetBuilder::new()
                                        .with_enabled(false)
                                        .with_margin(Thickness::uniform(1.0))
                                        .on_column(2),
                                )
                                .with_text("Edit Group...")
                                .build(ctx);
                                edit_lod_group
                            }),
                    )
                    .add_row(Row::stretch())
                    .add_column(Column::stretch())
                    .add_column(Column::stretch())
                    .add_column(Column::stretch())
                    .build(ctx),
                ),
        )
        .add_column(Column::strict(COLUMN_WIDTH))
        .add_column(Column::stretch())
        .add_row(Row::strict(ROW_HEIGHT))
        .add_row(Row::strict(ROW_HEIGHT))
        .add_row(Row::strict(ROW_HEIGHT))
        .add_row(Row::strict(ROW_HEIGHT))
        .add_row(Row::strict(ROW_HEIGHT))
        .add_row(Row::strict(ROW_HEIGHT))
        .add_row(Row::strict(ROW_HEIGHT))
        .add_row(Row::strict(ROW_HEIGHT))
        .add_row(Row::stretch())
        .build(ctx);

        Self {
            section,
            node_name,
            position,
            rotation,
            scale,
            resource,
            tag,
            physics_binding,
            create_lod_group,
            remove_lod_group,
            edit_lod_group,
        }
    }

    pub fn sync_to_model(&self, node: &Node, ui: &Ui) {
        send_sync_message(
            ui,
            TextBoxMessage::text(
                self.node_name,
                MessageDirection::ToWidget,
                node.name().to_owned(),
            ),
        );

        // Prevent edit names of nodes that were created from resource.
        // This is strictly necessary because resolving depends on node
        // names.
        send_sync_message(
            ui,
            WidgetMessage::enabled(
                self.node_name,
                MessageDirection::ToWidget,
                node.resource().is_none() || node.is_resource_instance_root(),
            ),
        );

        send_sync_message(
            ui,
            TextMessage::text(
                self.resource,
                MessageDirection::ToWidget,
                if let Some(resource) = node.resource() {
                    let state = resource.state();
                    state.path().to_string_lossy().into_owned()
                } else {
                    "None".to_owned()
                },
            ),
        );

        send_sync_message(
            ui,
            TextBoxMessage::text(self.tag, MessageDirection::ToWidget, node.tag().to_owned()),
        );

        send_sync_message(
            ui,
            Vec3EditorMessage::value(
                self.position,
                MessageDirection::ToWidget,
                **node.local_transform().position(),
            ),
        );

        let euler = node.local_transform().rotation().to_euler();
        let euler_degrees = Vector3::new(
            euler.x.to_degrees(),
            euler.y.to_degrees(),
            euler.z.to_degrees(),
        );
        send_sync_message(
            ui,
            Vec3EditorMessage::value(self.rotation, MessageDirection::ToWidget, euler_degrees),
        );

        send_sync_message(
            ui,
            Vec3EditorMessage::value(
                self.scale,
                MessageDirection::ToWidget,
                **node.local_transform().scale(),
            ),
        );

        let id = match node.physics_binding() {
            PhysicsBinding::NodeWithBody => 0,
            PhysicsBinding::BodyWithNode => 1,
        };
        send_sync_message(
            ui,
            DropdownListMessage::selection(
                self.physics_binding,
                MessageDirection::ToWidget,
                Some(id),
            ),
        );

        send_sync_message(
            ui,
            WidgetMessage::enabled(
                self.create_lod_group,
                MessageDirection::ToWidget,
                node.lod_group().is_none(),
            ),
        );
        send_sync_message(
            ui,
            WidgetMessage::enabled(
                self.remove_lod_group,
                MessageDirection::ToWidget,
                node.lod_group().is_some(),
            ),
        );
        send_sync_message(
            ui,
            WidgetMessage::enabled(
                self.edit_lod_group,
                MessageDirection::ToWidget,
                node.lod_group().is_some(),
            ),
        );
    }

    fn handle_ui_message(
        &self,
        message: &UiMessage,
        sender: &Sender<Message>,
        node: &Node,
        node_handle: Handle<Node>,
        ui: &mut Ui,
        lod_editor: &mut LodGroupEditor,
    ) {
        match message.data() {
            UiMessageData::Button(ButtonMessage::Click) => {
                if message.destination() == self.create_lod_group {
                    sender
                        .send(Message::DoSceneCommand(SceneCommand::SetLodGroup(
                            SetLodGroupCommand::new(node_handle, Some(Default::default())),
                        )))
                        .unwrap();
                } else if message.destination() == self.remove_lod_group {
                    sender
                        .send(Message::DoSceneCommand(SceneCommand::SetLodGroup(
                            SetLodGroupCommand::new(node_handle, None),
                        )))
                        .unwrap();
                } else if message.destination() == self.edit_lod_group {
                    lod_editor.open(ui);
                }
            }
            &UiMessageData::Vec3Editor(Vec3EditorMessage::Value(value)) => {
                let transform = node.local_transform();
                if message.destination() == self.rotation {
                    let old_rotation = **transform.rotation();
                    let euler = Vector3::new(
                        value.x.to_radians(),
                        value.y.to_radians(),
                        value.z.to_radians(),
                    );
                    let new_rotation = quat_from_euler(euler, RotationOrder::XYZ);
                    if !old_rotation.approx_eq(&new_rotation, 0.00001) {
                        sender
                            .send(Message::DoSceneCommand(SceneCommand::RotateNode(
                                RotateNodeCommand::new(node_handle, old_rotation, new_rotation),
                            )))
                            .unwrap();
                    }
                } else if message.destination() == self.position {
                    let old_position = **transform.position();
                    if old_position != value {
                        sender
                            .send(Message::DoSceneCommand(SceneCommand::MoveNode(
                                MoveNodeCommand::new(node_handle, old_position, value),
                            )))
                            .unwrap();
                    }
                } else if message.destination() == self.scale {
                    let old_scale = **transform.scale();
                    if old_scale != value {
                        sender
                            .send(Message::DoSceneCommand(SceneCommand::ScaleNode(
                                ScaleNodeCommand::new(node_handle, old_scale, value),
                            )))
                            .unwrap();
                    }
                }
            }
            UiMessageData::TextBox(TextBoxMessage::Text(value)) => {
                if message.destination() == self.node_name {
                    let old_name = node.name();
                    if old_name != value {
                        sender
                            .send(Message::DoSceneCommand(SceneCommand::SetName(
                                SetNameCommand::new(node_handle, value.to_owned()),
                            )))
                            .unwrap();
                    }
                } else if message.destination() == self.tag {
                    let old_tag = node.tag();
                    if old_tag != value {
                        sender
                            .send(Message::DoSceneCommand(SceneCommand::SetTag(
                                SetTagCommand::new(node_handle, value.to_owned()),
                            )))
                            .unwrap();
                    }
                }
            }

            UiMessageData::DropdownList(DropdownListMessage::SelectionChanged(Some(index))) => {
                if message.destination() == self.physics_binding {
                    let id = match node.physics_binding() {
                        PhysicsBinding::NodeWithBody => 0,
                        PhysicsBinding::BodyWithNode => 1,
                    };

                    if id != *index {
                        let value = match *index {
                            0 => PhysicsBinding::NodeWithBody,
                            1 => PhysicsBinding::BodyWithNode,
                            _ => unreachable!(),
                        };
                        sender
                            .send(Message::DoSceneCommand(SceneCommand::SetPhysicsBinding(
                                SetPhysicsBindingCommand::new(node_handle, value),
                            )))
                            .unwrap();
                    }
                }
            }

            _ => (),
        }
    }
}

impl SideBar {
    pub fn new(
        ctx: &mut BuildContext,
        sender: Sender<Message>,
        resource_manager: ResourceManager,
    ) -> Self {
        let scroll_viewer;

        let base_section = BaseSection::new(ctx);
        let lod_editor = LodGroupEditor::new(ctx, sender.clone());
        let light_section = LightSection::new(ctx, sender.clone());
        let camera_section = CameraSection::new(ctx, sender.clone());
        let particle_system_section =
            ParticleSystemSection::new(ctx, sender.clone(), resource_manager);
        let sprite_section = SpriteSection::new(ctx, sender.clone());
        let mesh_section = MeshSection::new(ctx, sender.clone());
        let physics_section = PhysicsSection::new(ctx, sender.clone());
        let terrain_section = TerrainSection::new(ctx);
        let sound_section = SoundSection::new(ctx);

        let window = WindowBuilder::new(WidgetBuilder::new())
            .can_minimize(false)
            .with_content({
                scroll_viewer =
                    ScrollViewerBuilder::new(WidgetBuilder::new().with_visibility(false))
                        .with_content(
                            StackPanelBuilder::new(WidgetBuilder::new().with_children(&[
                                base_section.section,
                                light_section.section,
                                camera_section.section,
                                particle_system_section.section,
                                sprite_section.section,
                                mesh_section.section,
                                terrain_section.section,
                                physics_section.section,
                                sound_section.section,
                            ]))
                            .build(ctx),
                        )
                        .build(ctx);
                scroll_viewer
            })
            .with_title(WindowTitle::text("Properties"))
            .build(ctx);

        Self {
            scroll_viewer,
            window,
            base_section,
            sender,
            lod_editor,
            light_section,
            camera_section,
            particle_system_section,
            sprite_section,
            mesh_section,
            physics_section,
            terrain_section,
            sound_section,
        }
    }

    pub fn sync_to_model(&mut self, editor_scene: &EditorScene, engine: &mut GameEngine) {
        scope_profile!();

        send_sync_message(
            &engine.user_interface,
            WidgetMessage::visibility(
                self.scroll_viewer,
                MessageDirection::ToWidget,
                editor_scene.selection.is_single_selection(),
            ),
        );

        let scene = &engine.scenes[editor_scene.scene];
        let ui = &engine.user_interface;

        match &editor_scene.selection {
            Selection::Graph(selection) => {
                if selection.is_single_selection() {
                    let node_handle = selection.nodes()[0];
                    if scene.graph.is_valid_handle(node_handle) {
                        let node = &scene.graph[node_handle];

                        let ui = &mut engine.user_interface;

                        send_sync_message(
                            ui,
                            WidgetMessage::visibility(
                                self.base_section.section,
                                MessageDirection::ToWidget,
                                true,
                            ),
                        );
                        send_sync_message(
                            ui,
                            WidgetMessage::visibility(
                                self.sound_section.section,
                                MessageDirection::ToWidget,
                                false,
                            ),
                        );

                        self.base_section.sync_to_model(node, ui);
                        self.lod_editor.sync_to_model(node, scene, ui);
                        self.light_section.sync_to_model(node, ui);
                        self.camera_section.sync_to_model(node, ui);
                        self.particle_system_section.sync_to_model(
                            node,
                            ui,
                            engine.resource_manager.clone(),
                        );
                        self.sprite_section.sync_to_model(node, ui);
                        self.mesh_section.sync_to_model(node, ui);
                        self.terrain_section.sync_to_model(node, ui);
                        self.physics_section.sync_to_model(editor_scene, engine);
                    }
                }
            }
            Selection::Sound(selection) => {
                for &section in &[
                    self.base_section.section,
                    self.sprite_section.section,
                    self.light_section.section,
                    self.camera_section.section,
                    self.particle_system_section.section,
                    self.mesh_section.section,
                    self.terrain_section.section,
                    self.physics_section.section,
                ] {
                    send_sync_message(
                        ui,
                        WidgetMessage::visibility(section, MessageDirection::ToWidget, false),
                    );
                }

                if selection.is_single_selection() {
                    if let Some(first) = selection.first() {
                        let state = scene.sound_context.state();
                        self.sound_section
                            .sync_to_model(state.source(first), &mut engine.user_interface);
                    }
                }
            }
            _ => {}
        }
    }

    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        editor_scene: &EditorScene,
        engine: &mut GameEngine,
    ) {
        scope_profile!();

        let scene = &engine.scenes[editor_scene.scene];

        match &editor_scene.selection {
            Selection::Graph(selection) => {
                let graph = &scene.graph;

                if selection.is_single_selection() {
                    let node_handle = selection.nodes()[0];
                    let node = &graph[node_handle];

                    self.physics_section
                        .handle_ui_message(message, editor_scene, engine);

                    if message.direction() == MessageDirection::FromWidget {
                        self.light_section
                            .handle_message(message, node, node_handle);
                        self.camera_section
                            .handle_message(message, node, node_handle);
                        self.particle_system_section.handle_message(
                            message,
                            node,
                            node_handle,
                            &engine.user_interface,
                        );
                        self.sprite_section
                            .handle_message(message, node, node_handle);
                        self.mesh_section.handle_message(message, node, node_handle);
                        self.terrain_section.handle_message(
                            message,
                            &mut engine.user_interface,
                            engine.resource_manager.clone(),
                            node,
                            graph,
                            node_handle,
                            &self.sender,
                        );

                        self.lod_editor.handle_ui_message(
                            message,
                            node_handle,
                            node,
                            scene,
                            &mut engine.user_interface,
                        );

                        self.base_section.handle_ui_message(
                            message,
                            &self.sender,
                            node,
                            node_handle,
                            &mut engine.user_interface,
                            &mut self.lod_editor,
                        );
                    }
                }
            }
            Selection::Sound(selection) => {
                if selection.is_single_selection() {
                    if let Some(first) = selection.first() {
                        let state = scene.sound_context.state();
                        self.sound_section.handle_message(
                            message,
                            &self.sender,
                            state.source(first),
                            first,
                        );
                    }
                }
            }
            _ => {}
        }
    }
}
