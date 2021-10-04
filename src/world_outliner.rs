use crate::gui::SceneItemMessage;
use crate::scene::commands::SceneCommand;
use crate::{
    load_image,
    scene::{
        commands::{
            graph::{LinkNodesCommand, SetVisibleCommand},
            make_delete_selection_command, ChangeSelectionCommand,
        },
        EditorScene, GraphSelection, Selection,
    },
    send_sync_message, GameEngine, Message,
};
use rg3d::gui::message::UiMessage;
use rg3d::gui::tree::TreeRoot;
use rg3d::gui::widget::Widget;
use rg3d::gui::{BuildContext, UiNode, UserInterface};
use rg3d::{
    core::{algebra::Vector2, pool::Handle, scope_profile},
    engine::resource_manager::ResourceManager,
    gui::{
        brush::Brush,
        button::ButtonBuilder,
        core::color::Color,
        draw::{DrawingContext, SharedTexture},
        grid::{Column, GridBuilder, Row},
        image::ImageBuilder,
        menu::{MenuItemBuilder, MenuItemContent},
        message::{
            ButtonMessage, DecoratorMessage, MenuItemMessage, MessageDirection, OsEvent,
            ScrollViewerMessage, TextMessage, TreeExpansionStrategy, TreeMessage, TreeRootMessage,
            UiMessageData, WidgetMessage,
        },
        popup::PopupBuilder,
        scroll_viewer::ScrollViewerBuilder,
        stack_panel::StackPanelBuilder,
        text::TextBuilder,
        tree::{Tree, TreeBuilder, TreeRootBuilder},
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowTitle},
        Control, HorizontalAlignment, NodeHandleMapping, Orientation, Thickness, VerticalAlignment,
    },
    scene::node::Node,
};
use std::{
    collections::HashMap,
    fmt::{Debug, Formatter},
    ops::{Deref, DerefMut},
    sync::mpsc::Sender,
};

struct ItemContextMenu {
    menu: Handle<UiNode>,
    delete_selection: Handle<UiNode>,
    copy_selection: Handle<UiNode>,
}

impl ItemContextMenu {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let delete_selection;
        let copy_selection;

        let menu = PopupBuilder::new(WidgetBuilder::new().with_visibility(false))
            .with_content(
                StackPanelBuilder::new(
                    WidgetBuilder::new()
                        .with_child({
                            delete_selection = MenuItemBuilder::new(
                                WidgetBuilder::new().with_min_size(Vector2::new(120.0, 20.0)),
                            )
                            .with_content(MenuItemContent::Text {
                                text: "Delete Selection",
                                shortcut: "Del",
                                icon: Default::default(),
                            })
                            .build(ctx);
                            delete_selection
                        })
                        .with_child({
                            copy_selection = MenuItemBuilder::new(
                                WidgetBuilder::new().with_min_size(Vector2::new(120.0, 20.0)),
                            )
                            .with_content(MenuItemContent::Text {
                                text: "Copy Selection",
                                shortcut: "Ctrl+C",
                                icon: Default::default(),
                            })
                            .build(ctx);
                            copy_selection
                        }),
                )
                .build(ctx),
            )
            .build(ctx);

        Self {
            menu,
            delete_selection,
            copy_selection,
        }
    }

    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        editor_scene: &mut EditorScene,
        engine: &GameEngine,
        sender: &Sender<Message>,
    ) {
        scope_profile!();

        if let UiMessageData::MenuItem(MenuItemMessage::Click) = message.data() {
            if message.destination() == self.delete_selection {
                sender
                    .send(Message::DoSceneCommand(make_delete_selection_command(
                        editor_scene,
                        engine,
                    )))
                    .unwrap();
            } else if message.destination() == self.copy_selection {
                if let Selection::Graph(graph_selection) = &editor_scene.selection {
                    editor_scene.clipboard.fill_from_selection(
                        graph_selection,
                        editor_scene.scene,
                        &editor_scene.physics,
                        engine,
                    );
                }
            }
        }
    }
}

pub struct WorldOutliner {
    pub window: Handle<UiNode>,
    root: Handle<UiNode>,
    sender: Sender<Message>,
    stack: Vec<(Handle<UiNode>, Handle<Node>)>,
    /// Hack. Due to delayed execution of UI code we can't sync immediately after we
    /// did sync_to_model, instead we defer selection syncing to post_update() - at
    /// this moment UI is completely built and we can do syncing.
    pub sync_selection: bool,
    node_path: Handle<UiNode>,
    breadcrumbs: HashMap<Handle<UiNode>, Handle<Node>>,
    collapse_all: Handle<UiNode>,
    expand_all: Handle<UiNode>,
    locate_selection: Handle<UiNode>,
    scroll_view: Handle<UiNode>,
    item_context_menu: ItemContextMenu,
}

#[derive(Clone)]
pub struct SceneItem {
    tree: Tree,
    text_name: Handle<UiNode>,
    node: Handle<Node>,
    visibility_toggle: Handle<UiNode>,
    sender: Sender<Message>,
    visibility: bool,
    resource_manager: ResourceManager,
}

impl Debug for SceneItem {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "SceneItem")
    }
}

impl Deref for SceneItem {
    type Target = Widget;

    fn deref(&self) -> &Self::Target {
        &self.tree
    }
}

impl DerefMut for SceneItem {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.tree
    }
}

impl Control for SceneItem {
    fn resolve(&mut self, node_map: &NodeHandleMapping) {
        self.tree.resolve(node_map);
        node_map.resolve(&mut self.text_name);
    }

    fn measure_override(&self, ui: &UserInterface, available_size: Vector2<f32>) -> Vector2<f32> {
        self.tree.measure_override(ui, available_size)
    }

    fn arrange_override(&self, ui: &UserInterface, final_size: Vector2<f32>) -> Vector2<f32> {
        self.tree.arrange_override(ui, final_size)
    }

    fn draw(&self, drawing_context: &mut DrawingContext) {
        self.tree.draw(drawing_context);
    }

    fn update(&mut self, dt: f32) {
        self.tree.update(dt);
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.tree.handle_routed_message(ui, message);

        match message.data() {
            UiMessageData::Button(msg) => {
                if message.destination() == self.visibility_toggle {
                    if let ButtonMessage::Click = msg {
                        let command =
                            SceneCommand::new(SetVisibleCommand::new(self.node, !self.visibility));
                        self.sender.send(Message::DoSceneCommand(command)).unwrap();
                    }
                }
            }
            UiMessageData::User(msg) => {
                if let Some(msg) = msg.cast::<SceneItemMessage>() {
                    match msg {
                        &SceneItemMessage::NodeVisibility(visibility) => {
                            if self.visibility != visibility
                                && message.destination() == self.handle()
                            {
                                self.visibility = visibility;
                                let image = if visibility {
                                    load_image(include_bytes!("../resources/embed/visible.png"))
                                } else {
                                    load_image(include_bytes!("../resources/embed/invisible.png"))
                                };
                                let image = ImageBuilder::new(WidgetBuilder::new())
                                    .with_opt_texture(image)
                                    .build(&mut ui.build_ctx());
                                ui.send_message(ButtonMessage::content(
                                    self.visibility_toggle,
                                    MessageDirection::ToWidget,
                                    image,
                                ));
                            }
                        }
                        &SceneItemMessage::Order(order) => {
                            if message.destination() == self.handle() {
                                ui.send_message(DecoratorMessage::normal_brush(
                                    self.tree.back(),
                                    MessageDirection::ToWidget,
                                    Brush::Solid(if order {
                                        Color::opaque(50, 50, 50)
                                    } else {
                                        Color::opaque(60, 60, 60)
                                    }),
                                ));
                            }
                        }
                        SceneItemMessage::Name(name) => {
                            if message.destination() == self.handle() {
                                let name = format!(
                                    "{} ({}:{})",
                                    name,
                                    self.node.index(),
                                    self.node.generation()
                                );

                                ui.send_message(TextMessage::text(
                                    self.text_name,
                                    MessageDirection::ToWidget,
                                    name,
                                ));
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn preview_message(&self, ui: &UserInterface, message: &mut UiMessage) {
        self.tree.preview_message(ui, message);
    }

    fn handle_os_event(
        &mut self,
        self_handle: Handle<UiNode>,
        ui: &mut UserInterface,
        event: &OsEvent,
    ) {
        self.tree.handle_os_event(self_handle, ui, event);
    }

    fn remove_ref(&mut self, handle: Handle<UiNode>) {
        self.tree.remove_ref(handle);
    }
}

#[derive(Default)]
pub struct SceneItemBuilder {
    node: Handle<Node>,
    name: String,
    visibility: bool,
    icon: Option<SharedTexture>,
    context_menu: Handle<UiNode>,
}

impl SceneItemBuilder {
    pub fn new() -> Self {
        Self {
            node: Default::default(),
            name: Default::default(),
            visibility: true,
            icon: None,
            context_menu: Default::default(),
        }
    }

    pub fn with_node(mut self, node: Handle<Node>) -> Self {
        self.node = node;
        self
    }

    pub fn with_name(mut self, name: String) -> Self {
        self.name = name;
        self
    }

    pub fn with_visibility(mut self, visibility: bool) -> Self {
        self.visibility = visibility;
        self
    }

    pub fn with_icon(mut self, icon: Option<SharedTexture>) -> Self {
        self.icon = icon;
        self
    }

    pub fn with_context_menu(mut self, menu: Handle<UiNode>) -> Self {
        self.context_menu = menu;
        self
    }

    pub fn build(
        self,
        ctx: &mut BuildContext,
        sender: Sender<Message>,
        resource_manager: ResourceManager,
        node: &Node,
    ) -> Handle<UiNode> {
        let visible_texture = load_image(include_bytes!("../resources/embed/visible.png"));

        let text_name;
        let visibility_toggle;
        let tree = TreeBuilder::new(
            WidgetBuilder::new()
                .with_context_menu(self.context_menu)
                .with_margin(Thickness {
                    left: 1.0,
                    top: 1.0,
                    right: 0.0,
                    bottom: 0.0,
                }),
        )
        .with_content(
            GridBuilder::new(
                WidgetBuilder::new()
                    .with_child(
                        ImageBuilder::new(
                            WidgetBuilder::new()
                                .with_width(16.0)
                                .with_height(16.0)
                                .on_column(0)
                                .with_margin(Thickness::uniform(1.0)),
                        )
                        .with_opt_texture(self.icon)
                        .build(ctx),
                    )
                    .with_child({
                        text_name = TextBuilder::new(
                            WidgetBuilder::new()
                                .with_foreground(if node.resource().is_some() {
                                    Brush::Solid(Color::opaque(160, 160, 200))
                                } else {
                                    Brush::Solid(rg3d::gui::COLOR_FOREGROUND)
                                })
                                .with_margin(Thickness::uniform(1.0))
                                .on_column(1)
                                .with_vertical_alignment(VerticalAlignment::Center),
                        )
                        .with_text(format!(
                            "{} ({}:{})",
                            self.name,
                            self.node.index(),
                            self.node.generation()
                        ))
                        .build(ctx);
                        text_name
                    })
                    .with_child({
                        visibility_toggle = ButtonBuilder::new(
                            WidgetBuilder::new()
                                .with_margin(Thickness::uniform(1.0))
                                .with_width(22.0)
                                .with_height(16.0)
                                .with_horizontal_alignment(HorizontalAlignment::Right)
                                .on_column(2),
                        )
                        .with_content(
                            ImageBuilder::new(
                                WidgetBuilder::new().with_margin(Thickness::uniform(1.0)),
                            )
                            .with_opt_texture(visible_texture)
                            .build(ctx),
                        )
                        .build(ctx);
                        visibility_toggle
                    }),
            )
            .add_row(Row::stretch())
            .add_column(Column::auto())
            .add_column(Column::auto())
            .add_column(Column::stretch())
            .build(ctx),
        )
        .build_tree(ctx);

        let item = SceneItem {
            tree,
            node: self.node,
            visibility_toggle,
            sender,
            visibility: self.visibility,
            resource_manager,
            text_name,
        };

        ctx.add_node(UiNode::new(item))
    }
}

fn make_tree(
    node: &Node,
    handle: Handle<Node>,
    ctx: &mut BuildContext,
    sender: Sender<Message>,
    resource_manager: ResourceManager,
    context_menu: Handle<UiNode>,
) -> Handle<UiNode> {
    let icon = match node {
        Node::Light(_) => load_image(include_bytes!("../resources/embed/light.png")),
        _ => load_image(include_bytes!("../resources/embed/cube.png")),
    };

    SceneItemBuilder::new()
        .with_name(node.name().to_owned())
        .with_node(handle)
        .with_visibility(node.visibility())
        .with_icon(icon)
        .with_context_menu(context_menu)
        .build(ctx, sender, resource_manager, node)
}

fn tree_node(ui: &UserInterface, tree: Handle<UiNode>) -> Handle<Node> {
    if let Some(item) = ui.node(tree).cast::<SceneItem>() {
        return item.node;
    }
    unreachable!()
}

fn colorize(tree: Handle<UiNode>, ui: &UserInterface, index: &mut usize) {
    let node = ui.node(tree);

    if let Some(i) = node.cast::<SceneItem>() {
        ui.send_message(UiMessage::user(
            tree,
            MessageDirection::ToWidget,
            Box::new(SceneItemMessage::Order(*index % 2 == 0)),
        ));

        *index += 1;

        for &item in i.tree.items() {
            colorize(item, ui, index);
        }
    } else if let Some(root) = node.cast::<TreeRoot>() {
        for &item in root.items() {
            colorize(item, ui, index);
        }
    }
}

impl WorldOutliner {
    pub fn new(ctx: &mut BuildContext, sender: Sender<Message>) -> Self {
        let root;
        let node_path;
        let collapse_all;
        let expand_all;
        let locate_selection;
        let scroll_view;
        let window = WindowBuilder::new(WidgetBuilder::new())
            .can_minimize(false)
            .with_title(WindowTitle::text("Scene Graph"))
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
                                    }),
                            )
                            .with_orientation(Orientation::Horizontal)
                            .build(ctx),
                        )
                        .with_child(
                            TextBuilder::new(
                                WidgetBuilder::new()
                                    .on_row(1)
                                    .on_column(0)
                                    .with_opacity(0.4),
                            )
                            .with_text("Breadcrumbs")
                            .with_vertical_text_alignment(VerticalAlignment::Center)
                            .with_horizontal_text_alignment(HorizontalAlignment::Center)
                            .build(ctx),
                        )
                        .with_child(
                            ScrollViewerBuilder::new(WidgetBuilder::new().on_row(1))
                                .with_content({
                                    node_path = StackPanelBuilder::new(WidgetBuilder::new())
                                        .with_orientation(Orientation::Horizontal)
                                        .build(ctx);
                                    node_path
                                })
                                .build(ctx),
                        )
                        .with_child({
                            scroll_view = ScrollViewerBuilder::new(WidgetBuilder::new().on_row(2))
                                .with_content({
                                    root = TreeRootBuilder::new(WidgetBuilder::new()).build(ctx);
                                    root
                                })
                                .build(ctx);
                            scroll_view
                        }),
                )
                .add_column(Column::stretch())
                .add_row(Row::strict(24.0))
                .add_row(Row::strict(24.0))
                .add_row(Row::stretch())
                .build(ctx),
            )
            .build(ctx);

        let item_context_menu = ItemContextMenu::new(ctx);

        Self {
            window,
            sender,
            root,
            node_path,
            stack: Default::default(),
            sync_selection: false,
            breadcrumbs: Default::default(),
            locate_selection,
            collapse_all,
            expand_all,
            scroll_view,
            item_context_menu,
        }
    }

    pub fn sync_to_model(&mut self, editor_scene: &EditorScene, engine: &mut GameEngine) {
        scope_profile!();

        let scene = &mut engine.scenes[editor_scene.scene];
        let graph = &mut scene.graph;
        let ui = &mut engine.user_interface;

        let mut selected_items = Vec::new();
        // Sync tree structure with graph structure.
        self.stack.clear();
        self.stack.push((self.root, graph.get_root()));
        while let Some((tree_handle, node_handle)) = self.stack.pop() {
            // Hide all editor nodes.
            if node_handle == editor_scene.root {
                continue;
            }
            let node = &graph[node_handle];
            let ui_node = ui.node(tree_handle);

            if let Some(item) = ui_node.cast::<SceneItem>() {
                // Since we are filtering out editor stuff from world outliner, we must
                // correctly count children, excluding editor nodes.
                let mut child_count = 0;
                for &child in node.children() {
                    if child != editor_scene.root {
                        child_count += 1;
                    }
                }
                let items = item.tree.items().to_vec();
                if child_count < items.len() {
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
                        } else {
                            self.stack.push((item, child_node));
                        }
                    }
                } else if child_count > item.tree.items().len() {
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
                            let tree = make_tree(
                                &graph[child_handle],
                                child_handle,
                                &mut ui.build_ctx(),
                                self.sender.clone(),
                                engine.resource_manager.clone(),
                                self.item_context_menu.menu,
                            );
                            send_sync_message(
                                ui,
                                TreeMessage::add_item(
                                    tree_handle,
                                    MessageDirection::ToWidget,
                                    tree,
                                ),
                            );
                            if let Selection::Graph(selection) = &editor_scene.selection {
                                if selection.contains(child_handle) {
                                    selected_items.push(tree);
                                }
                            }
                            self.stack.push((tree, child_handle));
                        }
                    }
                } else {
                    for &tree in items.iter() {
                        let child = tree_node(ui, tree);
                        self.stack.push((tree, child));
                    }
                }
            } else if let Some(root) = ui_node.cast::<TreeRoot>() {
                if root.items().is_empty() {
                    let tree = make_tree(
                        node,
                        node_handle,
                        &mut ui.build_ctx(),
                        self.sender.clone(),
                        engine.resource_manager.clone(),
                        self.item_context_menu.menu,
                    );
                    send_sync_message(
                        ui,
                        TreeRootMessage::add_item(tree_handle, MessageDirection::ToWidget, tree),
                    );
                    self.stack.push((tree, node_handle));
                } else {
                    self.stack.push((root.items()[0], node_handle));
                }
            }
        }

        if !selected_items.is_empty() {
            send_sync_message(
                ui,
                TreeRootMessage::select(self.root, MessageDirection::ToWidget, selected_items),
            );
        }

        // Update breadcrumbs.
        self.breadcrumbs.clear();
        for &child in ui.node(self.node_path).children() {
            send_sync_message(ui, WidgetMessage::remove(child, MessageDirection::ToWidget));
        }
        if let Selection::Graph(selection) = &editor_scene.selection {
            if let Some(&first_selected) = selection.nodes().first() {
                let mut item = first_selected;
                while item.is_some() {
                    let node = &graph[item];

                    let element = ButtonBuilder::new(
                        WidgetBuilder::new().with_margin(Thickness::uniform(1.0)),
                    )
                    .with_text(node.name())
                    .build(&mut ui.build_ctx());

                    send_sync_message(
                        ui,
                        WidgetMessage::link_reverse(
                            element,
                            MessageDirection::ToWidget,
                            self.node_path,
                        ),
                    );

                    self.breadcrumbs.insert(element, item);

                    item = node.parent();
                }
            }
        }

        // Sync items data.
        let mut stack = vec![self.root];
        while let Some(handle) = stack.pop() {
            let ui_node = ui.node(handle);

            if let Some(item) = ui_node.cast::<SceneItem>() {
                if graph.is_valid_handle(item.node) {
                    let node = &graph[item.node];
                    send_sync_message(
                        ui,
                        SceneItemMessage::node_visibility(handle, node.visibility()),
                    );
                    send_sync_message(ui, SceneItemMessage::name(handle, node.name().to_owned()));
                    stack.extend_from_slice(item.tree.items());
                }
            } else if let Some(root) = ui_node.cast::<TreeRoot>() {
                stack.extend_from_slice(root.items())
            }
        }

        self.colorize(ui);
    }

    fn map_tree_to_node(&self, tree: Handle<UiNode>, ui: &UserInterface) -> Handle<Node> {
        if tree.is_some() {
            tree_node(ui, tree)
        } else {
            Handle::NONE
        }
    }

    pub fn colorize(&mut self, ui: &UserInterface) {
        let mut index = 0;
        colorize(self.root, ui, &mut index);
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

        match message.data() {
            UiMessageData::TreeRoot(msg) => {
                if message.destination() == self.root
                    && message.direction() == MessageDirection::FromWidget
                {
                    if let TreeRootMessage::Selected(selection) = msg {
                        let new_selection = Selection::Graph(GraphSelection::from_list(
                            selection
                                .iter()
                                .map(|&h| self.map_tree_to_node(h, &engine.user_interface))
                                .collect(),
                        ));
                        if new_selection != editor_scene.selection {
                            self.sender
                                .send(Message::do_scene_command(ChangeSelectionCommand::new(
                                    new_selection,
                                    editor_scene.selection.clone(),
                                )))
                                .unwrap();
                        }
                    }
                }
            }
            &UiMessageData::Widget(WidgetMessage::Drop(node)) => {
                if engine.user_interface.is_node_child_of(node, self.root)
                    && engine
                        .user_interface
                        .is_node_child_of(message.destination(), self.root)
                    && node != message.destination()
                {
                    let child = self.map_tree_to_node(node, &engine.user_interface);
                    let parent =
                        self.map_tree_to_node(message.destination(), &engine.user_interface);
                    if child.is_some() && parent.is_some() {
                        // Make sure we won't create any loops - child must not have parent in its
                        // descendants.
                        let mut attach = true;
                        let graph = &engine.scenes[editor_scene.scene].graph;
                        let mut p = parent;
                        while p.is_some() {
                            if p == child {
                                attach = false;
                                break;
                            }
                            p = graph[p].parent();
                        }

                        if attach {
                            self.sender
                                .send(Message::do_scene_command(LinkNodesCommand::new(
                                    child, parent,
                                )))
                                .unwrap();
                        }
                    }
                }
            }
            UiMessageData::Button(ButtonMessage::Click) => {
                if let Some(&node) = self.breadcrumbs.get(&message.destination()) {
                    self.sender
                        .send(Message::do_scene_command(ChangeSelectionCommand::new(
                            Selection::Graph(GraphSelection::single_or_empty(node)),
                            editor_scene.selection.clone(),
                        )))
                        .unwrap();
                } else if message.destination() == self.collapse_all {
                    engine
                        .user_interface
                        .send_message(TreeRootMessage::collapse_all(
                            self.root,
                            MessageDirection::ToWidget,
                        ));
                } else if message.destination() == self.expand_all {
                    engine
                        .user_interface
                        .send_message(TreeRootMessage::expand_all(
                            self.root,
                            MessageDirection::ToWidget,
                        ));
                } else if message.destination() == self.locate_selection {
                    if let Selection::Graph(ref selection) = editor_scene.selection {
                        if let Some(&first) = selection.nodes().first() {
                            let tree = self.map_node_to_tree(&engine.user_interface, first);

                            engine.user_interface.send_message(TreeMessage::expand(
                                tree,
                                MessageDirection::ToWidget,
                                true,
                                TreeExpansionStrategy::RecursiveAncestors,
                            ));

                            engine.user_interface.send_message(
                                ScrollViewerMessage::bring_into_view(
                                    self.scroll_view,
                                    MessageDirection::ToWidget,
                                    tree,
                                ),
                            );
                        }
                    }
                }
            }
            _ => {}
        }
    }

    pub fn post_update(&mut self, editor_scene: &EditorScene, engine: &GameEngine) {
        // Hack. See `self.sync_selection` for details.
        if self.sync_selection {
            let ui = &engine.user_interface;

            let trees = if let Selection::Graph(selection) = &editor_scene.selection {
                selection
                    .nodes()
                    .iter()
                    .map(|&n| self.map_node_to_tree(ui, n))
                    .collect()
            } else {
                Default::default()
            };

            send_sync_message(
                ui,
                TreeRootMessage::select(self.root, MessageDirection::ToWidget, trees),
            );

            self.sync_selection = false;
        }
    }

    pub fn clear(&mut self, ui: &mut UserInterface) {
        ui.send_message(TreeRootMessage::items(
            self.root,
            MessageDirection::ToWidget,
            vec![],
        ));
    }

    fn map_node_to_tree(&self, ui: &UserInterface, node: Handle<Node>) -> Handle<UiNode> {
        let mut stack = vec![self.root];
        while let Some(tree_handle) = stack.pop() {
            let ui_node = ui.node(tree_handle);
            if let Some(item) = ui_node.cast::<SceneItem>() {
                if item.node == node {
                    return tree_handle;
                }
                stack.extend_from_slice(item.tree.items());
            } else if let Some(root) = ui_node.cast::<TreeRoot>() {
                stack.extend_from_slice(root.items());
            } else {
                unreachable!()
            }
        }
        unreachable!("Must not be reached. If still triggered then there is a bug.")
    }
}
