use crate::{
    load_image,
    scene::{
        commands::{graph::LinkNodesCommand, ChangeSelectionCommand},
        EditorScene, Selection,
    },
    send_sync_message,
    world::{
        graph::{
            item::{SceneItem, SceneItemBuilder, SceneItemMessage},
            menu::ItemContextMenu,
            selection::GraphSelection,
        },
        search::SearchBar,
        sound::selection::SoundSelection,
    },
    GameEngine, Message,
};
use rg3d::core::pool::ErasedHandle;
use rg3d::{
    core::{
        color::Color,
        pool::{Handle, Pool},
        scope_profile,
    },
    engine::Engine,
    gui::{
        brush::Brush,
        button::{Button, ButtonBuilder, ButtonMessage},
        check_box::{CheckBoxBuilder, CheckBoxMessage},
        decorator::{Decorator, DecoratorMessage},
        grid::{Column, GridBuilder, Row},
        message::{MessageDirection, UiMessage},
        scroll_viewer::{ScrollViewerBuilder, ScrollViewerMessage},
        stack_panel::StackPanelBuilder,
        text::TextBuilder,
        tree::{
            Tree, TreeBuilder, TreeExpansionStrategy, TreeMessage, TreeRoot, TreeRootBuilder,
            TreeRootMessage,
        },
        widget::{WidgetBuilder, WidgetMessage},
        window::{WindowBuilder, WindowTitle},
        BuildContext, HorizontalAlignment, Orientation, Thickness, UiNode, UserInterface,
        VerticalAlignment,
    },
    scene::{graph::Graph, node::Node, Scene},
    sound::{context::SoundContext, source::SoundSource},
};
use std::any::TypeId;
use std::{cmp::Ordering, collections::HashMap, sync::mpsc::Sender};

pub mod graph;
pub mod search;
pub mod sound;

pub struct WorldViewer {
    pub window: Handle<UiNode>,
    tree_root: Handle<UiNode>,
    graph_folder: Handle<UiNode>,
    sounds_folder: Handle<UiNode>,
    sender: Sender<Message>,
    track_selection: Handle<UiNode>,
    track_selection_state: bool,
    search_bar: SearchBar,
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
    sound_to_view_map: HashMap<Handle<SoundSource>, Handle<UiNode>>,
}

fn make_graph_node_item(
    node: &Node,
    handle: Handle<Node>,
    ctx: &mut BuildContext,
    context_menu: Handle<UiNode>,
) -> Handle<UiNode> {
    let icon = match node {
        Node::Light(_) => load_image(include_bytes!("../../resources/embed/light.png")),
        _ => load_image(include_bytes!("../../resources/embed/cube.png")),
    };

    SceneItemBuilder::new(TreeBuilder::new(
        WidgetBuilder::new()
            .with_margin(Thickness {
                left: 1.0,
                top: 1.0,
                right: 0.0,
                bottom: 0.0,
            })
            .with_context_menu(context_menu),
    ))
    .with_text_brush(if node.resource().is_some() {
        Brush::Solid(Color::opaque(160, 160, 200))
    } else {
        Brush::Solid(rg3d::gui::COLOR_FOREGROUND)
    })
    .with_name(node.name().to_owned())
    .with_entity_handle(handle)
    .with_icon(icon)
    .build(ctx)
}

fn tree_node(ui: &UserInterface, tree: Handle<UiNode>) -> Handle<Node> {
    if let Some(item) = ui.node(tree).cast::<SceneItem<Node>>() {
        return item.entity_handle;
    }
    unreachable!()
}

fn colorize(handle: Handle<UiNode>, ui: &UserInterface, index: &mut usize) {
    let node = ui.node(handle);

    if let Some(decorator) = node.cast::<Decorator>() {
        if node.parent().is_some() && ui.node(node.parent()).cast::<Button>().is_none() {
            let new_brush = Brush::Solid(if *index % 2 == 0 {
                Color::opaque(50, 50, 50)
            } else {
                Color::opaque(60, 60, 60)
            });

            if decorator.normal_brush() != &new_brush {
                ui.send_message(DecoratorMessage::normal_brush(
                    handle,
                    MessageDirection::ToWidget,
                    new_brush,
                ));
            }
        }
    }

    *index += 1;

    for &item in node.children() {
        colorize(item, ui, index);
    }
}

fn make_folder(ctx: &mut BuildContext, name: &str) -> Handle<UiNode> {
    TreeBuilder::new(WidgetBuilder::new())
        .with_content(
            TextBuilder::new(
                WidgetBuilder::new()
                    .with_margin(Thickness::left(5.0))
                    .with_foreground(Brush::Solid(Color::opaque(153, 217, 234))),
            )
            .with_vertical_text_alignment(VerticalAlignment::Center)
            .with_text(name)
            .build(ctx),
        )
        .build(ctx)
}

pub fn sync_pool<T, N, M>(
    folder: Handle<UiNode>,
    pool: &Pool<T>,
    ui: &mut UserInterface,
    selection: Option<&[Handle<T>]>,
    view_map: &mut HashMap<Handle<T>, Handle<UiNode>>,
    mut make_view: M,
    mut make_name: N,
) -> Vec<Handle<UiNode>>
where
    T: 'static,
    N: FnMut(Handle<T>) -> String,
    M: FnMut(&mut UserInterface, Handle<T>, &T) -> Handle<UiNode>,
{
    let folder_items = ui
        .node(folder)
        .cast::<Tree>()
        .unwrap()
        .items()
        .iter()
        .cloned()
        .filter(|i| ui.node(*i).cast::<SceneItem<T>>().is_some())
        .collect::<Vec<_>>();

    let pool_count = usize::try_from(pool.alive_count()).expect("overflowed usize");
    match pool_count.cmp(&folder_items.len()) {
        Ordering::Less => {
            // An entity was removed.
            for &item in folder_items.iter() {
                let entity_handle = ui.node(item).cast::<SceneItem<T>>().unwrap().entity_handle;

                if pool.pair_iter().all(|(h, _)| h != entity_handle) {
                    let removed = view_map.remove(&entity_handle);

                    assert!(removed.is_some());

                    send_sync_message(
                        ui,
                        TreeMessage::remove_item(folder, MessageDirection::ToWidget, item),
                    );
                }
            }
        }
        Ordering::Greater => {
            // An entity was added.
            for (handle, elem) in pool.pair_iter() {
                if folder_items
                    .iter()
                    .all(|i| ui.node(*i).cast::<SceneItem<T>>().unwrap().entity_handle != handle)
                {
                    let view = (make_view)(ui, handle, elem);

                    let previous = view_map.insert(handle, view);

                    assert!(previous.is_none());

                    send_sync_message(
                        ui,
                        TreeMessage::add_item(folder, MessageDirection::ToWidget, view),
                    );
                }
            }
        }
        _ => (),
    }

    let mut selected_items = Vec::new();

    // Sync selection.
    if let Some(selection) = selection {
        for selected in selection {
            if let Some(associated_item) = ui
                .node(folder)
                .cast::<Tree>()
                .unwrap()
                .items()
                .iter()
                .cloned()
                .find(|i| ui.node(*i).cast::<SceneItem<T>>().unwrap().entity_handle == *selected)
            {
                selected_items.push(associated_item)
            }
        }
    }

    // Sync names.
    for item in ui.node(folder).cast::<Tree>().unwrap().items() {
        let item_ref = ui.node(*item).cast::<SceneItem<T>>().unwrap();
        let entity_handle = item_ref.entity_handle;
        if pool.is_valid_handle(entity_handle) {
            let new_name = (make_name)(entity_handle);
            if new_name != item_ref.name() {
                ui.send_message(SceneItemMessage::name(
                    *item,
                    MessageDirection::ToWidget,
                    new_name,
                ));
            }
        }
    }

    selected_items
}

impl WorldViewer {
    pub fn new(ctx: &mut BuildContext, sender: Sender<Message>) -> Self {
        let track_selection_state = true;
        let tree_root;
        let node_path;
        let collapse_all;
        let expand_all;
        let locate_selection;
        let scroll_view;
        let track_selection;
        let search_bar = SearchBar::new(ctx);
        let graph_folder = make_folder(ctx, "Scene Graph");
        let sounds_folder = make_folder(ctx, "Sounds");
        let window = WindowBuilder::new(WidgetBuilder::new())
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
                                        collapse_all = ButtonBuilder::new(
                                            WidgetBuilder::new()
                                                .with_margin(Thickness::uniform(1.0)),
                                        )
                                        .with_text("Collapse All")
                                        .build(ctx);
                                        collapse_all
                                    })
                                    .with_child({
                                        expand_all = ButtonBuilder::new(
                                            WidgetBuilder::new()
                                                .with_margin(Thickness::uniform(1.0)),
                                        )
                                        .with_text("Expand All")
                                        .build(ctx);
                                        expand_all
                                    })
                                    .with_child({
                                        locate_selection = ButtonBuilder::new(
                                            WidgetBuilder::new()
                                                .with_margin(Thickness::uniform(1.0)),
                                        )
                                        .with_text("Locate Selection")
                                        .build(ctx);
                                        locate_selection
                                    })
                                    .with_child({
                                        track_selection = CheckBoxBuilder::new(
                                            WidgetBuilder::new()
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
                                        .checked(Some(track_selection_state))
                                        .build(ctx);
                                        track_selection
                                    }),
                            )
                            .with_orientation(Orientation::Horizontal)
                            .build(ctx),
                        )
                        .with_child(search_bar.container)
                        .with_child(
                            TextBuilder::new(
                                WidgetBuilder::new()
                                    .on_row(2)
                                    .on_column(0)
                                    .with_opacity(Some(0.4)),
                            )
                            .with_text("Breadcrumbs")
                            .with_vertical_text_alignment(VerticalAlignment::Center)
                            .with_horizontal_text_alignment(HorizontalAlignment::Center)
                            .build(ctx),
                        )
                        .with_child(
                            ScrollViewerBuilder::new(WidgetBuilder::new().on_row(2))
                                .with_content({
                                    node_path = StackPanelBuilder::new(WidgetBuilder::new())
                                        .with_orientation(Orientation::Horizontal)
                                        .build(ctx);
                                    node_path
                                })
                                .build(ctx),
                        )
                        .with_child({
                            scroll_view = ScrollViewerBuilder::new(WidgetBuilder::new().on_row(3))
                                .with_content({
                                    tree_root = TreeRootBuilder::new(WidgetBuilder::new())
                                        .with_items(vec![graph_folder, sounds_folder])
                                        .build(ctx);
                                    tree_root
                                })
                                .build(ctx);
                            scroll_view
                        }),
                )
                .add_column(Column::stretch())
                .add_row(Row::strict(24.0))
                .add_row(Row::strict(24.0))
                .add_row(Row::strict(24.0))
                .add_row(Row::stretch())
                .build(ctx),
            )
            .build(ctx);

        let item_context_menu = ItemContextMenu::new(ctx);

        Self {
            search_bar,
            track_selection,
            track_selection_state,
            window,
            sender,
            tree_root,
            graph_folder,
            node_path,
            stack: Default::default(),
            sync_selection: false,
            breadcrumbs: Default::default(),
            locate_selection,
            collapse_all,
            expand_all,
            scroll_view,
            item_context_menu,
            sounds_folder,
            node_to_view_map: Default::default(),
            sound_to_view_map: Default::default(),
            filter: Default::default(),
        }
    }

    pub fn sync_to_model(&mut self, editor_scene: &EditorScene, engine: &mut GameEngine) {
        scope_profile!();

        let scene = &mut engine.scenes[editor_scene.scene];
        let graph = &mut scene.graph;
        let ui = &mut engine.user_interface;

        let mut selected_items = Vec::new();

        selected_items.extend(self.sync_graph(ui, editor_scene, graph));
        selected_items.extend(self.sync_sounds(ui, editor_scene, scene.sound_context.clone()));

        if !selected_items.is_empty() {
            send_sync_message(
                ui,
                TreeRootMessage::select(self.tree_root, MessageDirection::ToWidget, selected_items),
            );
        }
    }

    fn build_breadcrumb(
        &mut self,
        name: &str,
        associated_item: Handle<UiNode>,
        ui: &mut UserInterface,
    ) {
        let element = ButtonBuilder::new(WidgetBuilder::new().with_margin(Thickness::uniform(1.0)))
            .with_text(name)
            .build(&mut ui.build_ctx());

        send_sync_message(
            ui,
            WidgetMessage::link_reverse(element, MessageDirection::ToWidget, self.node_path),
        );

        self.breadcrumbs.insert(element, associated_item);
    }

    fn update_breadcrumbs(
        &mut self,
        ui: &mut UserInterface,
        editor_scene: &EditorScene,
        scene: &Scene,
    ) {
        // Update breadcrumbs.
        self.breadcrumbs.clear();
        for &child in ui.node(self.node_path).children() {
            send_sync_message(ui, WidgetMessage::remove(child, MessageDirection::ToWidget));
        }

        match &editor_scene.selection {
            Selection::Graph(selection) => {
                if let Some(&first_selected) = selection.nodes().first() {
                    let mut item = first_selected;
                    while item.is_some() {
                        let node = &scene.graph[item];

                        let view = ui.find_by_criteria_down(self.graph_folder, &|n| {
                            n.cast::<SceneItem<Node>>()
                                .map(|i| i.entity_handle == item)
                                .unwrap_or_default()
                        });
                        assert!(view.is_some());
                        self.build_breadcrumb(node.name(), view, ui);

                        item = node.parent();
                    }
                }
            }
            Selection::Navmesh(_) => {
                // TODO
            }
            Selection::Sound(selection) => {
                if let Some(&first_selected) = selection.sources().first() {
                    let view = ui.find_by_criteria_down(self.sounds_folder, &|n| {
                        n.cast::<SceneItem<SoundSource>>()
                            .map(|i| i.entity_handle == first_selected)
                            .unwrap_or_default()
                    });
                    assert!(view.is_some());
                    self.build_breadcrumb(
                        scene.sound_context.state().source(first_selected).name(),
                        view,
                        ui,
                    );
                }
            }
            Selection::None => {}
        }
    }

    fn sync_sounds(
        &mut self,
        ui: &mut UserInterface,
        editor_scene: &EditorScene,
        ctx: SoundContext,
    ) -> Vec<Handle<UiNode>> {
        let ctx = ctx.state();

        sync_pool(
            self.sounds_folder,
            ctx.sources(),
            ui,
            if let Selection::Sound(ref s) = editor_scene.selection {
                Some(&s.sources)
            } else {
                None
            },
            &mut self.sound_to_view_map,
            |ui, handle, _| {
                SceneItemBuilder::<SoundSource>::new(TreeBuilder::new(WidgetBuilder::new()))
                    .with_name(ctx.source(handle).name_owned())
                    .with_icon(load_image(include_bytes!(
                        "../../resources/embed/sound_source.png"
                    )))
                    .with_entity_handle(handle)
                    .build(&mut ui.build_ctx())
            },
            |s| ctx.source(s).name_owned(),
        )
    }

    fn sync_graph(
        &mut self,
        ui: &mut UserInterface,
        editor_scene: &EditorScene,
        graph: &Graph,
    ) -> Vec<Handle<UiNode>> {
        let mut selected_items = Vec::new();

        // Sync tree structure with graph structure.
        self.stack.clear();
        self.stack.push((self.graph_folder, graph.get_root()));
        while let Some((tree_handle, node_handle)) = self.stack.pop() {
            // Hide all editor nodes.
            if node_handle == editor_scene.root {
                continue;
            }
            let node = &graph[node_handle];
            let ui_node = ui.node(tree_handle);

            if let Some(item) = ui_node.cast::<SceneItem<Node>>() {
                // Since we are filtering out editor stuff from world viewer, we must
                // correctly count children, excluding editor nodes.
                let mut child_count = 0;
                for &child in node.children() {
                    if child != editor_scene.root {
                        child_count += 1;
                    }
                }

                // We're interested only scene graph child items.
                // Such filtering is needed because we can have links as children in UI.
                let items = item
                    .tree
                    .items()
                    .iter()
                    .cloned()
                    .filter(|i| ui.node(*i).cast::<SceneItem<Node>>().is_some())
                    .collect::<Vec<_>>();

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
                                self.node_to_view_map.remove(&child_node);
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
                            // Hide all editor nodes.
                            if child_handle == editor_scene.root {
                                continue;
                            }
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
                                    self.item_context_menu.menu,
                                );
                                send_sync_message(
                                    ui,
                                    TreeMessage::add_item(
                                        tree_handle,
                                        MessageDirection::ToWidget,
                                        graph_node_item,
                                    ),
                                );
                                if let Selection::Graph(selection) = &editor_scene.selection {
                                    if selection.contains(child_handle) {
                                        selected_items.push(graph_node_item);
                                    }
                                }
                                self.node_to_view_map.insert(child_handle, graph_node_item);
                                self.stack.push((graph_node_item, child_handle));
                            }
                        }
                    }
                }
            } else if let Some(folder) = ui_node.cast::<Tree>() {
                if folder.items().is_empty() {
                    let graph_node_item = make_graph_node_item(
                        node,
                        node_handle,
                        &mut ui.build_ctx(),
                        self.item_context_menu.menu,
                    );
                    send_sync_message(
                        ui,
                        TreeMessage::add_item(
                            tree_handle,
                            MessageDirection::ToWidget,
                            graph_node_item,
                        ),
                    );
                    self.node_to_view_map.insert(node_handle, graph_node_item);
                    self.stack.push((graph_node_item, node_handle));
                } else {
                    self.stack.push((folder.items()[0], node_handle));
                }
            }
        }

        // Sync items data.
        let mut stack = vec![self.tree_root];
        while let Some(handle) = stack.pop() {
            let ui_node = ui.node(handle);

            if let Some(item) = ui_node.cast::<SceneItem<Node>>() {
                if graph.is_valid_handle(item.entity_handle) {
                    let node = &graph[item.entity_handle];

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

                    stack.extend_from_slice(item.tree.items());
                }
            } else if let Some(root) = ui_node.cast::<TreeRoot>() {
                stack.extend_from_slice(root.items())
            }
        }

        self.colorize(ui);

        self.node_to_view_map
            .retain(|k, v| graph.is_valid_handle(*k) && ui.try_get_node(*v).is_some());

        selected_items
    }

    pub fn colorize(&mut self, ui: &UserInterface) {
        let mut index = 0;
        colorize(self.tree_root, ui, &mut index);
    }

    fn apply_filter(&self, ui: &UserInterface) {
        fn apply_filter_recursive(node: Handle<UiNode>, filter: &str, ui: &UserInterface) -> bool {
            let node_ref = ui.node(node);

            let mut is_any_match = false;
            for &child in node_ref.children() {
                is_any_match |= apply_filter_recursive(child, filter, ui)
            }

            // TODO: It is very easy to forget to add a new condition here if a new type
            // of a scene item is added. Find a way of doing this in a better way.
            // Also due to very simple RTTI in Rust, it becomes boilerplate-ish very quick.
            let name = node_ref.cast::<SceneItem<Node>>().map(|i| i.name());

            if let Some(name) = name {
                is_any_match |= name.contains(filter);

                ui.send_message(WidgetMessage::visibility(
                    node,
                    MessageDirection::ToWidget,
                    is_any_match,
                ));
            }

            is_any_match
        }

        apply_filter_recursive(self.tree_root, &self.filter, ui);
    }

    pub fn set_filter(&mut self, filter: String, ui: &UserInterface) {
        self.filter = filter;
        self.apply_filter(ui)
    }

    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        editor_scene: &mut EditorScene,
        engine: &GameEngine,
    ) {
        scope_profile!();

        self.item_context_menu
            .handle_ui_message(message, editor_scene, engine, &self.sender);
        self.search_bar
            .handle_ui_message(message, &engine.user_interface, &self.sender);

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
                    .and_then(|n| n.cast::<SceneItem<Node>>())
                {
                    self.sender
                        .send(Message::do_scene_command(ChangeSelectionCommand::new(
                            Selection::Graph(GraphSelection::single_or_empty(
                                graph_node.entity_handle,
                            )),
                            editor_scene.selection.clone(),
                        )))
                        .unwrap();
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
                self.track_selection_state = *value;
                if *value {
                    self.locate_selection(&editor_scene.selection, engine);
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

            if let Some(graph_node) = selected_item_ref.cast::<SceneItem<Node>>() {
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
            } else if let Some(sound) = selected_item_ref.cast::<SceneItem<SoundSource>>() {
                match new_selection {
                    Selection::None => {
                        new_selection = Selection::Sound(SoundSelection {
                            sources: vec![sound.entity_handle],
                        });
                    }
                    Selection::Sound(ref mut selection) => {
                        selection.sources.push(sound.entity_handle)
                    }
                    _ => (),
                }
            } else {
                return;
            }
        }

        if new_selection != editor_scene.selection {
            self.sender
                .send(Message::do_scene_command(ChangeSelectionCommand::new(
                    new_selection,
                    editor_scene.selection.clone(),
                )))
                .unwrap();
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
                ui.node(dropped).cast::<SceneItem<Node>>(),
                ui.node(target).cast::<SceneItem<Node>>(),
            ) {
                // Make sure we won't create any loops - child must not have parent in its
                // descendants.
                let mut attach = true;
                let graph = &engine.scenes[editor_scene.scene].graph;
                let mut p = parent.entity_handle;
                while p.is_some() {
                    if p == child.entity_handle {
                        attach = false;
                        break;
                    }
                    p = graph[p].parent();
                }

                if attach {
                    self.sender
                        .send(Message::do_scene_command(LinkNodesCommand::new(
                            child.entity_handle,
                            parent.entity_handle,
                        )))
                        .unwrap();
                }
            }
        }
    }

    fn map_selection(&self, selection: &Selection, engine: &GameEngine) -> Vec<Handle<UiNode>> {
        let ui = &engine.user_interface;
        match selection {
            Selection::Graph(selection) => {
                map_selection(selection.nodes(), self.graph_folder, &engine.user_interface)
            }
            Selection::Sound(selection) => {
                map_selection(selection.sources(), self.sounds_folder, ui)
            }
            Selection::None | Selection::Navmesh(_) => Default::default(),
        }
    }

    pub fn post_update(&mut self, editor_scene: &EditorScene, engine: &mut GameEngine) {
        // Hack. See `self.sync_selection` for details.
        if self.sync_selection {
            let trees = self.map_selection(&editor_scene.selection, engine);

            let ui = &mut engine.user_interface;
            send_sync_message(
                ui,
                TreeRootMessage::select(self.tree_root, MessageDirection::ToWidget, trees),
            );

            self.update_breadcrumbs(ui, editor_scene, &engine.scenes[editor_scene.scene]);
            if self.track_selection_state {
                self.locate_selection(&editor_scene.selection, engine);
            }

            self.sync_selection = false;
        }
    }

    pub fn clear(&mut self, ui: &mut UserInterface) {
        self.node_to_view_map.clear();
        self.sound_to_view_map.clear();

        for folder in [self.graph_folder, self.sounds_folder] {
            ui.send_message(TreeMessage::set_items(
                folder,
                MessageDirection::ToWidget,
                vec![],
            ));
        }
    }
}

fn map_selection<T>(
    selection: &[Handle<T>],
    folder: Handle<UiNode>,
    ui: &UserInterface,
) -> Vec<Handle<UiNode>>
where
    T: 'static,
{
    selection
        .iter()
        .filter_map(|&handle| {
            let item = ui.find_by_criteria_down(folder, &|n| {
                n.cast::<SceneItem<T>>()
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
