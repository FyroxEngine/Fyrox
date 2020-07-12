use crate::{
    gui::{
        BuildContext, CustomWidget, EditorUiMessage, EditorUiNode, SceneItemMessage, Ui, UiMessage,
        UiNode,
    },
    scene::SetVisibleCommand,
    scene::{ChangeSelectionCommand, EditorScene, LinkNodesCommand, SceneCommand},
    GameEngine, Message,
};
use rg3d::{
    core::{math::vec2::Vec2, math::Rect, pool::Handle},
    engine::resource_manager::ResourceManager,
    gui::{
        button::ButtonBuilder,
        draw::DrawingContext,
        grid::{Column, GridBuilder, Row},
        image::ImageBuilder,
        message::{
            ButtonMessage, OsEvent, TreeMessage, TreeRootMessage, UiMessageData, WidgetMessage,
        },
        scroll_viewer::ScrollViewerBuilder,
        text::TextBuilder,
        tree::{Tree, TreeBuilder, TreeRootBuilder},
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowTitle},
        Control, HorizontalAlignment, NodeHandleMapping, Thickness,
    },
    resource::texture::TextureKind,
    scene::node::Node,
    utils::into_any_arc,
};
use std::{
    fmt::{Debug, Formatter},
    ops::{Deref, DerefMut},
    sync::{mpsc::Sender, Arc, Mutex},
};

pub struct WorldOutliner {
    pub window: Handle<UiNode>,
    root: Handle<UiNode>,
    sender: Sender<Message>,
    stack: Vec<(Handle<UiNode>, Handle<Node>)>,
}

pub struct SceneItem {
    tree: Tree<EditorUiMessage, EditorUiNode>,
    node: Handle<Node>,
    visibility_toggle: Handle<UiNode>,
    sender: Sender<Message>,
    visibility: bool,
    resource_manager: Arc<Mutex<ResourceManager>>,
}

impl Debug for SceneItem {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "SceneItem")
    }
}

impl Clone for SceneItem {
    fn clone(&self) -> Self {
        Self {
            tree: self.tree.clone(),
            node: self.node,
            visibility_toggle: self.visibility_toggle,
            sender: self.sender.clone(),
            visibility: self.visibility,
            resource_manager: self.resource_manager.clone(),
        }
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
    fn raw_copy(&self) -> UiNode {
        UiNode::User(EditorUiNode::SceneItem(self.clone()))
    }

    fn resolve(&mut self, node_map: &NodeHandleMapping<EditorUiMessage, EditorUiNode>) {
        self.tree.resolve(node_map);
    }

    fn measure_override(&self, ui: &Ui, available_size: Vec2) -> Vec2 {
        self.tree.measure_override(ui, available_size)
    }

    fn arrange_override(&self, ui: &Ui, final_size: Vec2) -> Vec2 {
        self.tree.arrange_override(ui, final_size)
    }

    fn arrange(&self, ui: &Ui, final_rect: &Rect<f32>) {
        self.tree.arrange(ui, final_rect);
    }

    fn measure(&self, ui: &Ui, available_size: Vec2) {
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

        match &message.data {
            UiMessageData::Button(msg) => {
                if message.destination == self.visibility_toggle {
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
                    if let &SceneItemMessage::NodeVisibility(visibility) = item {
                        if self.visibility != visibility {
                            self.visibility = visibility;

                            let path = if visibility {
                                "resources/visible.png"
                            } else {
                                "resources/invisible.png"
                            };
                            let image = ImageBuilder::new(WidgetBuilder::new())
                                .with_opt_texture(into_any_arc(
                                    self.resource_manager
                                        .lock()
                                        .unwrap()
                                        .request_texture(path, TextureKind::RGBA8),
                                ))
                                .build(&mut ui.build_ctx());
                            ui.send_message(ButtonMessage::content(self.visibility_toggle, image));
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn preview_message(&mut self, ui: &mut Ui, message: &mut UiMessage) {
        self.tree.preview_message(ui, message);
    }

    fn handle_os_event(&mut self, self_handle: Handle<UiNode>, ui: &mut Ui, event: &OsEvent) {
        self.tree.handle_os_event(self_handle, ui, event);
    }

    fn remove_ref(&mut self, handle: Handle<UiNode>) {
        self.tree.remove_ref(handle);
    }
}

pub struct SceneItemBuilder {
    node: Handle<Node>,
    name: String,
    visibility: bool,
}

impl SceneItemBuilder {
    pub fn new() -> Self {
        Self {
            node: Default::default(),
            name: Default::default(),
            visibility: true,
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

    pub fn build(
        self,
        ctx: &mut BuildContext,
        sender: Sender<Message>,
        resource_manager: Arc<Mutex<ResourceManager>>,
    ) -> Handle<UiNode> {
        let visibility_toggle;
        let item = SceneItem {
            tree: TreeBuilder::new(WidgetBuilder::new().with_margin(Thickness::uniform(1.0)))
                .with_content(
                    GridBuilder::new(
                        WidgetBuilder::new()
                            .with_child(
                                TextBuilder::new(WidgetBuilder::new())
                                    .with_text(self.name)
                                    .build(ctx),
                            )
                            .with_child({
                                visibility_toggle = ButtonBuilder::new(
                                    WidgetBuilder::new()
                                        .with_width(22.0)
                                        .with_height(16.0)
                                        .with_horizontal_alignment(HorizontalAlignment::Right)
                                        .on_column(1),
                                )
                                .with_content(
                                    ImageBuilder::new(WidgetBuilder::new())
                                        .with_opt_texture(into_any_arc(
                                            resource_manager.lock().unwrap().request_texture(
                                                "resources/visible.png",
                                                TextureKind::RGBA8,
                                            ),
                                        ))
                                        .build(ctx),
                                )
                                .build(ctx);
                                visibility_toggle
                            }),
                    )
                    .add_row(Row::stretch())
                    .add_column(Column::auto())
                    .add_column(Column::stretch())
                    .build(ctx),
                )
                .build_tree(ctx),
            node: self.node,
            visibility_toggle,
            sender,
            visibility: self.visibility,
            resource_manager,
        };

        ctx.add_node(UiNode::User(EditorUiNode::SceneItem(item)))
    }
}

fn make_tree(
    node: &Node,
    handle: Handle<Node>,
    ctx: &mut BuildContext,
    sender: Sender<Message>,
    resource_manager: Arc<Mutex<ResourceManager>>,
) -> Handle<UiNode> {
    SceneItemBuilder::new()
        .with_name(node.name().to_owned())
        .with_node(handle)
        .with_visibility(node.visibility())
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

impl WorldOutliner {
    pub fn new(ctx: &mut BuildContext, sender: Sender<Message>) -> Self {
        let root;
        let window = WindowBuilder::new(WidgetBuilder::new())
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

    pub fn sync_to_model(
        &mut self,
        editor_scene: &EditorScene,
        engine: &mut GameEngine,
        current_selection: Handle<Node>,
    ) {
        let scene = &mut engine.scenes[editor_scene.scene];
        let graph = &mut scene.graph;
        let ui = &mut engine.user_interface;

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
                                    ui.send_message(TreeMessage::remove_item(tree_handle, item));
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
                                    ui.send_message(TreeMessage::add_item(tree_handle, tree));
                                    if child_handle == current_selection {
                                        ui.send_message(TreeRootMessage::select(self.root, tree));
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
                        ui.send_message(TreeRootMessage::add_item(tree_handle, tree));
                        self.stack.push((tree, node_handle));
                    } else {
                        self.stack.push((root.items()[0], node_handle));
                    }
                }
                _ => unreachable!(),
            }
        }

        // Sync items data.
        let mut stack = vec![self.root];
        while let Some(handle) = stack.pop() {
            match ui.node(handle) {
                UiNode::User(usr) => {
                    if let EditorUiNode::SceneItem(item) = usr {
                        let node = &graph[item.node];
                        ui.send_message(SceneItemMessage::node_visibility(
                            handle,
                            node.visibility(),
                        ));
                        stack.extend_from_slice(item.tree.items());
                    }
                }
                UiNode::TreeRoot(root) => stack.extend_from_slice(root.items()),
                _ => unreachable!(),
            }
        }
    }

    fn map_tree_to_node(&self, tree: Handle<UiNode>, ui: &Ui) -> Handle<Node> {
        if tree.is_some() {
            tree_node(ui, tree)
        } else {
            Handle::NONE
        }
    }

    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        ui: &Ui,
        current_selection: Handle<Node>,
    ) {
        match &message.data {
            UiMessageData::TreeRoot(msg) => {
                if message.destination == self.root {
                    if let &TreeRootMessage::Selected(selection) = msg {
                        let node = self.map_tree_to_node(selection, ui);
                        if node != current_selection {
                            self.sender
                                .send(Message::DoSceneCommand(SceneCommand::ChangeSelection(
                                    ChangeSelectionCommand::new(node, current_selection),
                                )))
                                .unwrap();
                        }
                    }
                }
            }
            UiMessageData::Widget(msg) => {
                if let &WidgetMessage::Drop(node) = msg {
                    if ui.is_node_child_of(node, self.root)
                        && ui.is_node_child_of(message.destination, self.root)
                        && node != message.destination
                    {
                        let child = self.map_tree_to_node(node, ui);
                        let parent = self.map_tree_to_node(message.destination, ui);
                        if child.is_some() && parent.is_some() {
                            self.sender
                                .send(Message::DoSceneCommand(SceneCommand::LinkNodes(
                                    LinkNodesCommand::new(child, parent),
                                )))
                                .unwrap();
                        }
                    }
                }
            }
            _ => {}
        }
    }

    pub fn clear(&mut self, ui: &mut Ui) {
        ui.send_message(TreeRootMessage::items(self.root, vec![]));
    }

    pub fn handle_message(&mut self, message: &Message, engine: &mut GameEngine) {
        let ui = &engine.user_interface;

        match message {
            &Message::SetSelection(selection) => {
                let tree = self.map_node_to_tree(ui, selection);
                ui.send_message(TreeRootMessage::select(self.root, tree));
            }
            _ => (),
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
