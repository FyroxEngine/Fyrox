//! File browser is a tree view over file system. It allows to select file or folder.
//!
//! File selector is dialog window with file browser, it somewhat similar to standard
//! OS file selector.

use crate::message::{MessageData, MessageDirection};
use crate::{
    button::ButtonBuilder,
    core::{
        math::{vec2::Vec2, Rect},
        pool::Handle,
    },
    draw::DrawingContext,
    grid::{Column, GridBuilder, Row},
    message::{
        ButtonMessage, FileBrowserMessage, FileSelectorMessage, OsEvent, ScrollViewerMessage,
        TextBoxMessage, TreeMessage, TreeRootMessage, UiMessage, UiMessageData, WidgetMessage,
        WindowMessage,
    },
    node::UINode,
    scroll_viewer::ScrollViewerBuilder,
    stack_panel::StackPanelBuilder,
    text::TextBuilder,
    text_box::TextBoxBuilder,
    tree::{Tree, TreeBuilder, TreeRootBuilder},
    widget::{Widget, WidgetBuilder},
    window::{Window, WindowBuilder, WindowTitle},
    BuildContext, Control, HorizontalAlignment, NodeHandleMapping, Orientation, Thickness,
    UserInterface, VerticalAlignment,
};
use std::{
    cell::RefCell,
    collections::HashMap,
    ops::{Deref, DerefMut},
    path::{Component, Path, PathBuf, Prefix},
    rc::Rc,
};
use sysinfo::{DiskExt, SystemExt};

pub type Filter = dyn FnMut(&Path) -> bool;

#[derive(Clone)]
pub struct FileBrowser<M: MessageData, C: Control<M, C>> {
    widget: Widget<M, C>,
    tree_root: Handle<UINode<M, C>>,
    path_text: Handle<UINode<M, C>>,
    scroll_viewer: Handle<UINode<M, C>>,
    path: PathBuf,
    filter: Option<Rc<RefCell<Filter>>>,
}

impl<M: MessageData, C: Control<M, C>> Deref for FileBrowser<M, C> {
    type Target = Widget<M, C>;

    fn deref(&self) -> &Self::Target {
        &self.widget
    }
}

impl<M: MessageData, C: Control<M, C>> DerefMut for FileBrowser<M, C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.widget
    }
}

impl<M: MessageData, C: Control<M, C>> Control<M, C> for FileBrowser<M, C> {
    fn resolve(&mut self, node_map: &NodeHandleMapping<M, C>) {
        self.tree_root = *node_map.get(&self.tree_root).unwrap();
        self.path_text = *node_map.get(&self.path_text).unwrap();
        self.scroll_viewer = *node_map.get(&self.scroll_viewer).unwrap();
    }

    fn handle_routed_message(
        &mut self,
        ui: &mut UserInterface<M, C>,
        message: &mut UiMessage<M, C>,
    ) {
        self.widget.handle_routed_message(ui, message);

        match &message.data() {
            UiMessageData::FileBrowser(msg) => {
                if message.destination() == self.handle() {
                    match msg {
                        FileBrowserMessage::Path(path) => {
                            if &self.path != path {
                                let mut item = find_tree(self.tree_root, path, ui);
                                if item.is_none() {
                                    // Generate new tree contents.
                                    let (items, path_item) =
                                        build_all(path, self.filter.clone(), &mut ui.build_ctx());

                                    // Replace tree contents.
                                    ui.send_message(TreeRootMessage::items(
                                        self.tree_root,
                                        MessageDirection::ToWidget,
                                        items,
                                    ));

                                    item = path_item;
                                }

                                self.path = path.clone();

                                // Set value of text field.
                                ui.send_message(TextBoxMessage::text(
                                    self.path_text,
                                    MessageDirection::ToWidget,
                                    path.to_string_lossy().to_string(),
                                ));

                                // Path can be invalid, so we shouldn't do anything in such case.
                                if item.is_some() {
                                    // Select item of new path.
                                    ui.send_message(TreeRootMessage::select(
                                        self.tree_root,
                                        MessageDirection::ToWidget,
                                        vec![item],
                                    ));
                                    // Bring item of new path into view.
                                    ui.send_message(ScrollViewerMessage::bring_into_view(
                                        self.scroll_viewer,
                                        MessageDirection::ToWidget,
                                        item,
                                    ));
                                }
                            }
                        }
                    }
                }
            }
            UiMessageData::TextBox(msg) => {
                if message.destination() == self.path_text {
                    if let TextBoxMessage::Text(txt) = msg {
                        self.path = txt.into();
                    }
                }
            }
            UiMessageData::Tree(msg) => {
                if let TreeMessage::Expand(expand) = *msg {
                    if expand {
                        // Look into internals of directory and build tree items.
                        let parent_path = ui
                            .node(message.destination())
                            .user_data_ref::<PathBuf>()
                            .clone();
                        if let Ok(dir_iter) = std::fs::read_dir(&parent_path) {
                            for p in dir_iter {
                                if let Ok(entry) = p {
                                    let path = entry.path();
                                    let build = if let Some(filter) = self.filter.as_ref() {
                                        filter.deref().borrow_mut().deref_mut()(&path)
                                    } else {
                                        true
                                    };
                                    if build {
                                        build_tree(
                                            message.destination(),
                                            false,
                                            &path,
                                            &parent_path,
                                            ui,
                                        );
                                    }
                                }
                            }
                        }
                    } else {
                        // Nuke everything in collapsed item. This also will free some resources
                        // and will speed up layout pass.
                        ui.send_message(TreeMessage::set_items(
                            message.destination(),
                            MessageDirection::ToWidget,
                            vec![],
                        ));
                    }
                }
            }
            UiMessageData::TreeRoot(msg) => {
                if message.destination() == self.tree_root {
                    if let TreeRootMessage::Selected(selection) = msg {
                        if let Some(&first_selected) = selection.first() {
                            let path = ui.node(first_selected).user_data_ref::<PathBuf>();
                            if &self.path != path {
                                ui.send_message(FileBrowserMessage::path(
                                    self.handle,
                                    MessageDirection::ToWidget,
                                    path.as_path().to_owned(),
                                ));
                            }
                        }
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

fn find_tree<M: MessageData, C: Control<M, C>, P: AsRef<Path>>(
    node: Handle<UINode<M, C>>,
    path: &P,
    ui: &UserInterface<M, C>,
) -> Handle<UINode<M, C>> {
    let mut tree_handle = Handle::NONE;
    match ui.node(node) {
        UINode::Tree(tree) => {
            let tree_path = tree.user_data_ref::<PathBuf>();
            if tree_path == path.as_ref() {
                tree_handle = node;
            }
            for &item in tree.items() {
                let tree = find_tree(item, path, ui);
                if tree.is_some() {
                    tree_handle = tree;
                    break;
                }
            }
        }
        UINode::TreeRoot(root) => {
            for &item in root.items() {
                let tree = find_tree(item, path, ui);
                if tree.is_some() {
                    tree_handle = tree;
                    break;
                }
            }
        }
        _ => unreachable!(),
    }
    tree_handle
}

fn build_tree_item<M: MessageData, C: Control<M, C>, P: AsRef<Path>>(
    path: P,
    parent_path: P,
    ctx: &mut BuildContext<M, C>,
) -> Handle<UINode<M, C>> {
    let is_dir_empty = path
        .as_ref()
        .read_dir()
        .map_or(true, |mut f| f.next().is_none());
    TreeBuilder::new(WidgetBuilder::new().with_user_data(Rc::new(path.as_ref().to_owned())))
        .with_expanded(false)
        .with_always_show_expander(!is_dir_empty)
        .with_content(
            TextBuilder::new(WidgetBuilder::new())
                .with_text(
                    path.as_ref()
                        .to_string_lossy()
                        .replace(&parent_path.as_ref().to_string_lossy().to_string(), "")
                        .replace("\\", ""),
                )
                .build(ctx),
        )
        .build(ctx)
}

fn build_tree<M: MessageData, C: Control<M, C>, P: AsRef<Path>>(
    parent: Handle<UINode<M, C>>,
    is_parent_root: bool,
    path: P,
    parent_path: P,
    ui: &mut UserInterface<M, C>,
) -> Handle<UINode<M, C>> {
    let tree = build_tree_item(path, parent_path, &mut ui.build_ctx());

    if is_parent_root {
        ui.send_message(TreeRootMessage::add_item(
            parent,
            MessageDirection::ToWidget,
            tree,
        ));
    } else {
        ui.send_message(TreeMessage::add_item(
            parent,
            MessageDirection::ToWidget,
            tree,
        ));
    }

    tree
}

/// Builds entire file system tree to given final_path.
fn build_all<M: MessageData, C: Control<M, C>>(
    final_path: &Path,
    filter: Option<Rc<RefCell<Filter>>>,
    ctx: &mut BuildContext<M, C>,
) -> (Vec<Handle<UINode<M, C>>>, Handle<UINode<M, C>>) {
    // Create items for disks.
    let mut root_items = HashMap::new();
    for disk in sysinfo::System::new_all()
        .get_disks()
        .iter()
        .map(|i| i.get_mount_point().to_string_lossy())
    {
        root_items.insert(
            disk.chars().next().unwrap() as u8,
            build_tree_item(disk.as_ref(), "", ctx),
        );
    }

    let mut path_item = Handle::NONE;

    // Try to build tree only for given path.
    if let Ok(absolute_path) = final_path.canonicalize() {
        if absolute_path.has_root() {
            let components = absolute_path.components().collect::<Vec<Component>>();
            if let Component::Prefix(prefix) = components.get(0).unwrap() {
                if let Prefix::Disk(disk) | Prefix::VerbatimDisk(disk) = prefix.kind() {
                    if let Some(&root) = root_items.get(&disk) {
                        let mut parent = root;
                        let mut full_path = PathBuf::from(prefix.as_os_str());
                        for (i, component) in components.iter().enumerate().skip(1) {
                            // Concat parts of path one by one,
                            full_path = full_path.join(component.as_os_str());
                            let next = components.get(i + 1).map(|p| full_path.join(p));

                            let mut new_parent = parent;
                            if let Ok(dir_iter) = std::fs::read_dir(&full_path) {
                                for p in dir_iter {
                                    if let Ok(entry) = p {
                                        let path = entry.path();
                                        if filter.as_ref().map_or(true, |f| {
                                            f.deref().borrow_mut().deref_mut()(&path)
                                        }) {
                                            let item = build_tree_item(&path, &full_path, ctx);
                                            Tree::add_item(parent, item, ctx);
                                            if let Some(next) = next.as_ref() {
                                                if *next == path {
                                                    new_parent = item;
                                                }
                                            }

                                            if path == absolute_path {
                                                path_item = item;
                                            }
                                        }
                                    }
                                }
                            }
                            parent = new_parent;
                        }
                    }
                }
            }
        }
    }

    (root_items.values().copied().collect(), path_item)
}

pub struct FileBrowserBuilder<M: MessageData, C: Control<M, C>> {
    widget_builder: WidgetBuilder<M, C>,
    path: PathBuf,
    filter: Option<Rc<RefCell<Filter>>>,
}

impl<M: MessageData, C: Control<M, C>> FileBrowserBuilder<M, C> {
    pub fn new(widget_builder: WidgetBuilder<M, C>) -> Self {
        Self {
            widget_builder,
            path: Default::default(),
            filter: None,
        }
    }

    pub fn with_filter(mut self, filter: Rc<RefCell<Filter>>) -> Self {
        self.filter = Some(filter);
        self
    }

    pub fn with_opt_filter(mut self, filter: Option<Rc<RefCell<Filter>>>) -> Self {
        self.filter = filter;
        self
    }

    /// Sets desired path which will be used to build file system tree.
    ///
    /// # Notes
    ///
    /// It does **not** bring tree item with given path into view because it is impossible
    /// during construction stage - there is not enough layout information to do so. You
    /// can send FileBrowserMessage::Path right after creation and it will bring tree item
    /// into view without any problems. It is possible because all widgets were created at
    /// that moment and layout system can give correct offsets to bring item into view.  
    pub fn with_path<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.path = path.as_ref().to_owned();
        self
    }

    pub fn build(self, ctx: &mut BuildContext<M, C>) -> Handle<UINode<M, C>> {
        let (items, _) = build_all(self.path.as_path(), self.filter.clone(), ctx);

        let path_text;
        let tree_root;
        let scroll_viewer = ScrollViewerBuilder::new(WidgetBuilder::new().on_row(1).on_column(0))
            .with_content({
                tree_root = TreeRootBuilder::new(WidgetBuilder::new())
                    .with_items(items)
                    .build(ctx);
                tree_root
            })
            .build(ctx);

        let grid = GridBuilder::new(
            WidgetBuilder::new()
                .with_child({
                    path_text = TextBoxBuilder::new(
                        WidgetBuilder::new()
                            .on_row(0)
                            .on_column(0)
                            .with_margin(Thickness::uniform(1.0)),
                    )
                    .with_vertical_text_alignment(VerticalAlignment::Center)
                    .with_text(self.path.to_string_lossy().as_ref())
                    .build(ctx);
                    path_text
                })
                .with_child(scroll_viewer),
        )
        .add_column(Column::auto())
        .add_row(Row::strict(30.0))
        .add_row(Row::stretch())
        .build(ctx);

        let browser = FileBrowser {
            widget: self.widget_builder.with_child(grid).build(),
            tree_root,
            path_text,
            path: self.path,
            filter: self.filter,
            scroll_viewer,
        };

        ctx.add_node(UINode::FileBrowser(browser))
    }
}

/// File selector is a modal window that allows you to select a file (or directory) and commit or
/// cancel selection.
#[derive(Clone)]
pub struct FileSelector<M: MessageData, C: Control<M, C>> {
    window: Window<M, C>,
    browser: Handle<UINode<M, C>>,
    ok: Handle<UINode<M, C>>,
    cancel: Handle<UINode<M, C>>,
}

impl<M: MessageData, C: Control<M, C>> Deref for FileSelector<M, C> {
    type Target = Widget<M, C>;

    fn deref(&self) -> &Self::Target {
        &self.window
    }
}

impl<M: MessageData, C: Control<M, C>> DerefMut for FileSelector<M, C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.window
    }
}

// File selector extends Window widget so it delegates most of calls
// to inner window.
impl<M: MessageData, C: Control<M, C>> Control<M, C> for FileSelector<M, C> {
    fn resolve(&mut self, node_map: &NodeHandleMapping<M, C>) {
        self.window.resolve(node_map);
        self.ok = *node_map.get(&self.ok).unwrap();
        self.cancel = *node_map.get(&self.cancel).unwrap();
    }

    fn measure_override(&self, ui: &UserInterface<M, C>, available_size: Vec2) -> Vec2 {
        self.window.arrange_override(ui, available_size)
    }

    fn arrange_override(&self, ui: &UserInterface<M, C>, final_size: Vec2) -> Vec2 {
        self.window.arrange_override(ui, final_size)
    }

    fn arrange(&self, ui: &UserInterface<M, C>, final_rect: &Rect<f32>) {
        self.window.arrange(ui, final_rect);
    }

    fn is_measure_valid(&self, ui: &UserInterface<M, C>) -> bool {
        self.window.is_measure_valid(ui)
    }

    fn is_arrange_valid(&self, ui: &UserInterface<M, C>) -> bool {
        self.window.is_arrange_valid(ui)
    }

    fn measure(&self, ui: &UserInterface<M, C>, available_size: Vec2) {
        self.window.measure(ui, available_size)
    }

    fn draw(&self, drawing_context: &mut DrawingContext) {
        self.window.draw(drawing_context)
    }

    fn update(&mut self, dt: f32) {
        self.window.update(dt);
    }

    fn handle_routed_message(
        &mut self,
        ui: &mut UserInterface<M, C>,
        message: &mut UiMessage<M, C>,
    ) {
        self.window.handle_routed_message(ui, message);

        match &message.data() {
            UiMessageData::Button(msg) => {
                if let ButtonMessage::Click = msg {
                    if message.destination() == self.ok {
                        let path = if let UINode::FileBrowser(browser) = ui.node(self.browser) {
                            &browser.path
                        } else {
                            unreachable!();
                        };
                        ui.send_message(FileSelectorMessage::commit(
                            self.handle,
                            MessageDirection::ToWidget,
                            path.clone(),
                        ));
                    } else if message.destination() == self.cancel {
                        ui.send_message(FileSelectorMessage::cancel(
                            self.handle,
                            MessageDirection::ToWidget,
                        ))
                    }
                }
            }
            UiMessageData::FileSelector(msg) => {
                if message.destination() == self.handle {
                    match msg {
                        FileSelectorMessage::Commit(_) | FileSelectorMessage::Cancel => ui
                            .send_message(WindowMessage::close(
                                self.handle,
                                MessageDirection::ToWidget,
                            )),
                        FileSelectorMessage::Path(path) => {
                            ui.send_message(FileBrowserMessage::path(
                                self.browser,
                                MessageDirection::ToWidget,
                                path.clone(),
                            ))
                        }
                    }
                }
            }
            UiMessageData::Window(msg) => {
                if message.destination() == self.handle {
                    if let WindowMessage::Open | WindowMessage::OpenModal = msg {
                        ui.send_message(WidgetMessage::center(
                            self.handle,
                            MessageDirection::ToWidget,
                        ));
                    }
                }
            }
            _ => {}
        }
    }

    fn preview_message(&mut self, ui: &mut UserInterface<M, C>, message: &mut UiMessage<M, C>) {
        self.window.preview_message(ui, message);
    }

    fn handle_os_event(
        &mut self,
        self_handle: Handle<UINode<M, C>>,
        ui: &mut UserInterface<M, C>,
        event: &OsEvent,
    ) {
        self.window.handle_os_event(self_handle, ui, event);
    }

    fn remove_ref(&mut self, handle: Handle<UINode<M, C>>) {
        self.window.remove_ref(handle)
    }
}

pub struct FileSelectorBuilder<M: MessageData, C: Control<M, C>> {
    window_builder: WindowBuilder<M, C>,
    filter: Option<Rc<RefCell<Filter>>>,
    path: PathBuf,
}

impl<M: MessageData, C: Control<M, C>> FileSelectorBuilder<M, C> {
    pub fn new(window_builder: WindowBuilder<M, C>) -> Self {
        Self {
            window_builder,
            filter: None,
            path: Default::default(),
        }
    }

    pub fn with_filter(mut self, filter: Rc<RefCell<Filter>>) -> Self {
        self.filter = Some(filter);
        self
    }

    pub fn with_path<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.path = path.as_ref().to_owned();
        self
    }

    pub fn build(mut self, ctx: &mut BuildContext<M, C>) -> Handle<UINode<M, C>> {
        let browser;
        let ok;
        let cancel;

        if self.window_builder.title.is_none() {
            self.window_builder.title = Some(WindowTitle::text("Select File"));
        }

        let window = self
            .window_builder
            .with_content(
                GridBuilder::new(
                    WidgetBuilder::new()
                        .with_child({
                            browser = FileBrowserBuilder::new(WidgetBuilder::new().on_column(0))
                                .with_opt_filter(self.filter)
                                .with_path(self.path)
                                .build(ctx);
                            browser
                        })
                        .with_child(
                            StackPanelBuilder::new(
                                WidgetBuilder::new()
                                    .with_horizontal_alignment(HorizontalAlignment::Right)
                                    .on_column(0)
                                    .on_row(1)
                                    .with_child({
                                        ok = ButtonBuilder::new(
                                            WidgetBuilder::new()
                                                .with_margin(Thickness::uniform(1.0))
                                                .with_width(100.0)
                                                .with_height(30.0),
                                        )
                                        .with_text("OK")
                                        .build(ctx);
                                        ok
                                    })
                                    .with_child({
                                        cancel = ButtonBuilder::new(
                                            WidgetBuilder::new()
                                                .with_margin(Thickness::uniform(1.0))
                                                .with_width(100.0)
                                                .with_height(30.0),
                                        )
                                        .with_text("Cancel")
                                        .build(ctx);
                                        cancel
                                    }),
                            )
                            .with_orientation(Orientation::Horizontal)
                            .build(ctx),
                        ),
                )
                .add_column(Column::auto())
                .add_row(Row::stretch())
                .add_row(Row::auto())
                .build(ctx),
            )
            .build_window(ctx);

        let file_selector = FileSelector {
            window,
            browser,
            ok,
            cancel,
        };

        ctx.add_node(UINode::FileSelector(file_selector))
    }
}
