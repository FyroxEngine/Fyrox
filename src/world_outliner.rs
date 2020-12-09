use crate::{
    gui::{
        BuildContext, CustomWidget, EditorUiMessage, EditorUiNode, SceneItemMessage, Ui, UiMessage,
        UiNode,
    },
    load_image,
    scene::{
        ChangeSelectionCommand, EditorScene, LinkNodesCommand, SceneCommand, Selection,
        SetVisibleCommand,
    },
    GameEngine, Message,
};
use rg3d::gui::message::TextMessage;
use rg3d::{
    core::{algebra::Vector2, math::Rect, pool::Handle},
    engine::resource_manager::ResourceManager,
    gui::{
        brush::Brush,
        button::ButtonBuilder,
        core::color::Color,
        draw::{DrawingContext, SharedTexture},
        grid::{Column, GridBuilder, Row},
        image::ImageBuilder,
        message::{
            ButtonMessage, OsEvent, TreeMessage, TreeRootMessage, UiMessageData, WidgetMessage,
        },
        message::{DecoratorMessage, MessageDirection},
        node::UINode,
        scroll_viewer::ScrollViewerBuilder,
        text::TextBuilder,
        tree::{Tree, TreeBuilder, TreeRootBuilder},
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowTitle},
        Control, HorizontalAlignment, NodeHandleMapping, Thickness, VerticalAlignment,
    },
    scene::node::Node,
};
use std::{
    fmt::{Debug, Formatter},
    ops::{Deref, DerefMut},
    sync::mpsc::Sender,
};

pub struct WorldOutliner {
    pub window: Handle<UiNode>,
    root: Handle<UiNode>,
    sender: Sender<Message>,
    stack: Vec<(Handle<UiNode>, Handle<Node>)>,
}

#[derive(Clone)]
pub struct SceneItem {
    tree: Tree<EditorUiMessage, EditorUiNode>,
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
    type Target = CustomWidget;

    fn deref(&self) -> &Self::Target {
        &self.tree
    }
}

impl DerefMut for SceneItem {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.tree
    }
}

impl Control<EditorUiMessage, EditorUiNode> for SceneItem {
    fn resolve(&mut self, node_map: &NodeHandleMapping<EditorUiMessage, EditorUiNode>) {
        self.tree.resolve(node_map);
        node_map.resolve(&mut self.text_name);
    }

    fn measure_override(&self, ui: &Ui, available_size: Vector2<f32>) -> Vector2<f32> {
        self.tree.measure_override(ui, available_size)
    }

    fn arrange_override(&self, ui: &Ui, final_size: Vector2<f32>) -> Vector2<f32> {
        self.tree.arrange_override(ui, final_size)
    }

    fn arrange(&self, ui: &Ui, final_rect: &Rect<f32>) {
        self.tree.arrange(ui, final_rect);
    }

    fn measure(&self, ui: &Ui, available_size: Vector2<f32>) {
        self.tree.measure(ui, available_size)
    }

    fn draw(&self, drawing_context: &mut DrawingContext) {
        self.tree.draw(drawing_context);
    }

    fn update(&mut self, dt: f32) {
        self.tree.update(dt);
    }

    fn handle_routed_message(&mut self, ui: &mut Ui, message: &mut UiMessage) {
        self.tree.handle_routed_message(ui, message);

        match &message.data() {
            UiMessageData::Button(msg) => {
                if message.destination() == self.visibility_toggle {
                    if let ButtonMessage::Click = msg {
                        let command = SceneCommand::SetVisible(SetVisibleCommand::new(
                            self.node,
                            !self.visibility,
                        ));
                        self.sender.send(Message::DoSceneCommand(command)).unwrap();
                    }
                }
            }
            UiMessageData::User(msg) => {
                if let EditorUiMessage::SceneItem(item) = msg {
                    match item {
                        &SceneItemMessage::NodeVisibility(visibility) => {
                            if self.visibility != visibility
                                && message.destination() == self.handle()
                            {
                                self.visibility = visibility;

                                let path = if visibility {
                                    "resources/visible.png"
                                } else {
                                    "resources/invisible.png"
                                };
                                let image = ImageBuilder::new(WidgetBuilder::new())
                                    .with_opt_texture(load_image(
                                        path,
                                        self.resource_manager.clone(),
                                    ))
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
                                ui.send_message(TextMessage::text(
                                    self.text_name,
                                    MessageDirection::ToWidget,
                                    name.clone(),
                                ));
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn preview_message(&self, ui: &Ui, message: &mut UiMessage) {
        self.tree.preview_message(ui, message);
    }

    fn handle_os_event(&mut self, self_handle: Handle<UiNode>, ui: &mut Ui, event: &OsEvent) {
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
}

impl SceneItemBuilder {
    pub fn new() -> Self {
        Self {
            node: Default::default(),
            name: Default::default(),
            visibility: true,
            icon: None,
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

    pub fn build(
        self,
        ctx: &mut BuildContext,
        sender: Sender<Message>,
        resource_manager: ResourceManager,
    ) -> Handle<UiNode> {
        let visible_texture = load_image("resources/visible.png", resource_manager.clone());

        let text_name;
        let visibility_toggle;
        let tree = TreeBuilder::new(WidgetBuilder::new().with_margin(Thickness {
            left: 1.0,
            top: 1.0,
            right: 0.0,
            bottom: 0.0,
        }))
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
                                .with_margin(Thickness::uniform(1.0))
                                .on_column(1)
                                .with_vertical_alignment(VerticalAlignment::Center),
                        )
                        .with_text(self.name)
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

        ctx.add_node(UiNode::User(EditorUiNode::SceneItem(item)))
    }
}

fn make_tree(
    node: &Node,
    handle: Handle<Node>,
    ctx: &mut BuildContext,
    sender: Sender<Message>,
    resource_manager: ResourceManager,
) -> Handle<UiNode> {
    let icon_path = match node {
        Node::Light(_) => "resources/light.png",
        _ => "resources/cube.png",
    };

    let icon = load_image(icon_path, resource_manager.clone());

    SceneItemBuilder::new()
        .with_name(node.name().to_owned())
        .with_node(handle)
        .with_visibility(node.visibility())
        .with_icon(icon)
        .build(ctx, sender, resource_manager)
}

fn tree_node(ui: &Ui, tree: Handle<UiNode>) -> Handle<Node> {
    if let UiNode::User(usr) = ui.node(tree) {
        if let EditorUiNode::SceneItem(item) = usr {
            return item.node;
        }
    }
    unreachable!()
}

fn colorize(tree: Handle<UiNode>, ui: &Ui, index: &mut usize) {
    match ui.node(tree) {
        UINode::User(u) => {
            if let EditorUiNode::SceneItem(i) = u {
                ui.send_message(UiMessage::user(
                    tree,
                    MessageDirection::ToWidget,
                    EditorUiMessage::SceneItem(SceneItemMessage::Order(*index % 2 == 0)),
                ));

                *index += 1;

                for &item in i.tree.items() {
                    colorize(item, ui, index);
                }
            }
        }
        UINode::TreeRoot(root) => {
            for &item in root.items() {
                colorize(item, ui, index);
            }
        }
        _ => (),
    }
}

impl WorldOutliner {
    pub fn new(ctx: &mut BuildContext, sender: Sender<Message>) -> Self {
        let root;
        let window = WindowBuilder::new(WidgetBuilder::new())
            .can_minimize(false)
            .with_title(WindowTitle::text("World Outliner"))
            .with_content({
                ScrollViewerBuilder::new(WidgetBuilder::new())
                    .with_content({
                        root = TreeRootBuilder::new(WidgetBuilder::new()).build(ctx);
                        root
                    })
                    .build(ctx)
            })
            .build(ctx);

        Self {
            window,
            sender,
            root,
            stack: Default::default(),
        }
    }

    pub fn sync_to_model(&mut self, editor_scene: &EditorScene, engine: &mut GameEngine) {
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
            match ui.node(tree_handle) {
                UiNode::User(usr) => {
                    if let EditorUiNode::SceneItem(item) = usr {
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
                                    ui.send_message(TreeMessage::remove_item(
                                        tree_handle,
                                        MessageDirection::ToWidget,
                                        item,
                                    ));
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
                                    );
                                    ui.send_message(TreeMessage::add_item(
                                        tree_handle,
                                        MessageDirection::ToWidget,
                                        tree,
                                    ));
                                    if editor_scene.selection.contains(child_handle) {
                                        selected_items.push(tree);
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
                    }
                }
                UiNode::TreeRoot(root) => {
                    if root.items().is_empty() {
                        let tree = make_tree(
                            node,
                            node_handle,
                            &mut ui.build_ctx(),
                            self.sender.clone(),
                            engine.resource_manager.clone(),
                        );
                        ui.send_message(TreeRootMessage::add_item(
                            tree_handle,
                            MessageDirection::ToWidget,
                            tree,
                        ));
                        self.stack.push((tree, node_handle));
                    } else {
                        self.stack.push((root.items()[0], node_handle));
                    }
                }
                _ => unreachable!(),
            }
        }

        if !selected_items.is_empty() {
            ui.send_message(TreeRootMessage::select(
                self.root,
                MessageDirection::ToWidget,
                selected_items,
            ));
        }

        // Sync items data.
        let mut stack = vec![self.root];
        while let Some(handle) = stack.pop() {
            match ui.node(handle) {
                UiNode::User(usr) => {
                    if let EditorUiNode::SceneItem(item) = usr {
                        if graph.is_valid_handle(item.node) {
                            let node = &graph[item.node];
                            ui.send_message(SceneItemMessage::node_visibility(
                                handle,
                                node.visibility(),
                            ));
                            ui.send_message(SceneItemMessage::name(handle, node.name().to_owned()));
                            stack.extend_from_slice(item.tree.items());
                        }
                    }
                }
                UiNode::TreeRoot(root) => stack.extend_from_slice(root.items()),
                _ => unreachable!(),
            }
        }

        self.colorize(ui);
    }

    fn map_tree_to_node(&self, tree: Handle<UiNode>, ui: &Ui) -> Handle<Node> {
        if tree.is_some() {
            tree_node(ui, tree)
        } else {
            Handle::NONE
        }
    }

    pub fn colorize(&mut self, ui: &Ui) {
        let mut index = 0;
        colorize(self.root, ui, &mut index);
    }

    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        editor_scene: &EditorScene,
        engine: &GameEngine,
    ) {
        match &message.data() {
            UiMessageData::TreeRoot(msg) => {
                if message.destination() == self.root {
                    if let TreeRootMessage::Selected(selection) = msg {
                        let new_selection = Selection::from_list(
                            selection
                                .iter()
                                .map(|&h| self.map_tree_to_node(h, &engine.user_interface))
                                .collect(),
                        );
                        if new_selection != editor_scene.selection {
                            self.sender
                                .send(Message::DoSceneCommand(SceneCommand::ChangeSelection(
                                    ChangeSelectionCommand::new(
                                        new_selection,
                                        editor_scene.selection.clone(),
                                    ),
                                )))
                                .unwrap();
                        }
                    }
                }
            }
            UiMessageData::Widget(msg) => {
                if let WidgetMessage::Drop(node) = *msg {
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
                                    .send(Message::DoSceneCommand(SceneCommand::LinkNodes(
                                        LinkNodesCommand::new(child, parent),
                                    )))
                                    .unwrap();
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    pub fn clear(&mut self, ui: &mut Ui) {
        ui.send_message(TreeRootMessage::items(
            self.root,
            MessageDirection::ToWidget,
            vec![],
        ));
    }

    pub fn handle_message(
        &mut self,
        message: &Message,
        engine: &mut GameEngine,
        editor_scene: Option<&EditorScene>,
    ) {
        let ui = &engine.user_interface;

        if let Some(editor_scene) = editor_scene {
            if let Message::SelectionChanged = message {
                let trees = editor_scene
                    .selection
                    .nodes()
                    .iter()
                    .map(|&n| self.map_node_to_tree(ui, n))
                    .collect();
                ui.send_message(TreeRootMessage::select(
                    self.root,
                    MessageDirection::ToWidget,
                    trees,
                ));
            }
        }
    }

    fn map_node_to_tree(&self, ui: &Ui, node: Handle<Node>) -> Handle<UiNode> {
        let mut stack = vec![self.root];
        while let Some(tree_handle) = stack.pop() {
            match ui.node(tree_handle) {
                UiNode::User(usr) => {
                    if let EditorUiNode::SceneItem(item) = usr {
                        if item.node == node {
                            return tree_handle;
                        }
                        stack.extend_from_slice(item.tree.items());
                    }
                }
                UiNode::TreeRoot(root) => {
                    stack.extend_from_slice(root.items());
                }
                _ => unreachable!(),
            }
        }
        Handle::NONE
    }
}
