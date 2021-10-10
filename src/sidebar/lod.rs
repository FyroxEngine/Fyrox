use crate::gui::make_dropdown_list_option;
use crate::scene::commands::SceneCommand;
use crate::{
    scene::commands::{
        lod::{
            AddLodGroupLevelCommand, AddLodObjectCommand, ChangeLodRangeBeginCommand,
            ChangeLodRangeEndCommand, RemoveLodGroupLevelCommand, RemoveLodObjectCommand,
        },
        CommandGroup,
    },
    send_sync_message,
    sidebar::{make_text_mark, ROW_HEIGHT},
    Message,
};
use rg3d::gui::message::UiMessage;
use rg3d::gui::numeric::NumericUpDownMessage;
use rg3d::gui::{BuildContext, UiNode, UserInterface};
use rg3d::{
    core::pool::Handle,
    gui::{
        border::BorderBuilder,
        button::ButtonBuilder,
        decorator::DecoratorBuilder,
        grid::{Column, GridBuilder, Row},
        list_view::ListViewBuilder,
        message::{
            ButtonMessage, ListViewMessage, MessageDirection, TreeMessage, TreeRootMessage,
            UiMessageData, WidgetMessage, WindowMessage,
        },
        numeric::NumericUpDownBuilder,
        stack_panel::StackPanelBuilder,
        text::TextBuilder,
        tree::{TreeBuilder, TreeRootBuilder},
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowTitle},
        HorizontalAlignment, Orientation, Thickness,
    },
    scene::{graph::Graph, node::Node, Scene},
};
use std::{rc::Rc, sync::mpsc::Sender};

struct ChildSelector {
    window: Handle<UiNode>,
    tree: Handle<UiNode>,
    ok: Handle<UiNode>,
    cancel: Handle<UiNode>,
    sender: Sender<Message>,
    selection: Vec<Handle<Node>>,
}

fn make_tree(
    node: &Node,
    handle: Handle<Node>,
    graph: &Graph,
    ui: &mut UserInterface,
) -> Handle<UiNode> {
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

    fn open(&mut self, ui: &mut UserInterface, node: &Node, scene: &Scene) {
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
        ui: &mut UserInterface,
    ) {
        match message.data() {
            UiMessageData::Button(ButtonMessage::Click) => {
                if message.destination() == self.ok {
                    let commands = self
                        .selection
                        .iter()
                        .map(|&h| {
                            SceneCommand::new(AddLodObjectCommand::new(node_handle, lod_index, h))
                        })
                        .collect::<Vec<_>>();

                    if !commands.is_empty() {
                        self.sender
                            .send(Message::do_scene_command(CommandGroup::from(commands)))
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

pub struct LodGroupEditor {
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

    pub fn sync_to_model(&mut self, node: &Node, scene: &Scene, ui: &mut UserInterface) {
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

    pub fn open(&mut self, ui: &mut UserInterface) {
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
        scene: &Scene,
        ui: &mut UserInterface,
    ) {
        let node = &scene.graph[node_handle];

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
                        .send(Message::do_scene_command(AddLodGroupLevelCommand::new(
                            node_handle,
                            Default::default(),
                        )))
                        .unwrap();
                } else if message.destination() == self.remove_lod_level {
                    if let Some(current_lod_level) = self.current_lod_level {
                        self.sender
                            .send(Message::do_scene_command(RemoveLodGroupLevelCommand::new(
                                node_handle,
                                current_lod_level,
                            )))
                            .unwrap();
                    }
                } else if message.destination() == self.add_object {
                    self.child_selector.open(ui, node, scene)
                } else if message.destination() == self.remove_object {
                    if let Some(current_lod_level) = self.current_lod_level {
                        if let Some(selected_object) = self.selected_object {
                            self.sender
                                .send(Message::do_scene_command(RemoveLodObjectCommand::new(
                                    node_handle,
                                    current_lod_level,
                                    selected_object,
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
            UiMessageData::User(msg) => {
                if let Some(&NumericUpDownMessage::Value(value)) =
                    msg.cast::<NumericUpDownMessage<f32>>()
                {
                    if let Some(current_lod_level) = self.current_lod_level {
                        if message.destination() == self.lod_begin {
                            self.sender
                                .send(Message::do_scene_command(ChangeLodRangeBeginCommand::new(
                                    node_handle,
                                    current_lod_level,
                                    value,
                                )))
                                .unwrap();
                        } else if message.destination() == self.lod_end {
                            self.sender
                                .send(Message::do_scene_command(ChangeLodRangeEndCommand::new(
                                    node_handle,
                                    current_lod_level,
                                    value,
                                )))
                                .unwrap();
                        }
                    }
                }
            }
            _ => {}
        }
    }
}
