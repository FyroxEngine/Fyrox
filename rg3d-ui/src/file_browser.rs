//! File browser is a tree view over file system. It allows to select file or folder.
//!
//! File selector is dialog window with file browser, it somewhat similar to standard
//! OS file selector.

use crate::core::algebra::Vector2;
use crate::{
    button::ButtonBuilder,
    core::{math::Rect, pool::Handle},
    draw::DrawingContext,
    grid::{Column, GridBuilder, Row},
    message::{
        ButtonMessage, FileBrowserMessage, FileSelectorMessage, MessageData, MessageDirection,
        OsEvent, ScrollViewerMessage, TextBoxMessage, TreeMessage, TreeRootMessage, UiMessage,
        UiMessageData, WindowMessage,
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
    ops::{Deref, DerefMut},
    path::{Component, Path, PathBuf, Prefix},
    rc::Rc,
};
use sysinfo::{DiskExt, RefreshKind, SystemExt};

pub type Filter = dyn FnMut(&Path) -> bool;

#[derive(Clone)]
pub struct FileBrowser<M: MessageData, C: Control<M, C>> {
    widget: Widget<M, C>,
    tree_root: Handle<UINode<M, C>>,
    path_text: Handle<UINode<M, C>>,
    scroll_viewer: Handle<UINode<M, C>>,
    path: PathBuf,
    root: Option<PathBuf>,
    filter: Option<Rc<RefCell<Filter>>>,
}

crate::define_widget_deref!(FileBrowser<M, C>);

impl<M: MessageData, C: Control<M, C>> Control<M, C> for FileBrowser<M, C> {
    fn resolve(&mut self, node_map: &NodeHandleMapping<M, C>) {
        node_map.resolve(&mut self.tree_root);
        node_map.resolve(&mut self.path_text);
        node_map.resolve(&mut self.scroll_viewer);
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
                                    let result = build_all(
                                        self.root.as_ref(),
                                        path,
                                        self.filter.clone(),
                                        &mut ui.build_ctx(),
                                    );

                                    // Replace tree contents.
                                    ui.send_message(TreeRootMessage::items(
                                        self.tree_root,
                                        MessageDirection::ToWidget,
                                        result.root_items,
                                    ));

                                    item = result.path_item;
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
                        FileBrowserMessage::Root(root) => {
                            if &self.root != root {
                                self.root = root.clone();
                                self.path = root.clone().unwrap_or_default();

                                // Generate new tree contents.
                                let result = build_all(
                                    self.root.as_ref(),
                                    &self.path,
                                    self.filter.clone(),
                                    &mut ui.build_ctx(),
                                );

                                // Replace tree contents.
                                ui.send_message(TreeRootMessage::items(
                                    self.tree_root,
                                    MessageDirection::ToWidget,
                                    result.root_items,
                                ));

                                if result.path_item.is_some() {
                                    // Select item of new path.
                                    ui.send_message(TreeRootMessage::select(
                                        self.tree_root,
                                        MessageDirection::ToWidget,
                                        vec![result.path_item],
                                    ));
                                    // Bring item of new path into view.
                                    ui.send_message(ScrollViewerMessage::bring_into_view(
                                        self.scroll_viewer,
                                        MessageDirection::ToWidget,
                                        result.path_item,
                                    ));
                                } else {
                                    // Clear text field if path is invalid.
                                    ui.send_message(TextBoxMessage::text(
                                        self.path_text,
                                        MessageDirection::ToWidget,
                                        String::new(),
                                    ));
                                }
                            }
                        }
                    }
                }
            }
            UiMessageData::TextBox(msg) => {
                if message.destination() == self.path_text
                    && message.direction() == MessageDirection::FromWidget
                {
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
            TextBuilder::new(WidgetBuilder::new().with_margin(Thickness::left(4.0)))
                .with_text(
                    path.as_ref()
                        .to_string_lossy()
                        .replace(&parent_path.as_ref().to_string_lossy().to_string(), "")
                        .replace("\\", ""),
                )
                .with_vertical_text_alignment(VerticalAlignment::Center)
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

struct BuildResult<M: MessageData, C: Control<M, C>> {
    root_items: Vec<Handle<UINode<M, C>>>,
    path_item: Handle<UINode<M, C>>,
}

/// Builds entire file system tree to given final_path.
fn build_all<M: MessageData, C: Control<M, C>>(
    root: Option<&PathBuf>,
    final_path: &Path,
    filter: Option<Rc<RefCell<Filter>>>,
    ctx: &mut BuildContext<M, C>,
) -> BuildResult<M, C> {
    let mut dest_path = PathBuf::new();
    if let Ok(canonical_final_path) = final_path.canonicalize() {
        if let Some(canonical_root) = root.map(|r| r.canonicalize().ok()).flatten() {
            if let Ok(stripped) = canonical_final_path.strip_prefix(canonical_root) {
                dest_path = stripped.to_owned();
            }
        } else {
            dest_path = canonical_final_path;
        }
    }

    let dest_path_components = dest_path.components().collect::<Vec<Component>>();
    let dest_disk = dest_path_components.get(0).and_then(|c| {
        if let Component::Prefix(prefix) = c {
            if let Prefix::Disk(disk_letter) | Prefix::VerbatimDisk(disk_letter) = prefix.kind() {
                Some(disk_letter)
            } else {
                None
            }
        } else {
            None
        }
    });

    let mut root_items = Vec::new();
    let mut parent = if let Some(root) = root {
        let path = if std::env::current_dir().map_or(false, |dir| &dir == root) {
            Path::new(".")
        } else {
            root.as_path()
        };
        let item = build_tree_item(path, &Path::new(""), ctx);
        root_items.push(item);
        item
    } else {
        let mut parent = Handle::NONE;

        // Create items for disks.
        for disk in sysinfo::System::new_with_specifics(RefreshKind::new().with_disks_list())
            .get_disks()
            .iter()
            .map(|i| i.get_mount_point().to_string_lossy())
        {
            let item = build_tree_item(disk.as_ref(), "", ctx);

            let disk_letter = disk.chars().next().unwrap() as u8;

            if let Some(dest_disk) = dest_disk {
                if dest_disk == disk_letter {
                    parent = item;
                }
            }

            root_items.push(item);
        }

        parent
    };

    let mut path_item = Handle::NONE;

    // Try to build tree only for given path.
    let mut full_path = PathBuf::new();
    for (i, component) in dest_path_components.iter().enumerate() {
        // Concat parts of path one by one.
        full_path = full_path.join(component.as_os_str());
        let next = dest_path_components.get(i + 1).map(|p| full_path.join(p));

        let mut new_parent = parent;
        if let Ok(dir_iter) = std::fs::read_dir(&full_path) {
            for p in dir_iter {
                if let Ok(entry) = p {
                    let path = entry.path();
                    if filter
                        .as_ref()
                        .map_or(true, |f| f.deref().borrow_mut().deref_mut()(&path))
                    {
                        let item = build_tree_item(&path, &full_path, ctx);
                        if parent.is_some() {
                            Tree::add_item(parent, item, ctx);
                        } else {
                            root_items.push(item);
                        }
                        if let Some(next) = next.as_ref() {
                            if *next == path {
                                new_parent = item;
                            }
                        }

                        if path == dest_path {
                            path_item = item;
                        }
                    }
                }
            }
        }
        parent = new_parent;
    }

    BuildResult {
        root_items,
        path_item,
    }
}

pub struct FileBrowserBuilder<M: MessageData, C: Control<M, C>> {
    widget_builder: WidgetBuilder<M, C>,
    path: PathBuf,
    filter: Option<Rc<RefCell<Filter>>>,
    root: Option<PathBuf>,
}

impl<M: MessageData, C: Control<M, C>> FileBrowserBuilder<M, C> {
    pub fn new(widget_builder: WidgetBuilder<M, C>) -> Self {
        Self {
            widget_builder,
            path: Default::default(),
            filter: None,
            root: None,
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

    pub fn with_root(mut self, root: PathBuf) -> Self {
        self.root = Some(root);
        self
    }

    pub fn build(self, ctx: &mut BuildContext<M, C>) -> Handle<UINode<M, C>> {
        let BuildResult {
            root_items: items, ..
        } = build_all(
            self.root.as_ref(),
            self.path.as_path(),
            self.filter.clone(),
            ctx,
        );

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
                            .with_margin(Thickness::uniform(2.0)),
                    )
                    .with_vertical_text_alignment(VerticalAlignment::Center)
                    .with_text(self.path.to_string_lossy().as_ref())
                    .build(ctx);
                    path_text
                })
                .with_child(scroll_viewer),
        )
        .add_column(Column::stretch())
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
            root: self.root,
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
        node_map.resolve(&mut self.ok);
        node_map.resolve(&mut self.cancel);
    }

    fn measure_override(
        &self,
        ui: &UserInterface<M, C>,
        available_size: Vector2<f32>,
    ) -> Vector2<f32> {
        self.window.measure_override(ui, available_size)
    }

    fn arrange_override(&self, ui: &UserInterface<M, C>, final_size: Vector2<f32>) -> Vector2<f32> {
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

    fn measure(&self, ui: &UserInterface<M, C>, available_size: Vector2<f32>) {
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
            UiMessageData::Button(ButtonMessage::Click) => {
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
                        FileSelectorMessage::Root(root) => {
                            ui.send_message(FileBrowserMessage::root(
                                self.browser,
                                MessageDirection::ToWidget,
                                root.clone(),
                            ));
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn preview_message(&self, ui: &UserInterface<M, C>, message: &mut UiMessage<M, C>) {
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
                                    .with_margin(Thickness::uniform(1.0))
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
                .add_column(Column::stretch())
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
