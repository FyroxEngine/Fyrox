use crate::{EditorScene, GameEngine, UiNode, Ui, UiMessage, Message};
use rg3d::{
    gui::{
        window::{WindowTitle, WindowBuilder},
        widget::WidgetBuilder,
        tree::{TreeBuilder, TreeRootBuilder},
        text::TextBuilder,
        Thickness,
        message::{UiMessageData, TreeRootMessage},
    },
    scene::node::Node,
    core::{
        pool::Handle,
        math::vec2::Vec2
    }
};
use std::collections::HashMap;
use std::sync::mpsc::Sender;

pub struct WorldOutliner {
    nodes: HashMap<Handle<Node>, Handle<UiNode>>,
    root: Handle<UiNode>,
    sender: Sender<Message>
}

impl WorldOutliner {
    pub fn new(ui: &mut Ui, sender: Sender<Message>) -> Self {
        let root;
        WindowBuilder::new(WidgetBuilder::new()
            .with_width(250.0)
            .with_max_size(Vec2::new(std::f32::INFINITY, 300.0)))
            .with_title(WindowTitle::Text("World Outliner"))
            .with_content({
                root = TreeRootBuilder::new(WidgetBuilder::new())
                    .build(ui);
                root
            })
            .build(ui);

        Self {
            nodes: Default::default(),
            sender,
            root,
        }
    }

    pub fn sync_to_model(&mut self, editor_scene: &EditorScene, engine: &mut GameEngine) {
        let scene = &mut engine.scenes[editor_scene.scene];
        let graph = &mut scene.graph;
        let ui = &mut engine.user_interface;

        if self.nodes.len() != graph.node_count() {
            ui.send_message(UiMessage {
                destination: self.root,
                data: UiMessageData::TreeRoot(TreeRootMessage::SetItems(Vec::new())),
                handled: false
            });

            let mut stack = vec![graph.get_root()];
            while let Some(handle) = stack.pop() {
                let node = &graph[handle];

                let parent = if node.parent().is_none() {
                    self.root
                } else {
                    *self.nodes.get(&node.parent()).unwrap()
                };

                let tree = TreeBuilder::new(WidgetBuilder::new()
                    .with_margin(Thickness::uniform(1.0)))
                    .with_content(TextBuilder::new(WidgetBuilder::new())
                        .with_text(node.name())
                        .build(ui)
                    )
                    .build(ui);

                match ui.node_mut(parent) {
                    UiNode::Tree(parent_tree) => {
                        parent_tree.add_item(tree);
                    }
                    UiNode::TreeRoot(root) => {
                        root.add_item(tree);
                    }
                    _ => ()
                }

                self.nodes.insert(handle, tree);

                for &child in node.children() {
                    stack.push(child);
                }
            }
        }
    }

    pub fn handle_ui_message(&mut self, message: &UiMessage) {
        if let UiMessageData::TreeRoot(msg) = &message.data {
            if let &TreeRootMessage::SetSelected(selection) = msg {
                for (&node, &tree) in self.nodes.iter() {
                    if tree == selection {
                        self.sender.send(Message::SetSelection(node)).unwrap();
                    }
                }
            }
        }
    }

    pub fn handle_message(&mut self, message: &Message, engine: &mut GameEngine) {
        match message {
            &Message::SetSelection(selection) => {
                if let UiNode::TreeRoot(root) = engine.user_interface.node_mut(self.root) {
                    if let Some(&node) = self.nodes.get(&selection) {
                        root.set_selected(node);
                    }
                }
            },
            _ => ()
        }
    }
}