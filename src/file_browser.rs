use std::{
    path::{PathBuf, Path},
    ops::{Deref, DerefMut},
    rc::Rc,
};
use crate::{
    grid::{GridBuilder, Column, Row},
    text_box::TextBoxBuilder,
    text::TextBuilder,
    tree::{TreeBuilder, TreeRootBuilder},
    message::{
        UiMessage,
        UiMessageData,
        FileBrowserMessage,
        TreeRootMessage,
        TextBoxMessage
    },
    node::UINode,
    widget::{Widget, WidgetBuilder},
    Control,
    NodeHandleMapping,
    UserInterface,
    core::pool::Handle,
};
use crate::message::TreeMessage;

pub struct FileBrowser<M: 'static, C: 'static + Control<M, C>> {
    widget: Widget<M, C>,
    tree_root: Handle<UINode<M, C>>,
    path: PathBuf,
    path_text: Handle<UINode<M, C>>,
}

impl<M: 'static, C: 'static + Control<M, C>> Deref for FileBrowser<M, C> {
    type Target = Widget<M, C>;

    fn deref(&self) -> &Self::Target {
        &self.widget
    }
}

impl<M: 'static, C: 'static + Control<M, C>> DerefMut for FileBrowser<M, C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.widget
    }
}

impl<M: 'static, C: 'static + Control<M, C>> Clone for FileBrowser<M, C> {
    fn clone(&self) -> Self {
        Self {
            widget: self.widget.raw_copy(),
            tree_root: self.tree_root,
            path: self.path.clone(),
            path_text: self.path_text
        }
    }
}

impl<M, C: 'static + Control<M, C>> Control<M, C> for FileBrowser<M, C> {
    fn raw_copy(&self) -> UINode<M, C> {
        UINode::FileBrowser(self.clone())
    }

    fn resolve(&mut self, node_map: &NodeHandleMapping<M, C>) {
        self.tree_root = *node_map.get(&self.tree_root).unwrap();
        self.path_text = *node_map.get(&self.path_text).unwrap();
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface<M, C>, message: &mut UiMessage<M, C>) {
        self.widget.handle_routed_message(ui, message);

        match &message.data {
            UiMessageData::FileBrowser(msg) => {
                if message.destination == self.handle {
                    match msg {
                        FileBrowserMessage::Path(_) => {

                        }
                        FileBrowserMessage::SelectionChanged(_) => {}
                    }
                }
            }
            UiMessageData::TextBox(msg) => {
                if message.destination == self.path_text {
                    if let TextBoxMessage::Text(txt) = msg {
                        // Try to find tree corresponding to path.
                        let tree = find_tree(self.tree_root, txt, ui);
                        if tree.is_some() {
                            if let UINode::TreeRoot(root) = ui.node_mut(self.tree_root) {
                                root.set_selected(tree);
                            } else {
                                panic!("must be tree root");
                            }
                        }
                    }
                }
            }
            UiMessageData::Tree(msg) => {
                if let &TreeMessage::Expand(expand) = msg {
                    // Look into internals of directory and build tree items.
                    if expand {
                        if let UINode::Tree(tree) = ui.node(message.destination) {
                            let path = tree.user_data
                                .as_ref()
                                .unwrap()
                                .downcast_ref::<PathBuf>()
                                .unwrap();
                            if let Ok(dir_iter) = std::fs::read_dir(path) {
                                for p in dir_iter {
                                    if let Ok(entry) = p {
                                        build_tree(message.destination, false, &entry.path(), ui, false);
                                    }
                                }
                            }
                        } else {
                            panic!("must be tree");
                        }
                    } else {
                        // Nuke everything in collapsed item. This also will free some resources
                        // and will speed up layout pass.

                    }
                }
            }
            UiMessageData::TreeRoot(msg) => {
                if message.destination == self.tree_root {
                    if let &TreeRootMessage::SetSelected(selection) = msg {
                        let path = ui.node(selection).user_data.as_ref().unwrap().downcast_ref::<PathBuf>().unwrap().clone();
                        if let UINode::TextBox(path_text) = ui.node_mut(self.path_text) {
                            path_text.set_text(path.to_string_lossy());
                        } else {
                            panic!("must be text box");
                        }
                        self.send_message(UiMessage {
                            handled: false,
                            data: UiMessageData::FileBrowser(FileBrowserMessage::SelectionChanged(path)),
                            destination: self.handle,
                        });
                    }
                }
            }
            _ => {}
        }
    }

    fn remove_ref(&mut self, handle: Handle<UINode<M, C>>) {
        if self.tree_root == handle {
            self.tree_root = Handle::NONE;
        }
        if self.path_text == handle {
            self.path_text = Handle::NONE;
        }
    }
}

fn find_tree<M: 'static, C: 'static + Control<M, C>, P: AsRef<Path>>(node: Handle<UINode<M, C>>, path: &P, ui: &UserInterface<M, C>) -> Handle<UINode<M, C>>{
    let mut tree_handle = Handle::NONE;
    match ui.node(node) {
        UINode::Tree(tree) => {
            let tree_path = tree.user_data.as_ref().unwrap().downcast_ref::<PathBuf>().unwrap();
            if tree_path.to_string_lossy().starts_with(path.as_ref().to_string_lossy().deref()) {
                tree_handle = node;
            }
            for &item in tree.items() {
                let tree = find_tree(item, path, ui);
                if tree.is_some() {
                    tree_handle = tree;
                    break;
                }
            }
        },
        UINode::TreeRoot(root) => {
            for &item in root.items() {
                let tree = find_tree(item, path, ui);
                if tree.is_some() {
                    tree_handle = tree;
                    break;
                }
            }
        },
        _ => unreachable!()
    }
    tree_handle
}

fn build_tree<M: 'static, C: 'static + Control<M, C>>(parent: Handle<UINode<M, C>>, is_parent_root: bool, path: &Path, ui: &mut UserInterface<M, C>, recursive: bool) -> Handle<UINode<M, C>> {
    let tree = TreeBuilder::new(WidgetBuilder::new()
        .with_user_data(Rc::new(path.to_owned())))
        .with_expanded(false)
        .with_always_show_expander(true)
        .with_content(TextBuilder::new(WidgetBuilder::new())
            .with_text(path.to_string_lossy())
            .build(ui))
        .build(ui);

    if is_parent_root {
        ui.send_message(UiMessage {
            handled: false,
            data: UiMessageData::TreeRoot(TreeRootMessage::AddItem(tree)),
            destination: parent
        });
    } else {
        ui.send_message(UiMessage {
            handled: false,
            data: UiMessageData::Tree(TreeMessage::AddItem(tree)),
            destination: parent
        });
    }

    // Continue build.
    if recursive {
        if let Ok(dir_iter) = std::fs::read_dir(path) {
            for p in dir_iter {
                if let Ok(entry) = p {
                    build_tree(tree, false, &entry.path(), ui, recursive);
                }
            }
        }
    }

    tree
}

pub struct FileBrowserBuilder<M: 'static, C: 'static + Control<M, C>> {
    widget_builder: WidgetBuilder<M, C>,
    path: PathBuf,
}

impl<M: 'static, C: 'static + Control<M, C>> FileBrowserBuilder<M, C> {
    pub fn new(widget_builder: WidgetBuilder<M, C>) -> Self {
        Self {
            widget_builder,
            path: Default::default(),
        }
    }

    pub fn with_path<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.path = path.as_ref().to_owned();
        self
    }

    pub fn build(self, ui: &mut UserInterface<M, C>) -> Handle<UINode<M, C>> {
        let tree_root;
        let path_text;
        let grid = GridBuilder::new(WidgetBuilder::new()
            .with_child({
                path_text = TextBoxBuilder::new(WidgetBuilder::new()
                    .on_row(0)
                    .on_column(0))
                    .with_text("Foobar")
                    .build(ui);
                path_text
            })
            .with_child({
                tree_root = TreeRootBuilder::new(WidgetBuilder::new()
                    .on_row(1)
                    .on_column(0))
                    .build(ui);
                tree_root
            }))
            .add_column(Column::auto())
            .add_row(Row::strict(30.0))
            .add_row(Row::stretch())
            .build(ui);

        build_tree(tree_root, true, &self.path, ui, false);

        let browser = FileBrowser {
            widget: self.widget_builder
                .with_child(grid)
                .build(ui.sender()),
            tree_root,
            path: self.path,
            path_text,
        };

        let handle = ui.add_node(UINode::FileBrowser(browser));

        ui.flush_messages();

        handle
    }
}