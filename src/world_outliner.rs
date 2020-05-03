use crate::{EditorScene, GameEngine, UiNode, Ui};
use rg3d::{
    gui::{
        window::{WindowTitle, WindowBuilder},
        widget::WidgetBuilder,
        tree::TreeBuilder,
        text::TextBuilder,
        border::BorderBuilder
    },
    scene::node::Node,
    core::pool::Handle,
};
use std::collections::HashMap;

pub struct WorldOutliner {
    nodes: HashMap<Handle<Node>, Handle<UiNode>>,
    root: Handle<UiNode>,
}

impl WorldOutliner {
    pub fn new(ui: &mut Ui) -> Self {
        let root;
        WindowBuilder::new(WidgetBuilder::new()
            .with_width(250.0))
            .with_title(WindowTitle::Text("World Outliner"))
            .with_content({
                root = BorderBuilder::new(WidgetBuilder::new())
                    .build(ui);
                root
            })
            .build(ui);

        Self {
            nodes: Default::default(),
            root
        }
    }

    pub fn sync_to_model(&mut self, editor_scene: &EditorScene, engine: &mut GameEngine) {
        let scene = &mut engine.scenes[editor_scene.scene];
        let graph = &mut scene.graph;
        let ui = &mut engine.user_interface;

        if self.nodes.len() != graph.node_count() {
            for child in ui.node(self.root).children().to_vec() {
                ui.remove_node(child);
            }

            let mut stack = vec![graph.get_root()];
            while let Some(handle) = stack.pop() {
                let node = &graph[handle];

                let parent = if node.parent().is_none() {
                    self.root
                } else {
                    *self.nodes.get(&node.parent()).unwrap()
                };

                let tree = TreeBuilder::new(WidgetBuilder::new())
                    .with_content(TextBuilder::new(WidgetBuilder::new())
                        .with_text(node.name())
                        .build(ui))
                    .build(ui);

                if let UiNode::Tree(parent_tree) = ui.node_mut(parent) {
                    parent_tree.add_item(tree);
                } else {
                    ui.link_nodes(tree, parent);
                }

                self.nodes.insert(handle, tree);

                for &child in node.children() {
                    stack.push(child);
                }
            }
        }
    }
}