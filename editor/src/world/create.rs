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
        core::{algebra::Vector3, pool::Handle, ImmutableString},
        engine::Engine,
        fxhash::FxHashMap,
        graph::{
            constructor::{ConstructorVariantId, GraphNodeConstructorContainer, VariantResult},
            SceneGraph,
        },
        gui::{
            button::{Button, ButtonBuilder, ButtonMessage},
            grid::{Column, GridBuilder, Row},
            list_view::{ListView, ListViewBuilder, ListViewMessage},
            message::UiMessage,
            scroll_viewer::{ScrollViewer, ScrollViewerBuilder, ScrollViewerMessage},
            searchbar::{SearchBar, SearchBarBuilder, SearchBarMessage},
            stack_panel::StackPanelBuilder,
            text::TextBuilder,
            tree::{Tree, TreeBuilder, TreeMessage, TreeRoot, TreeRootBuilder, TreeRootMessage},
            utils,
            widget::{UserData, WidgetBuilder, WidgetMessage},
            window::{Window, WindowAlignment, WindowBuilder, WindowMessage, WindowTitle},
            BuildContext, HorizontalAlignment, Orientation, Thickness, UiNode, UserInterface,
        },
        scene::{graph::Graph, node::Node},
    },
    message::MessageSender,
    scene::{
        commands::graph::{
            AddNodeCommand, LinkNodesCommand, MoveNodeCommand, ReplaceNodeCommand,
            SetGraphRootCommand,
        },
        GameScene, Selection,
    },
    ui_scene::{commands::graph::AddWidgetCommand, UiScene},
};

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
    search_bar: Handle<SearchBar>,
    groups_scroll_viewer: Handle<ScrollViewer>,
}

type VariantName = ImmutableString;

#[derive(Clone)]
struct TreeData {
    variant_name: VariantName,
}

fn make_tree_item(ui: &mut UserInterface, variant_name: VariantName, text: &str) -> Handle<Tree> {
    let ctx = &mut ui.build_ctx();
    TreeBuilder::new(WidgetBuilder::new().with_user_data(UserData::new(TreeData { variant_name })))
        .with_content(
            TextBuilder::new(WidgetBuilder::new())
                .with_text(text)
                .build(ctx),
        )
        .build(ctx)
}

impl EntityCreator {
    const TITLE: &str = "Entity Creator";

    pub fn new(ctx: &mut BuildContext) -> Self {
        let search_bar = SearchBarBuilder::new(
            WidgetBuilder::new()
                .with_tab_index(Some(0))
                .on_row(0)
                .with_margin(Thickness::uniform(2.0)),
        )
        .build(ctx);

        let recent_list = ListViewBuilder::new(WidgetBuilder::new().on_column(0)).build(ctx);

        let groups_tree = TreeRootBuilder::new(WidgetBuilder::new()).build(ctx);

        let groups_scroll_viewer = ScrollViewerBuilder::new(WidgetBuilder::new().on_column(1))
            .with_content(groups_tree)
            .build(ctx);

        let create = ButtonBuilder::new(
            WidgetBuilder::new()
                .with_tab_index(Some(1))
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
                .on_row(2)
                .with_child(create)
                .with_child(cancel),
        )
        .with_orientation(Orientation::Horizontal)
        .build(ctx);

        let grid = GridBuilder::new(
            WidgetBuilder::new()
                .on_row(1)
                .with_margin(Thickness::uniform(2.0))
                .with_child(recent_list)
                .with_child(groups_scroll_viewer),
        )
        .add_column(Column::strict(200.0))
        .add_column(Column::stretch())
        .add_row(Row::stretch())
        .build(ctx);

        let content = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(search_bar)
                .with_child(grid)
                .with_child(buttons),
        )
        .add_column(Column::stretch())
        .add_row(Row::auto())
        .add_row(Row::stretch())
        .add_row(Row::auto())
        .build(ctx);

        let window = WindowBuilder::new(WidgetBuilder::new().with_width(500.0).with_height(600.0))
            .with_content(content)
            .open(false)
            .with_close_by_esc(true)
            .with_title(WindowTitle::text(Self::TITLE))
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
            search_bar,
            groups_scroll_viewer,
        }
    }

    pub fn open(&mut self, mode: EntityCreatorMode, ui: &UserInterface) {
        self.mode = mode;
        ui.send(
            self.window,
            WindowMessage::Open {
                alignment: WindowAlignment::Center,
                modal: true,
                focus_content: false,
            },
        );
        let title = match self.mode {
            EntityCreatorMode::CreateChild => "Create Child Node",
            EntityCreatorMode::CreateParent => "Create Parent Node",
            EntityCreatorMode::CreateReplacement => "Create Node Replacement",
        };
        ui.send(
            self.window,
            WindowMessage::Title(WindowTitle::text(format!("{} - {}", Self::TITLE, title))),
        );
        ui.send(self.search_bar, WidgetMessage::Focus);
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
                let item = make_tree_item(ui, variant.name.clone(), &variant.name);
                self.items_map
                    .insert(item, ConstructorVariantId::new(type_uuid, variant_index));
                if constructor.group.is_empty() {
                    ui.send(self.groups_tree, TreeRootMessage::AddItem(item));
                } else {
                    let group = *groups.entry(constructor.group).or_insert_with(|| {
                        let group = make_tree_item(
                            ui,
                            VariantName::new(constructor.group),
                            constructor.group,
                        );
                        ui.send(self.groups_tree, TreeRootMessage::AddItem(group));
                        group
                    });
                    ui.send(group, TreeMessage::AddItem(item))
                }
            }
        }
    }

    fn has_recent_item(&self, ui: &UserInterface, name: &str) -> bool {
        if let Ok(recent_list) = ui.try_get(self.recent_list) {
            for item in recent_list.items.iter() {
                if let Ok(item_ref) = ui.try_get(*item) {
                    if let Some(existing_name) = item_ref.user_data_cloned::<VariantName>() {
                        if existing_name.as_str() == name {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    fn on_recent_item_selected(&self, ui: &UserInterface, selection: &[usize]) {
        if let Some(name) = ui
            .try_get(self.recent_list)
            .ok()
            .and_then(|list| selection.first().and_then(|n| list.items.get(*n)))
            .and_then(|item| ui.try_get(*item).ok())
            .and_then(|item_ref| item_ref.user_data_cloned::<VariantName>())
        {
            let query = name.as_str().to_owned();
            ui.send(self.search_bar, SearchBarMessage::Text(query));
        }
    }

    fn on_create_clicked<N, Ctx>(
        &self,
        constructors: &GraphNodeConstructorContainer<N, Ctx>,
        ui: &mut UserInterface,
        ctx: &mut Ctx,
    ) -> Option<VariantResult<N>> {
        let constructor_id = self.selection.as_ref()?;
        ui.send(self.window, WindowMessage::Close);
        let variant = constructors.try_get_variant(constructor_id)?;
        if !self.has_recent_item(ui, variant.name.as_str()) {
            let recent_item = utils::make_dropdown_list_option_universal(
                &mut ui.build_ctx(),
                variant.name.as_str(),
                24.0,
                variant.name.clone(),
            );
            ui.send(self.recent_list, ListViewMessage::AddItem(recent_item));
        }
        Some((variant.constructor)(ctx))
    }

    fn on_filter_changed(&self, filter_text: &str, ui: &UserInterface) {
        fn apply_filter_recursive(
            node: Handle<UiNode>,
            filter: &str,
            ui: &UserInterface,
            first_match_selected: &mut bool,
            tree_root: Handle<TreeRoot>,
        ) -> bool {
            let node_ref = ui.node(node);

            let mut is_any_match = false;
            for &child in node_ref.children() {
                is_any_match |=
                    apply_filter_recursive(child, filter, ui, first_match_selected, tree_root)
            }

            if let Some(data) = node_ref
                .self_or_field_ref::<Tree>()
                .and_then(|n| n.user_data_cloned::<TreeData>())
            {
                is_any_match |= data.variant_name.to_lowercase().contains(filter);

                if !*first_match_selected && is_any_match {
                    ui.send(tree_root, TreeRootMessage::Select(vec![node.to_variant()]));
                    *first_match_selected = true;
                }

                ui.send(node, WidgetMessage::Visibility(is_any_match));
            }

            is_any_match
        }

        let mut first_match_selected = false;
        apply_filter_recursive(
            self.groups_tree.to_base(),
            &filter_text.to_lowercase(),
            ui,
            &mut first_match_selected,
            self.groups_tree,
        );

        // Bring first item of current selection in the view when clearing the filter.
        if filter_text.is_empty() {
            if let Some(first) = ui[self.groups_tree].selected.first() {
                ui.send(
                    self.groups_scroll_viewer,
                    ScrollViewerMessage::BringIntoView(first.to_base()),
                );
            }
        }
    }

    fn on_constructor_selected(&mut self, selection: &[Handle<Tree>], ui: &UserInterface) {
        if let Some(first) = selection.first() {
            let constructor_id = self.items_map.get(first);
            if let Some(constructor_id) = constructor_id {
                self.selection = Some(constructor_id.clone());
            }
            let can_create = constructor_id.is_some();
            ui.send(self.create, WidgetMessage::Enabled(can_create));
        }
    }

    fn handle_ui_message_internal<N, Ctx>(
        &mut self,
        constructors: &GraphNodeConstructorContainer<N, Ctx>,
        message: &UiMessage,
        ui: &mut UserInterface,
        ctx: &mut Ctx,
    ) -> Option<VariantResult<N>> {
        if let Some(TreeRootMessage::Select(selection)) = message.data_from(self.groups_tree) {
            self.on_constructor_selected(selection, ui)
        } else if let Some(ButtonMessage::Click) = message.data_from(self.create) {
            return self.on_create_clicked(constructors, ui, ctx);
        } else if let Some(ButtonMessage::Click) = message.data_from(self.cancel) {
            ui.send(self.window, WindowMessage::Close);
        } else if let Some(SearchBarMessage::Text(filter_text)) = message.data_from(self.search_bar)
        {
            self.on_filter_changed(filter_text, ui);
        } else if let Some(ListViewMessage::Selection(selection)) =
            message.data_from(self.recent_list)
        {
            self.on_recent_item_selected(ui, selection)
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
    ) -> Option<()> {
        let graph = engine
            .scenes
            .try_get_mut(game_scene.scene)
            .ok()
            .map(|s| &mut s.graph)?;
        if let Some(VariantResult::Owned(node)) = self.handle_ui_message_internal::<Node, Graph>(
            &engine.serialization_context.node_constructors,
            message,
            engine.user_interfaces.first_mut(),
            graph,
        ) {
            let graph_selection = editor_selection.as_graph()?;
            let first_selected_node = graph_selection.nodes().first().cloned()?;

            match self.mode {
                EntityCreatorMode::CreateChild => {
                    sender.do_command(AddNodeCommand::new(node, first_selected_node, true));
                }
                EntityCreatorMode::CreateParent => {
                    let position = game_scene
                        .camera_controller
                        .placement_position(graph, first_selected_node);

                    let first_ref = graph.try_get(first_selected_node).ok()?;
                    let parent = if first_ref.parent().is_some() {
                        first_ref.parent()
                    } else {
                        game_scene.scene_content_root
                    };

                    let new_parent_handle = graph.generate_free_handles(1)[0];
                    let mut commands = CommandGroup::from(vec![
                        Command::new(AddNodeCommand::new(node, parent, true)),
                        Command::new(LinkNodesCommand::new(
                            first_selected_node,
                            new_parent_handle,
                        )),
                    ]);

                    if parent == game_scene.scene_content_root {
                        commands.push(MoveNodeCommand::new(
                            new_parent_handle,
                            Vector3::default(),
                            position,
                        ));
                    }

                    if first_selected_node == game_scene.scene_content_root {
                        commands.push(SetGraphRootCommand {
                            root: new_parent_handle,
                            link_scheme: Default::default(),
                        })
                    }
                    sender.do_command(commands);
                }
                EntityCreatorMode::CreateReplacement => {
                    sender.do_command(ReplaceNodeCommand {
                        handle: first_selected_node,
                        node,
                    });
                }
            }
        }
        None
    }

    pub fn handle_ui_message_with_ui_scene(
        &mut self,
        ui_scene: &mut UiScene,
        engine: &mut Engine,
        message: &UiMessage,
        editor_selection: &Selection,
        sender: &MessageSender,
    ) {
        if let Some(VariantResult::Handle(ui_node_handle)) = self
            .handle_ui_message_internal::<UiNode, UserInterface>(
                &engine.widget_constructors,
                message,
                engine.user_interfaces.first_mut(),
                &mut ui_scene.ui,
            )
        {
            match self.mode {
                EntityCreatorMode::CreateChild => {
                    let sub_graph = ui_scene.ui.take_reserve_sub_graph(ui_node_handle);
                    let parent = if let Some(selection) = editor_selection.as_ui() {
                        selection.widgets.first().cloned().unwrap_or_default()
                    } else {
                        Handle::NONE
                    };
                    sender.do_command(AddWidgetCommand::new(sub_graph, parent, true));
                }
                EntityCreatorMode::CreateParent => {
                    // TODO
                }
                EntityCreatorMode::CreateReplacement => {
                    // TODO
                }
            }
        }
    }
}
