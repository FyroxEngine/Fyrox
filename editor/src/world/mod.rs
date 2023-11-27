use crate::{
    gui::make_image_button_with_tooltip,
    load_image,
    message::MessageSender,
    scene::{
        commands::{graph::LinkNodesCommand, ChangeSelectionCommand, CommandGroup, SceneCommand},
        EditorScene, Selection,
    },
    send_sync_message,
    utils::window_content,
    world::graph::{
        item::{SceneItem, SceneItemBuilder, SceneItemMessage},
        menu::ItemContextMenu,
        selection::GraphSelection,
    },
    Message, Mode, Settings,
};
use fyrox::{
    core::{
        color::Color,
        pool::{ErasedHandle, Handle},
        scope_profile,
    },
    engine::Engine,
    gui::{
        border::BorderBuilder,
        brush::Brush,
        button::{ButtonBuilder, ButtonMessage},
        check_box::{CheckBoxBuilder, CheckBoxMessage},
        decorator::{Decorator, DecoratorBuilder, DecoratorMessage},
        grid::{Column, GridBuilder, Row},
        message::{MessageDirection, UiMessage},
        scroll_viewer::{ScrollViewerBuilder, ScrollViewerMessage},
        searchbar::{SearchBarBuilder, SearchBarMessage},
        stack_panel::StackPanelBuilder,
        text::TextBuilder,
        tree::{
            TreeBuilder, TreeExpansionStrategy, TreeMessage, TreeRoot, TreeRootBuilder,
            TreeRootMessage,
        },
        ttf::{FontBuilder, SharedFont},
        widget::{WidgetBuilder, WidgetMessage},
        window::{WindowBuilder, WindowTitle},
        wrap_panel::WrapPanelBuilder,
        BuildContext, Orientation, RcUiNodeHandle, Thickness, UiNode, UserInterface,
        VerticalAlignment, BRUSH_BRIGHT_BLUE, BRUSH_PRIMARY,
    },
    scene::{graph::Graph, node::Node, Scene},
};
use rust_fuzzy_search::fuzzy_compare;
use std::{any::TypeId, cmp::Ordering, collections::HashMap};

pub mod graph;

pub struct WorldViewer {
    pub window: Handle<UiNode>,
    tree_root: Handle<UiNode>,
    sender: MessageSender,
    track_selection: Handle<UiNode>,
    search_bar: Handle<UiNode>,
    filter: String,
    stack: Vec<(Handle<UiNode>, Handle<Node>)>,
    /// Hack. Due to delayed execution of UI code we can't sync immediately after we
    /// did sync_to_model, instead we defer selection syncing to post_update() - at
    /// this moment UI is completely built and we can do syncing.
    pub sync_selection: bool,
    node_path: Handle<UiNode>,
    breadcrumbs: HashMap<Handle<UiNode>, Handle<UiNode>>,
    collapse_all: Handle<UiNode>,
    expand_all: Handle<UiNode>,
    locate_selection: Handle<UiNode>,
    scroll_view: Handle<UiNode>,
    item_context_menu: ItemContextMenu,
    node_to_view_map: HashMap<Handle<Node>, Handle<UiNode>>,
    small_font: SharedFont,
}

fn make_graph_node_item(
    node: &Node,
    handle: Handle<Node>,
    ctx: &mut BuildContext,
    context_menu: RcUiNodeHandle,
    sender: MessageSender,
    is_expanded: bool,
) -> Handle<UiNode> {
    let icon = if node.is_point_light() || node.is_directional_light() || node.is_spot_light() {
        load_image(include_bytes!("../../resources/light.png"))
    } else if node.is_joint() || node.is_joint2d() {
        load_image(include_bytes!("../../resources/joint.png"))
    } else if node.is_rigid_body() || node.is_rigid_body2d() {
        load_image(include_bytes!("../../resources/rigid_body.png"))
    } else if node.is_collider() || node.is_collider2d() {
        load_image(include_bytes!("../../resources/collider.png"))
    } else if node.is_sound() {
        load_image(include_bytes!("../../resources/sound_source.png"))
    } else {
        load_image(include_bytes!("../../resources/cube.png"))
    };

    SceneItemBuilder::new(
        TreeBuilder::new(
            WidgetBuilder::new()
                .with_margin(Thickness {
                    left: 1.0,
                    top: 1.0,
                    right: 0.0,
                    bottom: 0.0,
                })
                .with_context_menu(context_menu),
        )
        .with_expanded(is_expanded),
    )
    .with_text_brush(if node.resource().is_some() {
        Brush::Solid(Color::opaque(160, 160, 200))
    } else {
        Brush::Solid(fyrox::gui::COLOR_FOREGROUND)
    })
    .with_name(node.name().to_owned())
    .with_entity_handle(handle)
    .with_icon(icon)
    .build(ctx, sender)
}

fn tree_node(ui: &UserInterface, tree: Handle<UiNode>) -> Handle<Node> {
    ui.node(tree)
        .cast::<SceneItem>()
        .expect("Malformed scene item!")
        .entity_handle
}

fn colorize(handle: Handle<UiNode>, ui: &UserInterface, index: &mut usize) {
    let node = ui.node(handle);

    if let Some(decorator) = node.cast::<Decorator>() {
        if node.parent().is_some() {
            let new_brush = Brush::Solid(if *index % 2 == 0 {
                Color::opaque(50, 50, 50)
            } else {
                Color::opaque(60, 60, 60)
            });

            if decorator.normal_brush != new_brush {
                ui.send_message(DecoratorMessage::normal_brush(
                    handle,
                    MessageDirection::ToWidget,
                    new_brush,
                ));
            }

            *index += 1;
        }
    }

    for &item in node.children() {
        colorize(item, ui, index);
    }
}

fn fetch_expanded_state(
    node: Handle<Node>,
    editor_scene: &EditorScene,
    settings: &Settings,
) -> bool {
    editor_scene
        .path
        .as_ref()
        .and_then(|p| settings.scene_settings.get(p))
        .and_then(|s| s.node_infos.get(&node))
        .map_or(true, |i| i.is_expanded)
}

impl WorldViewer {
    pub fn new(ctx: &mut BuildContext, sender: MessageSender, settings: &Settings) -> Self {
        let small_font = SharedFont::new(
            FontBuilder::new()
                .with_height(11.0)
                .build_builtin()
                .unwrap(),
        );

        let tree_root;
        let node_path;
        let collapse_all;
        let expand_all;
        let locate_selection;
        let scroll_view;
        let track_selection;
        let search_bar = SearchBarBuilder::new(
            WidgetBuilder::new()
                .on_row(1)
                .with_margin(Thickness::uniform(1.0)),
        )
        .build(ctx);
        let size = 15.0;
        let window = WindowBuilder::new(WidgetBuilder::new().with_name("WorldOutliner"))
            .can_minimize(false)
            .with_title(WindowTitle::text("World Viewer"))
            .with_content(
                GridBuilder::new(
                    WidgetBuilder::new()
                        .with_child(
                            StackPanelBuilder::new(
                                WidgetBuilder::new()
                                    .with_margin(Thickness::uniform(1.0))
                                    .on_row(0)
                                    .with_child({
                                        collapse_all = make_image_button_with_tooltip(
                                            ctx,
                                            size,
                                            size,
                                            load_image(include_bytes!(
                                                "../../resources/collapse.png"
                                            )),
                                            "Collapse Everything",
                                        );
                                        collapse_all
                                    })
                                    .with_child({
                                        expand_all = make_image_button_with_tooltip(
                                            ctx,
                                            size,
                                            size,
                                            load_image(include_bytes!(
                                                "../../resources/expand.png"
                                            )),
                                            "Expand Everything",
                                        );
                                        expand_all
                                    })
                                    .with_child({
                                        locate_selection = make_image_button_with_tooltip(
                                            ctx,
                                            size,
                                            size,
                                            load_image(include_bytes!(
                                                "../../resources/locate.png"
                                            )),
                                            "Locate Selection",
                                        );
                                        locate_selection
                                    })
                                    .with_child({
                                        track_selection = CheckBoxBuilder::new(
                                            WidgetBuilder::new()
                                                .with_vertical_alignment(VerticalAlignment::Center)
                                                .with_margin(Thickness::uniform(1.0)),
                                        )
                                        .with_content(
                                            TextBuilder::new(WidgetBuilder::new())
                                                .with_vertical_text_alignment(
                                                    VerticalAlignment::Center,
                                                )
                                                .with_text("Track Selection")
                                                .build(ctx),
                                        )
                                        .checked(Some(settings.selection.track_selection))
                                        .build(ctx);
                                        track_selection
                                    }),
                            )
                            .with_orientation(Orientation::Horizontal)
                            .build(ctx),
                        )
                        .with_child(search_bar)
                        .with_child({
                            scroll_view = ScrollViewerBuilder::new(WidgetBuilder::new().on_row(2))
                                .with_content({
                                    tree_root =
                                        TreeRootBuilder::new(WidgetBuilder::new()).build(ctx);
                                    tree_root
                                })
                                .build(ctx);
                            scroll_view
                        })
                        .with_child({
                            node_path = WrapPanelBuilder::new(
                                WidgetBuilder::new()
                                    .on_row(3)
                                    .with_vertical_alignment(VerticalAlignment::Top),
                            )
                            .with_orientation(Orientation::Horizontal)
                            .build(ctx);
                            node_path
                        }),
                )
                .add_column(Column::stretch())
                .add_row(Row::strict(25.0))
                .add_row(Row::strict(22.0))
                .add_row(Row::stretch())
                .add_row(Row::auto())
                .build(ctx),
            )
            .build(ctx);

        let item_context_menu = ItemContextMenu::new(ctx);

        Self {
            search_bar,
            track_selection,
            window,
            sender,
            tree_root,
            node_path,
            stack: Default::default(),
            sync_selection: false,
            breadcrumbs: Default::default(),
            locate_selection,
            collapse_all,
            expand_all,
            scroll_view,
            item_context_menu,
            node_to_view_map: Default::default(),
            filter: Default::default(),
            small_font,
        }
    }

    pub fn sync_to_model(
        &mut self,
        editor_scene: &EditorScene,
        engine: &mut Engine,
        settings: &Settings,
    ) {
        scope_profile!();

        let scene = &mut engine.scenes[editor_scene.scene];
        let graph = &mut scene.graph;
        let ui = &mut engine.user_interface;

        self.sync_graph(ui, editor_scene, graph, settings);

        self.validate(editor_scene, engine);
    }

    fn build_breadcrumb(
        &mut self,
        name: &str,
        associated_item: Handle<UiNode>,
        ui: &mut UserInterface,
    ) {
        let ctx = &mut ui.build_ctx();

        let element = ButtonBuilder::new(WidgetBuilder::new().with_height(16.0))
            .with_back(
                DecoratorBuilder::new(BorderBuilder::new(
                    WidgetBuilder::new().with_foreground(BRUSH_PRIMARY),
                ))
                .with_normal_brush(BRUSH_PRIMARY)
                .with_hover_brush(BRUSH_BRIGHT_BLUE)
                .build(ctx),
            )
            .with_content(
                TextBuilder::new(WidgetBuilder::new())
                    .with_vertical_text_alignment(VerticalAlignment::Center)
                    .with_text(if self.breadcrumbs.is_empty() {
                        name.to_owned()
                    } else {
                        format!("{} >", name)
                    })
                    .with_font(self.small_font.clone())
                    .build(ctx),
            )
            .build(ctx);

        send_sync_message(
            ui,
            WidgetMessage::link_reverse(element, MessageDirection::ToWidget, self.node_path),
        );

        self.breadcrumbs.insert(element, associated_item);
    }

    fn clear_breadcrumbs(&mut self, ui: &UserInterface) {
        self.breadcrumbs.clear();
        for &child in ui.node(self.node_path).children() {
            send_sync_message(ui, WidgetMessage::remove(child, MessageDirection::ToWidget));
        }
    }

    fn update_breadcrumbs(
        &mut self,
        ui: &mut UserInterface,
        editor_scene: &EditorScene,
        scene: &Scene,
    ) {
        // Update breadcrumbs.
        self.clear_breadcrumbs(ui);

        if let Selection::Graph(selection) = &editor_scene.selection {
            if let Some(&first_selected) = selection.nodes().first() {
                let mut node_handle = first_selected;
                while node_handle.is_some() && node_handle != scene.graph.get_root() {
                    let node = &scene.graph[node_handle];

                    let view = ui.find_by_criteria_down(self.tree_root, &|n| {
                        n.cast::<SceneItem>()
                            .map(|i| i.entity_handle == node_handle)
                            .unwrap_or_default()
                    });
                    assert!(view.is_some());
                    self.build_breadcrumb(&format!("{}({})", node.name(), node_handle), view, ui);

                    node_handle = node.parent();
                }
            }
        }
    }

    fn sync_graph(
        &mut self,
        ui: &mut UserInterface,
        editor_scene: &EditorScene,
        graph: &Graph,
        settings: &Settings,
    ) {
        // Sync tree structure with graph structure.
        self.stack.clear();
        self.stack
            .push((self.tree_root, editor_scene.scene_content_root));
        while let Some((tree_handle, node_handle)) = self.stack.pop() {
            let node = &graph[node_handle];
            let ui_node = ui.node(tree_handle);

            if let Some(item) = ui_node.cast::<SceneItem>() {
                let child_count = node.children().len();
                let items = item.tree.items.clone();

                match child_count.cmp(&items.len()) {
                    Ordering::Less => {
                        for &item in items.iter() {
                            let child_node = tree_node(ui, item);
                            if !node.children().contains(&child_node) {
                                send_sync_message(
                                    ui,
                                    TreeMessage::remove_item(
                                        tree_handle,
                                        MessageDirection::ToWidget,
                                        item,
                                    ),
                                );
                                if let Some(existing_view) = self.node_to_view_map.get(&child_node)
                                {
                                    if *existing_view == item {
                                        self.node_to_view_map.remove(&child_node);
                                    }
                                }
                            } else {
                                self.stack.push((item, child_node));
                            }
                        }
                    }
                    Ordering::Equal => {
                        for &tree in items.iter() {
                            let child = tree_node(ui, tree);
                            self.stack.push((tree, child));
                        }
                    }
                    Ordering::Greater => {
                        for &child_handle in node.children() {
                            let mut found = false;
                            for &item in items.iter() {
                                let tree_node_handle = tree_node(ui, item);
                                if tree_node_handle == child_handle {
                                    self.stack.push((item, child_handle));
                                    found = true;
                                    break;
                                }
                            }
                            if !found {
                                let graph_node_item = make_graph_node_item(
                                    &graph[child_handle],
                                    child_handle,
                                    &mut ui.build_ctx(),
                                    self.item_context_menu.menu.clone(),
                                    self.sender.clone(),
                                    fetch_expanded_state(child_handle, editor_scene, settings),
                                );
                                send_sync_message(
                                    ui,
                                    TreeMessage::add_item(
                                        tree_handle,
                                        MessageDirection::ToWidget,
                                        graph_node_item,
                                    ),
                                );
                                self.node_to_view_map.insert(child_handle, graph_node_item);
                                self.stack.push((graph_node_item, child_handle));
                            }
                        }
                    }
                }
            } else if let Some(tree_root) = ui_node.cast::<TreeRoot>() {
                if tree_root.items.is_empty()
                    || tree_node(ui, tree_root.items[0]) != editor_scene.scene_content_root
                {
                    let new_root_item = make_graph_node_item(
                        node,
                        node_handle,
                        &mut ui.build_ctx(),
                        self.item_context_menu.menu.clone(),
                        self.sender.clone(),
                        fetch_expanded_state(node_handle, editor_scene, settings),
                    );
                    send_sync_message(
                        ui,
                        TreeRootMessage::items(
                            tree_handle,
                            MessageDirection::ToWidget,
                            vec![new_root_item],
                        ),
                    );
                    self.node_to_view_map.insert(node_handle, new_root_item);
                    self.stack.push((new_root_item, node_handle));
                } else {
                    self.stack.push((tree_root.items[0], node_handle));
                }
            }
        }

        // Sync items data.
        let mut stack = vec![self.tree_root];
        while let Some(handle) = stack.pop() {
            let ui_node = ui.node(handle);

            if let Some(item) = ui_node.cast::<SceneItem>() {
                if let Some(node) = graph.try_get(item.entity_handle) {
                    if item.name() != node.name() {
                        send_sync_message(
                            ui,
                            SceneItemMessage::name(
                                handle,
                                MessageDirection::ToWidget,
                                node.name().to_owned(),
                            ),
                        );
                    }

                    stack.extend_from_slice(&item.tree.items);
                }
            } else if let Some(root) = ui_node.cast::<TreeRoot>() {
                stack.extend_from_slice(&root.items)
            }
        }

        self.colorize(ui);

        self.node_to_view_map
            .retain(|k, v| graph.is_valid_handle(*k) && ui.try_get_node(*v).is_some());
    }

    pub fn colorize(&mut self, ui: &UserInterface) {
        let mut index = 0;
        colorize(self.tree_root, ui, &mut index);
    }

    fn apply_filter(&self, editor_scene: &EditorScene, ui: &UserInterface) {
        fn apply_filter_recursive(node: Handle<UiNode>, filter: &str, ui: &UserInterface) -> bool {
            let node_ref = ui.node(node);

            let mut is_any_match = false;
            for &child in node_ref.children() {
                is_any_match |= apply_filter_recursive(child, filter, ui)
            }

            let name = node_ref.cast::<SceneItem>().map(|i| i.name());

            if let Some(name) = name {
                is_any_match |= name.to_lowercase().contains(filter)
                    || fuzzy_compare(filter, name.to_lowercase().as_str()) >= 0.33;

                ui.send_message(WidgetMessage::visibility(
                    node,
                    MessageDirection::ToWidget,
                    is_any_match,
                ));
            }

            is_any_match
        }

        apply_filter_recursive(self.tree_root, &self.filter.to_lowercase(), ui);

        if self.filter.is_empty() {
            if let Selection::Graph(ref selection) = editor_scene.selection {
                if let Some(first) = selection.nodes().first() {
                    if let Some(view) = self.node_to_view_map.get(first) {
                        ui.send_message(ScrollViewerMessage::bring_into_view(
                            self.scroll_view,
                            MessageDirection::ToWidget,
                            *view,
                        ));
                    }
                }
            }
        }
    }

    pub fn set_filter(&mut self, filter: String, editor_scene: &EditorScene, ui: &UserInterface) {
        self.filter = filter;
        self.apply_filter(editor_scene, ui)
    }

    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        editor_scene: &mut EditorScene,
        engine: &Engine,
        settings: &mut Settings,
    ) {
        scope_profile!();

        self.item_context_menu.handle_ui_message(
            message,
            editor_scene,
            engine,
            &self.sender,
            settings,
        );

        if let Some(TreeRootMessage::Selected(selection)) = message.data::<TreeRootMessage>() {
            if message.destination() == self.tree_root
                && message.direction() == MessageDirection::FromWidget
            {
                self.handle_selection(selection, editor_scene, engine);
            }
        } else if let Some(&WidgetMessage::Drop(node)) = message.data::<WidgetMessage>() {
            self.handle_drop(engine, editor_scene, message.destination(), node);
        } else if let Some(ButtonMessage::Click) = message.data::<ButtonMessage>() {
            if let Some(&view) = self.breadcrumbs.get(&message.destination()) {
                if let Some(graph_node) = engine
                    .user_interface
                    .try_get_node(view)
                    .and_then(|n| n.cast::<SceneItem>())
                {
                    self.sender.do_scene_command(ChangeSelectionCommand::new(
                        Selection::Graph(GraphSelection::single_or_empty(graph_node.entity_handle)),
                        editor_scene.selection.clone(),
                    ));
                } else {
                    // Rest are not handled intentionally because other entities cannot have
                    // hierarchy and thus there is no need to change selection when we already
                    // have it selected.
                }
            } else if message.destination() == self.collapse_all {
                engine
                    .user_interface
                    .send_message(TreeRootMessage::collapse_all(
                        self.tree_root,
                        MessageDirection::ToWidget,
                    ));
            } else if message.destination() == self.expand_all {
                engine
                    .user_interface
                    .send_message(TreeRootMessage::expand_all(
                        self.tree_root,
                        MessageDirection::ToWidget,
                    ));
            } else if message.destination() == self.locate_selection {
                self.locate_selection(&editor_scene.selection, engine)
            }
        } else if let Some(CheckBoxMessage::Check(Some(value))) = message.data::<CheckBoxMessage>()
        {
            if message.destination() == self.track_selection {
                settings.selection.track_selection = *value;
                if *value {
                    self.locate_selection(&editor_scene.selection, engine);
                }
            }
        } else if let Some(SearchBarMessage::Text(text)) = message.data() {
            if message.destination() == self.search_bar
                && message.direction == MessageDirection::FromWidget
            {
                self.sender
                    .send(Message::SetWorldViewerFilter(text.clone()));
            }
        } else if let Some(TreeMessage::Expand { expand, .. }) = message.data() {
            if let Some(scene_view_item) = engine
                .user_interface
                .node(message.destination())
                .query_component::<SceneItem>()
            {
                if let Some(path) = editor_scene.path.as_ref() {
                    settings
                        .scene_settings
                        .entry(path.clone())
                        .or_default()
                        .node_infos
                        .entry(scene_view_item.entity_handle)
                        .or_default()
                        .is_expanded = *expand;
                }
            }
        }
    }

    pub fn try_locate_object(&self, type_id: TypeId, handle: ErasedHandle, engine: &Engine) {
        if type_id == TypeId::of::<Node>() {
            let selection = Selection::Graph(GraphSelection::single_or_empty(handle.into()));
            self.locate_selection(&selection, engine)
        } else {
            // TODO: Add more types here.
        }
    }

    fn locate_selection(&self, selection: &Selection, engine: &Engine) {
        let tree_to_focus = self.map_selection(selection, engine);

        if let Some(tree_to_focus) = tree_to_focus.first() {
            engine.user_interface.send_message(TreeMessage::expand(
                *tree_to_focus,
                MessageDirection::ToWidget,
                true,
                TreeExpansionStrategy::RecursiveAncestors,
            ));

            engine
                .user_interface
                .send_message(ScrollViewerMessage::bring_into_view(
                    self.scroll_view,
                    MessageDirection::ToWidget,
                    *tree_to_focus,
                ));
        }
    }

    fn handle_selection(
        &self,
        selection: &[Handle<UiNode>],
        editor_scene: &EditorScene,
        engine: &Engine,
    ) {
        let mut new_selection = Selection::None;
        for selected_item in selection {
            let selected_item_ref = engine.user_interface.node(*selected_item);

            if let Some(graph_node) = selected_item_ref.cast::<SceneItem>() {
                match new_selection {
                    Selection::None => {
                        new_selection = Selection::Graph(GraphSelection::single_or_empty(
                            graph_node.entity_handle,
                        ));
                    }
                    Selection::Graph(ref mut selection) => {
                        selection.insert_or_exclude(graph_node.entity_handle)
                    }
                    _ => (),
                }
            } else {
                return;
            }
        }

        if new_selection != editor_scene.selection {
            self.sender.do_scene_command(ChangeSelectionCommand::new(
                new_selection,
                editor_scene.selection.clone(),
            ));
        }
    }

    /// `target` - is a node at which `dropped` was dropped.
    /// `dropped` - is a node which was dropped at `target`.
    fn handle_drop(
        &self,
        engine: &Engine,
        editor_scene: &EditorScene,
        target: Handle<UiNode>,
        dropped: Handle<UiNode>,
    ) {
        let ui = &engine.user_interface;

        if ui.is_node_child_of(dropped, self.tree_root)
            && ui.is_node_child_of(target, self.tree_root)
            && dropped != target
        {
            if let (Some(child), Some(parent)) = (
                ui.node(dropped).cast::<SceneItem>(),
                ui.node(target).cast::<SceneItem>(),
            ) {
                if let Selection::Graph(ref selection) = editor_scene.selection {
                    if selection.nodes.contains(&child.entity_handle) {
                        let mut commands = Vec::new();

                        for &node_handle in selection.nodes.iter() {
                            // Make sure we won't create any loops - child must not have parent in its
                            // descendants.
                            let mut attach = true;
                            let graph = &engine.scenes[editor_scene.scene].graph;
                            let mut p = parent.entity_handle;
                            while p.is_some() {
                                if p == node_handle {
                                    attach = false;
                                    break;
                                }
                                p = graph[p].parent();
                            }

                            if attach {
                                commands.push(SceneCommand::new(LinkNodesCommand::new(
                                    node_handle,
                                    parent.entity_handle,
                                )));
                            }
                        }

                        if !commands.is_empty() {
                            self.sender.do_scene_command(CommandGroup::from(commands));
                        }
                    }
                }
            }
        }
    }

    fn map_selection(&self, selection: &Selection, engine: &Engine) -> Vec<Handle<UiNode>> {
        match selection {
            Selection::Graph(selection) => {
                map_selection(selection.nodes(), self.tree_root, &engine.user_interface)
            }
            _ => Default::default(),
        }
    }

    pub fn post_update(
        &mut self,
        editor_scene: &EditorScene,
        engine: &mut Engine,
        settings: &Settings,
    ) {
        // Hack. See `self.sync_selection` for details.
        if self.sync_selection {
            let trees = self.map_selection(&editor_scene.selection, engine);

            let ui = &mut engine.user_interface;
            send_sync_message(
                ui,
                TreeRootMessage::select(self.tree_root, MessageDirection::ToWidget, trees),
            );

            self.update_breadcrumbs(ui, editor_scene, &engine.scenes[editor_scene.scene]);
            if settings.selection.track_selection {
                self.locate_selection(&editor_scene.selection, engine);
            }

            self.sync_selection = false;
        }
    }

    pub fn clear(&mut self, ui: &UserInterface) {
        self.node_to_view_map.clear();
        self.clear_breadcrumbs(ui);
        ui.send_message(TreeRootMessage::items(
            self.tree_root,
            MessageDirection::ToWidget,
            vec![],
        ));
    }

    pub fn on_configure(&self, ui: &UserInterface, settings: &Settings) {
        ui.send_message(CheckBoxMessage::checked(
            self.track_selection,
            MessageDirection::ToWidget,
            Some(settings.selection.track_selection),
        ));
    }

    pub fn on_mode_changed(&mut self, ui: &UserInterface, mode: &Mode) {
        ui.send_message(WidgetMessage::enabled(
            window_content(self.window, ui),
            MessageDirection::ToWidget,
            mode.is_edit(),
        ));
    }

    pub fn validate(&self, editor_scene: &EditorScene, engine: &Engine) {
        let scene = &engine.scenes[editor_scene.scene];
        let graph = &scene.graph;
        for (node_handle, node) in graph.pair_iter() {
            if let Some(view) = self.node_to_view_map.get(&node_handle) {
                let result = node.validate(scene);

                let view_ref = engine
                    .user_interface
                    .node(*view)
                    .query_component::<SceneItem>()
                    .unwrap();

                if view_ref.warning_icon.is_none() && result.is_err()
                    || view_ref.warning_icon.is_some() && result.is_ok()
                {
                    send_sync_message(
                        &engine.user_interface,
                        SceneItemMessage::validate(*view, MessageDirection::ToWidget, result),
                    );
                }
            }
        }
    }
}

fn map_selection(
    selection: &[Handle<Node>],
    root_node: Handle<UiNode>,
    ui: &UserInterface,
) -> Vec<Handle<UiNode>> {
    selection
        .iter()
        .filter_map(|&handle| {
            let item = ui.find_by_criteria_down(root_node, &|n| {
                n.cast::<SceneItem>()
                    .map(|n| n.entity_handle == handle)
                    .unwrap_or_default()
            });
            if item.is_some() {
                Some(item)
            } else {
                None
            }
        })
        .collect()
}
