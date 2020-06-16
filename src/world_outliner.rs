use crate::{
    GameEngine,
    gui::UiNode,
    gui::Ui,
    gui::UiMessage,
    Message,
    scene::{
        EditorScene,
        SceneCommand,
        LinkNodesCommand,
        ChangeSelectionCommand,
    },
};
use rg3d::{
    utils::into_any_arc,
    gui::{
        window::{WindowTitle, WindowBuilder},
        widget::WidgetBuilder,
        tree::{TreeBuilder, TreeRootBuilder},
        text::TextBuilder,
        Thickness,
        image::ImageBuilder,
        grid::{GridBuilder, Row, Column},
        button::ButtonBuilder,
        HorizontalAlignment,
        message::{
            UiMessageData,
            TreeRootMessage,
            WidgetMessage,
            TreeMessage,
            ButtonMessage,
        },
        scroll_viewer::ScrollViewerBuilder,
    },
    scene::node::Node,
    core::{
        pool::Handle,
    },
    engine::resource_manager::ResourceManager,
    resource::texture::TextureKind,
};
use std::{
    sync::mpsc::Sender,
    rc::Rc,
};
use crate::gui::BuildContext;

pub struct WorldOutliner {
    pub window: Handle<UiNode>,
    root: Handle<UiNode>,
    sender: Sender<Message>,
    stack: Vec<(Handle<UiNode>, Handle<Node>)>,
}

fn make_tree(node: &Node, handle: Handle<Node>, ctx: &mut BuildContext, resource_manager: &mut ResourceManager) -> Handle<UiNode> {
    TreeBuilder::new(WidgetBuilder::new()
        .with_user_data(Rc::new(handle))
        .with_margin(Thickness::uniform(1.0)))
        .with_content(GridBuilder::new(WidgetBuilder::new()
            .with_child(TextBuilder::new(WidgetBuilder::new())
                .with_text(node.name())
                .build(ctx))
            .with_child(ButtonBuilder::new(WidgetBuilder::new()
                .with_width(22.0)
                .with_height(16.0)
                .with_horizontal_alignment(HorizontalAlignment::Right)
                .on_column(1))
                .with_content(ImageBuilder::new(WidgetBuilder::new())
                    .with_opt_texture(into_any_arc(resource_manager.request_texture("resources/visible.png", TextureKind::RGBA8)))
                    .build(ctx))
                .build(ctx)))
            .add_row(Row::stretch())
            .add_column(Column::auto())
            .add_column(Column::stretch())
            .build(ctx)
        )
        .build(ctx)
}

fn tree_node(ui: &Ui, tree: Handle<UiNode>) -> Handle<Node> {
    *ui.node(tree)
        .user_data
        .as_ref()
        .unwrap()
        .downcast_ref::<Handle<Node>>()
        .unwrap()
}

impl WorldOutliner {
    pub fn new(ctx: &mut BuildContext, sender: Sender<Message>) -> Self {
        let root;
        let window = WindowBuilder::new(WidgetBuilder::new())
            .with_title(WindowTitle::text("World Outliner"))
            .with_content({
                ScrollViewerBuilder::new(WidgetBuilder::new())
                    .with_content({
                        root = TreeRootBuilder::new(WidgetBuilder::new())
                            .build(ctx);
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

    pub fn sync_to_model(&mut self, editor_scene: &EditorScene, engine: &mut GameEngine, current_selection: Handle<Node>) {
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
                UiNode::Tree(tree) => {
                    // Since we are filtering out editor stuff from world outliner, we must
                    // correctly count children, excluding editor nodes.
                    let mut child_count = 0;
                    for &child in node.children() {
                        if child != editor_scene.root {
                            child_count += 1;
                        }
                    }
                    let items = tree.items().to_vec();
                    if child_count < items.len() {
                        for &item in items.iter() {
                            let child_node = tree_node(ui, item);
                            if !node.children().contains(&child_node) {
                                ui.send_message(TreeMessage::remove_item(tree_handle, item));
                            } else {
                                self.stack.push((item, child_node));
                            }
                        }
                    } else if child_count > tree.items().len() {
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
                                let tree = make_tree(&graph[child_handle], child_handle, &mut ui.build_ctx(), &mut engine.resource_manager.lock().unwrap());
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
                UiNode::TreeRoot(root) => {
                    if root.items().is_empty() {
                        let tree = make_tree(node, node_handle, &mut ui.build_ctx(), &mut engine.resource_manager.lock().unwrap());
                        ui.send_message(TreeRootMessage::add_item(tree_handle, tree));
                        self.stack.push((tree, node_handle));
                    } else {
                        self.stack.push((root.items()[0], node_handle));
                    }
                }
                _ => unreachable!()
            }
        }
    }

    fn map_tree_to_node(&self, tree: Handle<UiNode>, ui: &Ui) -> Handle<Node> {
        if tree.is_some() {
            *ui.node(tree).user_data.as_ref().unwrap().downcast_ref::<Handle<Node>>().unwrap()
        } else {
            Handle::NONE
        }
    }

    pub fn handle_ui_message(&mut self, message: &UiMessage, ui: &Ui, current_selection: Handle<Node>) {
        match &message.data {
            UiMessageData::TreeRoot(msg) => {
                if message.destination == self.root {
                    if let &TreeRootMessage::Selected(selection) = msg {
                        let node = self.map_tree_to_node(selection, ui);
                        if node != current_selection {
                            self.sender
                                .send(Message::DoSceneCommand(SceneCommand::ChangeSelection(ChangeSelectionCommand::new(node, current_selection))))
                                .unwrap();
                        }
                    }
                }
            }
            UiMessageData::Widget(msg) => {
                if let &WidgetMessage::Drop(node) = msg {
                    if ui.is_node_child_of(node, self.root) && ui.is_node_child_of(message.destination, self.root) && node != message.destination {
                        let child = self.map_tree_to_node(node, ui);
                        let parent = self.map_tree_to_node(message.destination, ui);
                        if child.is_some() && parent.is_some() {
                            self.sender
                                .send(Message::DoSceneCommand(SceneCommand::LinkNodes(LinkNodesCommand::new(child, parent))))
                                .unwrap();
                        }
                    }
                }
            }
            UiMessageData::Button(msg) => {
                if let ButtonMessage::Click = msg {}
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
            _ => ()
        }
    }

    fn map_node_to_tree(&self, ui: &Ui, node: Handle<Node>) -> Handle<UiNode> {
        let mut stack = vec![self.root];
        while let Some(tree_handle) = stack.pop() {
            match ui.node(tree_handle) {
                UiNode::Tree(tree) => {
                    let handle = *tree.user_data.as_ref().unwrap().downcast_ref::<Handle<Node>>().unwrap();
                    if handle == node {
                        return tree_handle;
                    } else {
                        stack.extend_from_slice(tree.items());
                    }
                }
                UiNode::TreeRoot(root) => {
                    stack.extend_from_slice(root.items());
                }
                _ => unreachable!()
            }
        }
        Handle::NONE
    }
}