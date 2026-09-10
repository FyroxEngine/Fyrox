// Copyright (c) 2019-present Dmitry Stepanov and Fyrox Engine contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use crate::{
    command::{Command, CommandGroup},
    fyrox::{
        core::{algebra::Vector3, pool::Handle},
        engine::Engine,
        fxhash::FxHashMap,
        graph::constructor::{ConstructorVariantId, GraphNodeConstructorContainer, VariantResult},
        gui::{
            button::{Button, ButtonBuilder, ButtonMessage},
            grid::{Column, GridBuilder, Row},
            list_view::{ListView, ListViewBuilder},
            message::UiMessage,
            scroll_viewer::ScrollViewerBuilder,
            stack_panel::StackPanelBuilder,
            text::TextBuilder,
            tree::{Tree, TreeBuilder, TreeMessage, TreeRoot, TreeRootBuilder, TreeRootMessage},
            widget::WidgetBuilder,
            window::{Window, WindowAlignment, WindowBuilder, WindowMessage, WindowTitle},
            BuildContext, HorizontalAlignment, Orientation, Thickness, UiNode, UserInterface,
        },
        scene::{graph::Graph, node::Node},
    },
    message::MessageSender,
    scene::commands::graph::{
        AddNodeCommand, LinkNodesCommand, MoveNodeCommand, ReplaceNodeCommand, SetGraphRootCommand,
    },
    scene::{GameScene, Selection},
    ui_scene::{commands::graph::AddWidgetCommand, UiScene},
};
use fyrox::gui::widget::WidgetMessage;

#[derive(Default, Eq, PartialEq, Copy, Clone, Debug)]
pub enum EntityCreatorMode {
    #[default]
    CreateChild,
    CreateParent,
    CreateReplacement,
}

pub struct EntityCreator {
    window: Handle<Window>,
    recent_list: Handle<ListView>,
    groups_tree: Handle<TreeRoot>,
    items_map: FxHashMap<Handle<Tree>, ConstructorVariantId>,
    create: Handle<Button>,
    cancel: Handle<Button>,
    selection: Option<ConstructorVariantId>,
    mode: EntityCreatorMode,
}

fn make_tree_item(ui: &mut UserInterface, text: &str) -> Handle<Tree> {
    let ctx = &mut ui.build_ctx();
    TreeBuilder::new(WidgetBuilder::new())
        .with_content(
            TextBuilder::new(WidgetBuilder::new())
                .with_text(text)
                .build(ctx),
        )
        .build(ctx)
}

impl EntityCreator {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let recent_list = ListViewBuilder::new(WidgetBuilder::new().on_column(0)).build(ctx);

        let groups_tree = TreeRootBuilder::new(WidgetBuilder::new()).build(ctx);

        let groups_scroll_viewer = ScrollViewerBuilder::new(WidgetBuilder::new().on_column(1))
            .with_content(groups_tree)
            .build(ctx);

        let create = ButtonBuilder::new(
            WidgetBuilder::new()
                .with_enabled(false)
                .with_width(100.0)
                .with_margin(Thickness::uniform(2.0)),
        )
        .with_text("Create")
        .build(ctx);
        let cancel = ButtonBuilder::new(
            WidgetBuilder::new()
                .with_width(100.0)
                .with_margin(Thickness::uniform(2.0)),
        )
        .with_text("Cancel")
        .build(ctx);

        let buttons = StackPanelBuilder::new(
            WidgetBuilder::new()
                .with_height(28.0)
                .with_horizontal_alignment(HorizontalAlignment::Right)
                .on_row(1)
                .with_child(create)
                .with_child(cancel),
        )
        .with_orientation(Orientation::Horizontal)
        .build(ctx);

        let grid = GridBuilder::new(
            WidgetBuilder::new()
                .with_margin(Thickness::uniform(2.0))
                .with_child(recent_list)
                .with_child(groups_scroll_viewer),
        )
        .add_column(Column::strict(200.0))
        .add_column(Column::stretch())
        .add_row(Row::stretch())
        .build(ctx);

        let content = GridBuilder::new(WidgetBuilder::new().with_child(grid).with_child(buttons))
            .add_column(Column::stretch())
            .add_row(Row::stretch())
            .add_row(Row::auto())
            .build(ctx);

        let window = WindowBuilder::new(WidgetBuilder::new().with_width(500.0).with_height(600.0))
            .with_content(content)
            .open(false)
            .with_title(WindowTitle::text("Entity Creator"))
            .build(ctx);

        Self {
            window,
            recent_list,
            groups_tree,
            items_map: FxHashMap::default(),
            create,
            cancel,
            selection: None,
            mode: EntityCreatorMode::default(),
        }
    }

    pub fn open(&mut self, mode: EntityCreatorMode, ui: &UserInterface) {
        self.mode = mode;
        ui.send(
            self.window,
            WindowMessage::Open {
                alignment: WindowAlignment::Center,
                modal: true,
                focus_content: true,
            },
        )
    }

    pub fn on_constructors_changed<N, C>(
        &mut self,
        ui: &mut UserInterface,
        constructors: &GraphNodeConstructorContainer<N, C>,
    ) {
        self.selection = None;
        ui.send(self.create, WidgetMessage::Enabled(false));
        ui.send(self.groups_tree, TreeRootMessage::Items(Vec::new()));
        self.items_map.clear();
        let constructors = constructors.map();
        let mut groups = FxHashMap::default();
        for (type_uuid, constructor) in constructors.iter() {
            for (variant_index, variant) in constructor.variants.iter().enumerate() {
                let item = make_tree_item(ui, &variant.name);
                self.items_map
                    .insert(item, ConstructorVariantId::new(type_uuid, variant_index));
                if constructor.group.is_empty() {
                    ui.send(self.groups_tree, TreeRootMessage::AddItem(item));
                } else {
                    let group = *groups.entry(constructor.group).or_insert_with(|| {
                        let group = make_tree_item(ui, constructor.group);
                        ui.send(self.groups_tree, TreeRootMessage::AddItem(group));
                        group
                    });
                    ui.send(group, TreeMessage::AddItem(item))
                }
            }
        }
    }

    fn handle_ui_message_internal<N, Ctx>(
        &mut self,
        constructors: &GraphNodeConstructorContainer<N, Ctx>,
        message: &UiMessage,
        ui: &UserInterface,
        ctx: &mut Ctx,
    ) -> Option<VariantResult<N>> {
        if let Some(TreeRootMessage::Select(selection)) = message.data_from(self.groups_tree) {
            if let Some(first) = selection.first() {
                let constructor_id = self.items_map.get(first);
                if let Some(constructor_id) = constructor_id {
                    self.selection = Some(constructor_id.clone());
                }
                let can_create = constructor_id.is_some();
                ui.send(self.create, WidgetMessage::Enabled(can_create));
            }
        } else if let Some(ButtonMessage::Click) = message.data_from(self.create) {
            if let Some(constructor_id) = self.selection.as_ref() {
                ui.send(self.window, WindowMessage::Close);
                return constructors.try_create_variant(constructor_id, ctx);
            }
        } else if let Some(ButtonMessage::Click) = message.data_from(self.cancel) {
            ui.send(self.window, WindowMessage::Close);
        }
        None
    }

    pub fn handle_ui_message_with_game_scene(
        &mut self,
        game_scene: &GameScene,
        engine: &mut Engine,
        message: &UiMessage,
        editor_selection: &Selection,
        sender: &MessageSender,
    ) {
        let ui = engine.user_interfaces.first();
        let graph = &mut engine.scenes[game_scene.scene].graph;
        if let Some(VariantResult::Owned(node)) = self.handle_ui_message_internal::<Node, Graph>(
            &engine.serialization_context.node_constructors,
            message,
            ui,
            graph,
        ) {
            match self.mode {
                EntityCreatorMode::CreateChild => {
                    if let Some(graph_selection) = editor_selection.as_graph() {
                        if let Some(parent) = graph_selection.nodes().first() {
                            sender.do_command(AddNodeCommand::new(node, *parent, true));
                        }
                    }
                }
                EntityCreatorMode::CreateParent => {
                    if let Some(graph_selection) = editor_selection.as_graph() {
                        if let Some(first) = graph_selection.nodes().first() {
                            let scene = &engine.scenes[game_scene.scene];

                            let position = game_scene
                                .camera_controller
                                .placement_position(&scene.graph, *first);

                            let first_ref = &scene.graph[*first];
                            let parent = if first_ref.parent().is_some() {
                                first_ref.parent()
                            } else {
                                game_scene.scene_content_root
                            };

                            let new_parent_handle = scene.graph.generate_free_handles(1)[0];
                            let mut commands = CommandGroup::from(vec![
                                Command::new(AddNodeCommand::new(node, parent, true)),
                                Command::new(LinkNodesCommand::new(*first, new_parent_handle)),
                            ]);

                            if parent == game_scene.scene_content_root {
                                commands.push(MoveNodeCommand::new(
                                    new_parent_handle,
                                    Vector3::default(),
                                    position,
                                ));
                            }

                            if *first == game_scene.scene_content_root {
                                commands.push(SetGraphRootCommand {
                                    root: new_parent_handle,
                                    link_scheme: Default::default(),
                                })
                            }
                            sender.do_command(commands);
                        }
                    }
                }
                EntityCreatorMode::CreateReplacement => {
                    if let Some(graph_selection) = editor_selection.as_graph() {
                        if let Some(first) = graph_selection.nodes().first() {
                            sender.do_command(ReplaceNodeCommand {
                                handle: *first,
                                node,
                            });
                        }
                    }
                }
            }
        }
    }

    pub fn handle_ui_message_with_ui_scene(
        &mut self,
        ui_scene: &mut UiScene,
        engine: &mut Engine,
        message: &UiMessage,
        editor_selection: &Selection,
        sender: &MessageSender,
    ) {
        let ui = engine.user_interfaces.first();
        if let Some(VariantResult::Handle(ui_node_handle)) = self
            .handle_ui_message_internal::<UiNode, UserInterface>(
                &engine.widget_constructors,
                message,
                ui,
                &mut ui_scene.ui,
            )
        {
            let sub_graph = ui_scene.ui.take_reserve_sub_graph(ui_node_handle);
            let parent = if let Some(selection) = editor_selection.as_ui() {
                selection.widgets.first().cloned().unwrap_or_default()
            } else {
                Handle::NONE
            };
            sender.do_command(AddWidgetCommand::new(sub_graph, parent, true));
        }
    }
}
