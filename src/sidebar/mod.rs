use crate::scene::commands::CommandGroup;
use crate::{
    gui::{BuildContext, Ui, UiMessage, UiNode},
    scene::{
        commands::{
            graph::{
                MoveNodeCommand, RotateNodeCommand, ScaleNodeCommand, SetNameCommand,
                SetPhysicsBindingCommand, SetTagCommand,
            },
            lod::{
                AddLodGroupLevelCommand, AddLodObjectCommand, ChangeLodRangeBeginCommand,
                ChangeLodRangeEndCommand, RemoveLodGroupLevelCommand, RemoveLodObjectCommand,
                SetLodGroupCommand,
            },
            SceneCommand,
        },
        EditorScene, Selection,
    },
    send_sync_message,
    sidebar::{
        camera::CameraSection, light::LightSection, mesh::MeshSection,
        particle::ParticleSystemSection, physics::PhysicsSection, sprite::SpriteSection,
        terrain::TerrainSection,
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
        list_view::ListViewBuilder,
        message::{
            ButtonMessage, DropdownListMessage, ListViewMessage, MessageDirection,
            NumericUpDownMessage, TextBoxMessage, TextMessage, TreeMessage, TreeRootMessage,
            UiMessageData, Vec3EditorMessage, WidgetMessage, WindowMessage,
        },
        numeric::NumericUpDownBuilder,
        scroll_viewer::ScrollViewerBuilder,
        stack_panel::StackPanelBuilder,
        text::TextBuilder,
        text_box::TextBoxBuilder,
        tree::{TreeBuilder, TreeRootBuilder},
        vec::Vec3EditorBuilder,
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowTitle},
        HorizontalAlignment, Orientation, Thickness, VerticalAlignment,
    },
    scene::{base::PhysicsBinding, graph::Graph, node::Node, Scene},
};
use std::{rc::Rc, sync::mpsc::Sender};

mod camera;
mod light;
mod mesh;
mod particle;
mod physics;
mod sprite;
mod terrain;

const ROW_HEIGHT: f32 = 25.0;
const COLUMN_WIDTH: f32 = 140.0;

pub struct SideBar {
    pub window: Handle<UiNode>,
    scroll_viewer: Handle<UiNode>,
    node_name: Handle<UiNode>,
    position: Handle<UiNode>,
    rotation: Handle<UiNode>,
    scale: Handle<UiNode>,
    resource: Handle<UiNode>,
    tag: Handle<UiNode>,
    create_lod_group: Handle<UiNode>,
    remove_lod_group: Handle<UiNode>,
    edit_lod_group: Handle<UiNode>,
    lod_editor: LodGroupEditor,
    physics_binding: Handle<UiNode>,
    sender: Sender<Message>,
    light_section: LightSection,
    camera_section: CameraSection,
    particle_system_section: ParticleSystemSection,
    sprite_section: SpriteSection,
    mesh_section: MeshSection,
    physics_section: PhysicsSection,
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

struct ChildSelector {
    window: Handle<UiNode>,
    tree: Handle<UiNode>,
    ok: Handle<UiNode>,
    cancel: Handle<UiNode>,
    sender: Sender<Message>,
    selection: Vec<Handle<Node>>,
}

fn make_tree(node: &Node, handle: Handle<Node>, graph: &Graph, ui: &mut Ui) -> Handle<UiNode> {
    let tree = TreeBuilder::new(WidgetBuilder::new().with_user_data(Rc::new(handle)))
        .with_content(
            TextBuilder::new(WidgetBuilder::new())
                .with_text(format!(
                    "{}({}:{})",
                    graph[handle].name(),
                    handle.index(),
                    handle.generation()
                ))
                .build(&mut ui.build_ctx()),
        )
        .build(&mut ui.build_ctx());

    for &child_handle in node.children() {
        let child = &graph[child_handle];

        let sub_tree = make_tree(child, child_handle, graph, ui);

        ui.send_message(TreeMessage::add_item(
            tree,
            MessageDirection::ToWidget,
            sub_tree,
        ));
    }

    tree
}

impl ChildSelector {
    pub fn new(ctx: &mut BuildContext, sender: Sender<Message>) -> Self {
        let tree;
        let ok;
        let cancel;
        let window = WindowBuilder::new(WidgetBuilder::new().with_width(140.0).with_height(200.0))
            .with_title(WindowTitle::text("Select Child Object"))
            .open(false)
            .can_minimize(false)
            .with_content(
                GridBuilder::new(
                    WidgetBuilder::new()
                        .with_child({
                            tree = TreeRootBuilder::new(WidgetBuilder::new().on_row(0)).build(ctx);
                            tree
                        })
                        .with_child(
                            StackPanelBuilder::new(
                                WidgetBuilder::new()
                                    .on_row(1)
                                    .with_horizontal_alignment(HorizontalAlignment::Right)
                                    .with_child({
                                        ok = ButtonBuilder::new(
                                            WidgetBuilder::new().with_width(60.0),
                                        )
                                        .with_text("OK")
                                        .build(ctx);
                                        ok
                                    })
                                    .with_child({
                                        cancel = ButtonBuilder::new(
                                            WidgetBuilder::new().with_width(60.0),
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
                .add_row(Row::strict(30.0))
                .add_column(Column::stretch())
                .build(ctx),
            )
            .build(ctx);
        Self {
            window,
            tree,
            ok,
            cancel,
            sender,
            selection: Default::default(),
        }
    }

    fn open(&mut self, ui: &mut Ui, node: &Node, scene: &Scene) {
        ui.send_message(WindowMessage::open_modal(
            self.window,
            MessageDirection::ToWidget,
            true,
        ));

        let roots = node
            .children()
            .iter()
            .map(|&c| make_tree(&scene.graph[c], c, &scene.graph, ui))
            .collect::<Vec<_>>();

        ui.send_message(TreeRootMessage::items(
            self.tree,
            MessageDirection::ToWidget,
            roots,
        ));

        ui.send_message(TreeRootMessage::select(
            self.tree,
            MessageDirection::ToWidget,
            vec![],
        ));

        ui.send_message(WidgetMessage::enabled(
            self.ok,
            MessageDirection::ToWidget,
            false,
        ));

        self.selection.clear();
    }

    fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        node_handle: Handle<Node>,
        graph: &Graph,
        lod_index: usize,
        ui: &mut Ui,
    ) {
        match message.data() {
            UiMessageData::Button(ButtonMessage::Click) => {
                if message.destination() == self.ok {
                    let commands = self
                        .selection
                        .iter()
                        .map(|&h| {
                            SceneCommand::AddLodObject(AddLodObjectCommand::new(
                                node_handle,
                                lod_index,
                                h,
                            ))
                        })
                        .collect::<Vec<_>>();

                    if !commands.is_empty() {
                        self.sender
                            .send(Message::DoSceneCommand(SceneCommand::CommandGroup(
                                CommandGroup::from(commands),
                            )))
                            .unwrap();

                        ui.send_message(WindowMessage::close(
                            self.window,
                            MessageDirection::ToWidget,
                        ));
                    }
                } else if message.destination() == self.cancel {
                    ui.send_message(WindowMessage::close(
                        self.window,
                        MessageDirection::ToWidget,
                    ));
                }
            }
            UiMessageData::TreeRoot(TreeRootMessage::Selected(selection)) => {
                if message.destination() == self.tree {
                    self.selection.clear();

                    for &item in selection {
                        let node = *ui.node(item).user_data_ref::<Handle<Node>>().unwrap();

                        for descendant in graph.traverse_handle_iter(node) {
                            self.selection.push(descendant);
                        }
                    }

                    if !self.selection.is_empty() {
                        ui.send_message(WidgetMessage::enabled(
                            self.ok,
                            MessageDirection::ToWidget,
                            true,
                        ))
                    }
                }
            }
            _ => (),
        }
    }
}

struct LodGroupEditor {
    window: Handle<UiNode>,
    lod_levels: Handle<UiNode>,
    add_lod_level: Handle<UiNode>,
    remove_lod_level: Handle<UiNode>,
    current_lod_level: Option<usize>,
    lod_begin: Handle<UiNode>,
    lod_end: Handle<UiNode>,
    objects: Handle<UiNode>,
    add_object: Handle<UiNode>,
    remove_object: Handle<UiNode>,
    sender: Sender<Message>,
    child_selector: ChildSelector,
    selected_object: Option<usize>,
    lod_section: Handle<UiNode>,
}

impl LodGroupEditor {
    pub fn new(ctx: &mut BuildContext, sender: Sender<Message>) -> Self {
        let lod_levels;
        let add_lod_level;
        let lod_begin;
        let lod_end;
        let remove_lod_level;
        let objects;
        let add_object;
        let remove_object;
        let lod_section;
        let window = WindowBuilder::new(WidgetBuilder::new().with_width(500.0).with_height(300.0))
            .open(false)
            .with_title(WindowTitle::text("Edit LOD Group"))
            .can_minimize(false)
            .with_content(
                GridBuilder::new(
                    WidgetBuilder::new()
                        .with_margin(Thickness::uniform(1.0))
                        .with_child(
                            GridBuilder::new(
                                WidgetBuilder::new()
                                    .on_column(0)
                                    .with_child(
                                        GridBuilder::new(
                                            WidgetBuilder::new()
                                                .on_row(0)
                                                .with_child({
                                                    add_lod_level = ButtonBuilder::new(
                                                        WidgetBuilder::new()
                                                            .with_margin(Thickness::uniform(1.0))
                                                            .on_column(0),
                                                    )
                                                    .with_text("Add Level")
                                                    .build(ctx);
                                                    add_lod_level
                                                })
                                                .with_child({
                                                    remove_lod_level = ButtonBuilder::new(
                                                        WidgetBuilder::new()
                                                            .with_margin(Thickness::uniform(1.0))
                                                            .on_column(1),
                                                    )
                                                    .with_text("Remove Level")
                                                    .build(ctx);
                                                    remove_lod_level
                                                }),
                                        )
                                        .add_column(Column::stretch())
                                        .add_column(Column::stretch())
                                        .add_row(Row::stretch())
                                        .build(ctx),
                                    )
                                    .with_child({
                                        lod_levels =
                                            ListViewBuilder::new(WidgetBuilder::new().on_row(1))
                                                .build(ctx);
                                        lod_levels
                                    }),
                            )
                            .add_row(Row::strict(ROW_HEIGHT))
                            .add_row(Row::stretch())
                            .add_column(Column::stretch())
                            .build(ctx),
                        )
                        .with_child({
                            lod_section = GridBuilder::new(
                                WidgetBuilder::new()
                                    .with_enabled(false)
                                    .on_column(1)
                                    .with_child(make_text_mark(ctx, "Begin", 0))
                                    .with_child({
                                        lod_begin = NumericUpDownBuilder::new(
                                            WidgetBuilder::new().on_row(0).on_column(1),
                                        )
                                        .with_min_value(0.0)
                                        .with_max_value(1.0)
                                        .build(ctx);
                                        lod_begin
                                    })
                                    .with_child(make_text_mark(ctx, "End", 1))
                                    .with_child({
                                        lod_end = NumericUpDownBuilder::new(
                                            WidgetBuilder::new().on_row(1).on_column(1),
                                        )
                                        .with_min_value(0.0)
                                        .with_max_value(1.0)
                                        .build(ctx);
                                        lod_end
                                    })
                                    .with_child(
                                        GridBuilder::new(
                                            WidgetBuilder::new()
                                                .on_row(2)
                                                .on_column(1)
                                                .with_child({
                                                    add_object = ButtonBuilder::new(
                                                        WidgetBuilder::new().on_column(0),
                                                    )
                                                    .with_text("Add...")
                                                    .build(ctx);
                                                    add_object
                                                })
                                                .with_child({
                                                    remove_object = ButtonBuilder::new(
                                                        WidgetBuilder::new().on_column(1),
                                                    )
                                                    .with_text("Remove")
                                                    .build(ctx);
                                                    remove_object
                                                }),
                                        )
                                        .add_column(Column::stretch())
                                        .add_column(Column::stretch())
                                        .add_row(Row::stretch())
                                        .build(ctx),
                                    )
                                    .with_child(make_text_mark(ctx, "Objects", 3))
                                    .with_child({
                                        objects = ListViewBuilder::new(
                                            WidgetBuilder::new().on_row(3).on_column(1),
                                        )
                                        .build(ctx);
                                        objects
                                    }),
                            )
                            .add_column(Column::strict(70.0))
                            .add_column(Column::stretch())
                            .add_row(Row::strict(ROW_HEIGHT))
                            .add_row(Row::strict(ROW_HEIGHT))
                            .add_row(Row::strict(ROW_HEIGHT))
                            .add_row(Row::stretch())
                            .build(ctx);
                            lod_section
                        }),
                )
                .add_row(Row::stretch())
                .add_column(Column::stretch())
                .add_column(Column::stretch())
                .build(ctx),
            )
            .build(ctx);

        Self {
            window,
            lod_levels,
            add_lod_level,
            remove_lod_level,
            lod_begin,
            lod_end,
            objects,
            selected_object: None,
            current_lod_level: None,
            child_selector: ChildSelector::new(ctx, sender.clone()),
            sender,
            add_object,
            remove_object,
            lod_section,
        }
    }

    pub fn sync_to_model(&mut self, node: &Node, scene: &Scene, ui: &mut Ui) {
        if let Some(lod_levels) = node.lod_group() {
            let ctx = &mut ui.build_ctx();
            let levels = lod_levels
                .levels
                .iter()
                .enumerate()
                .map(|(i, lod)| {
                    make_dropdown_list_option(
                        ctx,
                        &format!(
                            "LOD {}: {:.1} .. {:.1}%",
                            i,
                            lod.begin() * 100.0,
                            lod.end() * 100.0
                        ),
                    )
                })
                .collect::<Vec<_>>();

            send_sync_message(
                ui,
                ListViewMessage::items(self.lod_levels, MessageDirection::ToWidget, levels),
            );

            if let Some(level) = self.current_lod_level {
                if let Some(level) = lod_levels.levels.get(level) {
                    send_sync_message(
                        ui,
                        NumericUpDownMessage::value(
                            self.lod_begin,
                            MessageDirection::ToWidget,
                            level.begin(),
                        ),
                    );

                    send_sync_message(
                        ui,
                        NumericUpDownMessage::value(
                            self.lod_end,
                            MessageDirection::ToWidget,
                            level.end(),
                        ),
                    );

                    let objects = level
                        .objects
                        .iter()
                        .map(|&object| {
                            DecoratorBuilder::new(BorderBuilder::new(
                                WidgetBuilder::new().with_child(
                                    TextBuilder::new(WidgetBuilder::new())
                                        .with_text(format!(
                                            "{}({}:{})",
                                            scene.graph[object].name(),
                                            object.index(),
                                            object.generation()
                                        ))
                                        .build(&mut ui.build_ctx()),
                                ),
                            ))
                            .build(&mut ui.build_ctx())
                        })
                        .collect::<Vec<_>>();

                    send_sync_message(
                        ui,
                        ListViewMessage::items(self.objects, MessageDirection::ToWidget, objects),
                    );
                }
            }
        } else {
            self.current_lod_level = None;
        }

        send_sync_message(
            ui,
            ListViewMessage::selection(
                self.lod_levels,
                MessageDirection::ToWidget,
                self.current_lod_level,
            ),
        );
    }

    fn open(&mut self, ui: &mut Ui) {
        ui.send_message(WindowMessage::open_modal(
            self.window,
            MessageDirection::ToWidget,
            true,
        ));

        self.selected_object = None;
        self.current_lod_level = None;

        ui.send_message(WidgetMessage::enabled(
            self.lod_section,
            MessageDirection::ToWidget,
            false,
        ));

        ui.send_message(WidgetMessage::enabled(
            self.remove_object,
            MessageDirection::ToWidget,
            false,
        ));

        // Force-sync.
        self.sender.send(Message::SyncToModel).unwrap();
    }

    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        node_handle: Handle<Node>,
        node: &Node,
        scene: &Scene,
        ui: &mut Ui,
    ) {
        if let Some(lod_index) = self.current_lod_level {
            self.child_selector.handle_ui_message(
                message,
                node_handle,
                &scene.graph,
                lod_index,
                ui,
            );
        }

        match message.data() {
            UiMessageData::Button(ButtonMessage::Click) => {
                if message.destination() == self.add_lod_level {
                    self.sender
                        .send(Message::DoSceneCommand(SceneCommand::AddLodGroupLevel(
                            AddLodGroupLevelCommand::new(node_handle, Default::default()),
                        )))
                        .unwrap();
                } else if message.destination() == self.remove_lod_level {
                    if let Some(current_lod_level) = self.current_lod_level {
                        self.sender
                            .send(Message::DoSceneCommand(SceneCommand::RemoveLodGroupLevel(
                                RemoveLodGroupLevelCommand::new(node_handle, current_lod_level),
                            )))
                            .unwrap();
                    }
                } else if message.destination() == self.add_object {
                    self.child_selector.open(ui, node, scene)
                } else if message.destination() == self.remove_object {
                    if let Some(current_lod_level) = self.current_lod_level {
                        if let Some(selected_object) = self.selected_object {
                            self.sender
                                .send(Message::DoSceneCommand(SceneCommand::RemoveLodObject(
                                    RemoveLodObjectCommand::new(
                                        node_handle,
                                        current_lod_level,
                                        selected_object,
                                    ),
                                )))
                                .unwrap();
                        }
                    }
                }
            }
            UiMessageData::ListView(ListViewMessage::SelectionChanged(index)) => {
                if message.destination() == self.lod_levels {
                    self.current_lod_level = *index;
                    self.selected_object = None;
                    // Force sync.
                    self.sender.send(Message::SyncToModel).unwrap();

                    ui.send_message(WidgetMessage::enabled(
                        self.remove_object,
                        MessageDirection::ToWidget,
                        index.is_some(),
                    ));

                    ui.send_message(WidgetMessage::enabled(
                        self.lod_section,
                        MessageDirection::ToWidget,
                        index.is_some(),
                    ));
                } else if message.destination() == self.objects {
                    self.selected_object = *index;
                    // Force sync.
                    self.sender.send(Message::SyncToModel).unwrap();

                    ui.send_message(WidgetMessage::enabled(
                        self.remove_object,
                        MessageDirection::ToWidget,
                        index.is_some(),
                    ));
                }
            }
            UiMessageData::NumericUpDown(NumericUpDownMessage::Value(value)) => {
                if let Some(current_lod_level) = self.current_lod_level {
                    if message.destination() == self.lod_begin {
                        self.sender
                            .send(Message::DoSceneCommand(SceneCommand::ChangeLodRangeBegin(
                                ChangeLodRangeBeginCommand::new(
                                    node_handle,
                                    current_lod_level,
                                    *value,
                                ),
                            )))
                            .unwrap();
                    } else if message.destination() == self.lod_end {
                        self.sender
                            .send(Message::DoSceneCommand(SceneCommand::ChangeLodRangeEnd(
                                ChangeLodRangeEndCommand::new(
                                    node_handle,
                                    current_lod_level,
                                    *value,
                                ),
                            )))
                            .unwrap();
                    }
                }
            }
            _ => {}
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
        let lod_editor = LodGroupEditor::new(ctx, sender.clone());

        let light_section = LightSection::new(ctx, sender.clone());
        let camera_section = CameraSection::new(ctx, sender.clone());
        let particle_system_section =
            ParticleSystemSection::new(ctx, sender.clone(), resource_manager);
        let sprite_section = SpriteSection::new(ctx, sender.clone());
        let mesh_section = MeshSection::new(ctx, sender.clone());
        let physics_section = PhysicsSection::new(ctx, sender.clone());
        let terrain_section = TerrainSection::new(ctx);

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
                                            })
                                            .with_child(make_text_mark(ctx, "Resource", 4))
                                            .with_child({
                                                resource = TextBuilder::new(
                                                    WidgetBuilder::new().on_column(1).on_row(4),
                                                )
                                                .build(ctx);
                                                resource
                                            })
                                            .with_child(make_text_mark(ctx, "Tag", 5))
                                            .with_child({
                                                tag = TextBoxBuilder::new(
                                                    WidgetBuilder::new().on_column(1).on_row(5),
                                                )
                                                .build(ctx);
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
                                                    make_dropdown_list_option(
                                                        ctx,
                                                        "Node With Body",
                                                    ),
                                                    make_dropdown_list_option(
                                                        ctx,
                                                        "Body With Node",
                                                    ),
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
                                                                    .with_margin(
                                                                        Thickness::uniform(1.0),
                                                                    )
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
                                                                    .with_margin(
                                                                        Thickness::uniform(1.0),
                                                                    )
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
                                                                    .with_margin(
                                                                        Thickness::uniform(1.0),
                                                                    )
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
                                    .build(ctx),
                                    light_section.section,
                                    camera_section.section,
                                    particle_system_section.section,
                                    sprite_section.section,
                                    mesh_section.section,
                                    terrain_section.section,
                                    physics_section.section,
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
            resource,
            tag,
            lod_editor,
            light_section,
            camera_section,
            particle_system_section,
            sprite_section,
            mesh_section,
            physics_section,
            terrain_section,
            physics_binding,
            create_lod_group,
            remove_lod_group,
            edit_lod_group,
        }
    }

    pub fn sync_to_model(&mut self, editor_scene: &EditorScene, engine: &mut GameEngine) {
        scope_profile!();

        // For now only nodes are editable through side bar.
        if let Selection::Graph(selection) = &editor_scene.selection {
            let scene = &engine.scenes[editor_scene.scene];
            send_sync_message(
                &engine.user_interface,
                WidgetMessage::visibility(
                    self.scroll_viewer,
                    MessageDirection::ToWidget,
                    selection.is_single_selection(),
                ),
            );
            if selection.is_single_selection() {
                let node_handle = selection.nodes()[0];
                if scene.graph.is_valid_handle(node_handle) {
                    let node = &scene.graph[node_handle];

                    let ui = &mut engine.user_interface;

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
                        TextBoxMessage::text(
                            self.tag,
                            MessageDirection::ToWidget,
                            node.tag().to_owned(),
                        ),
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
                        Vec3EditorMessage::value(
                            self.rotation,
                            MessageDirection::ToWidget,
                            euler_degrees,
                        ),
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
    }

    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        editor_scene: &EditorScene,
        engine: &mut GameEngine,
    ) {
        scope_profile!();

        // For now only nodes are editable through side bar.
        if let Selection::Graph(selection) = &editor_scene.selection {
            let scene = &engine.scenes[editor_scene.scene];
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

                    match message.data() {
                        UiMessageData::Button(ButtonMessage::Click) => {
                            if message.destination() == self.create_lod_group {
                                self.sender
                                    .send(Message::DoSceneCommand(SceneCommand::SetLodGroup(
                                        SetLodGroupCommand::new(
                                            node_handle,
                                            Some(Default::default()),
                                        ),
                                    )))
                                    .unwrap();
                            } else if message.destination() == self.remove_lod_group {
                                self.sender
                                    .send(Message::DoSceneCommand(SceneCommand::SetLodGroup(
                                        SetLodGroupCommand::new(node_handle, None),
                                    )))
                                    .unwrap();
                            } else if message.destination() == self.edit_lod_group {
                                self.lod_editor.open(&mut engine.user_interface);
                            }
                        }
                        &UiMessageData::Vec3Editor(Vec3EditorMessage::Value(value)) => {
                            let transform = graph[node_handle].local_transform();
                            if message.destination() == self.rotation {
                                let old_rotation = **transform.rotation();
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
                                let old_position = **transform.position();
                                if old_position != value {
                                    self.sender
                                        .send(Message::DoSceneCommand(SceneCommand::MoveNode(
                                            MoveNodeCommand::new(node_handle, old_position, value),
                                        )))
                                        .unwrap();
                                }
                            } else if message.destination() == self.scale {
                                let old_scale = **transform.scale();
                                if old_scale != value {
                                    self.sender
                                        .send(Message::DoSceneCommand(SceneCommand::ScaleNode(
                                            ScaleNodeCommand::new(node_handle, old_scale, value),
                                        )))
                                        .unwrap();
                                }
                            }
                        }
                        UiMessageData::TextBox(TextBoxMessage::Text(value)) => {
                            if message.destination() == self.node_name {
                                let old_name = graph[node_handle].name();
                                if old_name != value {
                                    self.sender
                                        .send(Message::DoSceneCommand(SceneCommand::SetName(
                                            SetNameCommand::new(node_handle, value.to_owned()),
                                        )))
                                        .unwrap();
                                }
                            } else if message.destination() == self.tag {
                                let old_tag = graph[node_handle].tag();
                                if old_tag != value {
                                    self.sender
                                        .send(Message::DoSceneCommand(SceneCommand::SetTag(
                                            SetTagCommand::new(node_handle, value.to_owned()),
                                        )))
                                        .unwrap();
                                }
                            }
                        }

                        UiMessageData::DropdownList(DropdownListMessage::SelectionChanged(
                            Some(index),
                        )) => {
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
                                    self.sender
                                        .send(Message::DoSceneCommand(
                                            SceneCommand::SetPhysicsBinding(
                                                SetPhysicsBindingCommand::new(node_handle, value),
                                            ),
                                        ))
                                        .unwrap();
                                }
                            }
                        }

                        _ => (),
                    }
                }
            }
        }
    }
}
