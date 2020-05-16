use crate::{
    EditorScene,
    GameEngine,
    UiNode,
    Ui,
    UiMessage,
    Message,
    command::{
        Command,
        LinkNodesCommand,
        ChangeSelectionCommand,
    },
};
use rg3d::{
    gui::{
        window::{WindowTitle, WindowBuilder},
        widget::WidgetBuilder,
        tree::{TreeBuilder, TreeRootBuilder},
        text::TextBuilder,
        Thickness,
        message::{UiMessageData, TreeRootMessage, WidgetMessage, TreeMessage},
    },
    scene::node::Node,
    core::{
        pool::Handle,
        math::vec2::Vec2,
    },
};
use std::{
    sync::mpsc::Sender,
    rc::Rc,
};

pub struct WorldOutliner {
    pub window: Handle<UiNode>,
    root: Handle<UiNode>,
    sender: Sender<Message>,
    stack: Vec<(Handle<UiNode>, Handle<Node>)>,
}

fn make_tree(node: &Node, handle: Handle<Node>, ui: &mut Ui) -> Handle<UiNode> {
    TreeBuilder::new(WidgetBuilder::new()
        .with_user_data(Rc::new(handle))
        .with_margin(Thickness::uniform(1.0)))
        .with_content(TextBuilder::new(WidgetBuilder::new())
            .with_text(node.name())
            .build(ui)
        )
        .build(ui)
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
    pub fn new(ui: &mut Ui, sender: Sender<Message>) -> Self {
        let root;
        let window = WindowBuilder::new(WidgetBuilder::new()
            .with_max_size(Vec2::new(std::f32::INFINITY, 300.0)))
            .with_title(WindowTitle::Text("World Outliner"))
            .with_content({
                root = TreeRootBuilder::new(WidgetBuilder::new())
                    .build(ui);
                root
            })
            .build(ui);

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

        // Sync tree structure with graph structure.
        self.stack.clear();
        self.stack.push((self.root, graph.get_root()));
        while let Some((tree_handle, node_handle)) = self.stack.pop() {
            // Hide all editor nodes.
            if node_handle == editor_scene.root {
                continue;
            }
            let node = &graph[node_handle];
            match ui.node_mut(tree_handle) {
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
                                ui.send_message(UiMessage {
                                    destination: tree_handle,
                                    data: UiMessageData::Tree(TreeMessage::RemoveItem(item)),
                                    handled: false,
                                });
                                ui.flush_messages();
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
                                let tree = make_tree(&graph[child_handle], child_handle, ui);
                                ui.send_message(UiMessage {
                                    data: UiMessageData::Tree(TreeMessage::AddItem(tree)),
                                    destination: tree_handle,
                                    handled: false,
                                });
                                ui.flush_messages();
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
                        let tree = make_tree(node, node_handle, ui);
                        ui.send_message(UiMessage {
                            data: UiMessageData::TreeRoot(TreeRootMessage::AddItem(tree)),
                            destination: tree_handle,
                            handled: false,
                        });
                        ui.flush_messages();
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
                    if let &TreeRootMessage::SetSelected(selection) = msg {
                        let node = self.map_tree_to_node(selection, ui);
                        if node != current_selection {
                            self.sender
                                .send(Message::ExecuteCommand(Command::ChangeSelection(ChangeSelectionCommand::new(node, current_selection))))
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
                                .send(Message::ExecuteCommand(Command::LinkNodes(LinkNodesCommand::new(child, parent))))
                                .unwrap();
                        }
                    }
                }
            }
            _ => {}
        }
    }

    pub fn clear(&mut self, ui: &mut Ui) {
        ui.send_message(UiMessage {
            handled: false,
            data: UiMessageData::TreeRoot(TreeRootMessage::SetItems(vec![])),
            destination: self.root,
        });
        ui.flush_messages();
    }

    pub fn handle_message(&mut self, message: &Message, engine: &mut GameEngine) {
        let ui = &engine.user_interface;

        match message {
            &Message::SetSelection(selection) => {
                let tree = self.map_node_to_tree(ui, selection);
                if let UiNode::TreeRoot(root) = engine.user_interface.node_mut(self.root) {
                    root.set_selected(tree);
                }
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
                        for &item in tree.items() {
                            stack.push(item);
                        }
                    }
                }
                UiNode::TreeRoot(root) => {
                    for &item in root.items() {
                        stack.push(item);
                    }
                }
                _ => unreachable!()
            }
        }
        Handle::NONE
    }
}