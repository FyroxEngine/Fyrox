//! File browser is a tree view over file system. It allows to select file or folder.
//!
//! File selector is dialog window with file browser, it somewhat similar to standard
//! OS file selector.

use crate::{
    button::ButtonBuilder,
    core::{algebra::Vector2, pool::Handle},
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
use core::time;
use std::{
    borrow::BorrowMut,
    cell,
    ops::{Deref, DerefMut},
    path::{Component, Path, PathBuf, Prefix},
    rc::Rc,
    sync::{mpsc, Arc, Mutex},
    thread,
};

use crate::text_box::TextCommitMode;
use notify::Watcher;
use std::fmt::{Debug, Formatter};
#[cfg(not(target_arch = "wasm32"))]
use sysinfo::{DiskExt, RefreshKind, SystemExt};

#[derive(Clone)]
pub struct Filter(pub Arc<Mutex<dyn FnMut(&Path) -> bool + Send>>);

impl Filter {
    pub fn new<F: FnMut(&Path) -> bool + 'static + Send>(filter: F) -> Self {
        Self(Arc::new(Mutex::new(filter)))
    }
}

impl PartialEq for Filter {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(&*self.0, &*other.0)
    }
}

impl Debug for Filter {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "Filter")
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum FileBrowserMode {
    Open,
    Save { default_file_name: PathBuf },
}

#[derive(Clone)]
pub struct FileBrowser<M: MessageData, C: Control<M, C>> {
    widget: Widget<M, C>,
    tree_root: Handle<UINode<M, C>>,
    path_text: Handle<UINode<M, C>>,
    scroll_viewer: Handle<UINode<M, C>>,
    path: PathBuf,
    root: Option<PathBuf>,
    filter: Option<Filter>,
    mode: FileBrowserMode,
    file_name: Handle<UINode<M, C>>,
    file_name_value: PathBuf,
    #[allow(clippy::type_complexity)]
    watcher: Rc<cell::Cell<Option<(notify::RecommendedWatcher, thread::JoinHandle<()>)>>>,
}

crate::define_widget_deref!(FileBrowser<M, C>);

impl<M: MessageData, C: Control<M, C>> FileBrowser<M, C> {
    fn rebuild_from_root(&mut self, ui: &mut UserInterface<M, C>) {
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
                            if message.direction() == MessageDirection::ToWidget
                                && &self.path != path
                            {
                                let existing_path = ignore_nonexistent_sub_dirs(path);

                                let mut item = find_tree(self.tree_root, &existing_path, ui);

                                if item.is_none() {
                                    // Generate new tree contents.
                                    let result = build_all(
                                        self.root.as_ref(),
                                        &existing_path,
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

                                ui.send_message(message.reverse());
                            }
                        }
                        FileBrowserMessage::Root(root) => {
                            if &self.root != root {
                                let watcher_replacment = match self.watcher.replace(None) {
                                    Some((mut watcher, converter)) => {
                                        let current_root = match &self.root {
                                            Some(path) => path.clone(),
                                            None => self.path.clone(),
                                        };
                                        let _ = watcher.unwatch(current_root);
                                        let new_root = match &root {
                                            Some(path) => path.clone(),
                                            None => self.path.clone(),
                                        };
                                        let _ = watcher
                                            .watch(new_root, notify::RecursiveMode::Recursive);
                                        Some((watcher, converter))
                                    }
                                    None => None,
                                };
                                self.root = root.clone();
                                self.path = root.clone().unwrap_or_default();
                                self.rebuild_from_root(ui);
                                self.watcher.replace(watcher_replacment);
                            }
                        }
                        FileBrowserMessage::Filter(filter) => {
                            let equal = match (&self.filter, filter) {
                                (Some(current), Some(new)) => std::ptr::eq(&*new, &*current),
                                _ => false,
                            };
                            if !equal {
                                self.filter = filter.clone();
                                self.rebuild_from_root(ui);
                            }
                        }
                        FileBrowserMessage::Add(path) => {
                            if let Some(filter) = self.filter.as_mut() {
                                if !filter.0.borrow_mut().deref_mut().lock().unwrap()(path) {
                                    return;
                                }
                            }
                            let mut parent_path = path.clone();
                            parent_path.pop();
                            let existing_parent_node = find_tree(self.tree_root, &parent_path, ui);
                            if existing_parent_node.is_some() {
                                build_tree(
                                    existing_parent_node,
                                    existing_parent_node == self.tree_root,
                                    path,
                                    &parent_path,
                                    ui,
                                );
                            }
                        }
                        FileBrowserMessage::Remove(_path) => {
                            println!("FileBrowserMessage::Remove Received and Ignored");
                        }
                        FileBrowserMessage::Rescan => {
                            println!("FileBrowserMessage::Rescan Received and Ignored");
                        }
                    }
                }
            }
            UiMessageData::TextBox(TextBoxMessage::Text(txt)) => {
                if message.direction() == MessageDirection::FromWidget {
                    if message.destination() == self.path_text {
                        self.path = txt.into();
                    } else if message.destination() == self.file_name {
                        self.file_name_value = txt.into();
                        ui.send_message(FileBrowserMessage::path(
                            self.handle,
                            MessageDirection::ToWidget,
                            {
                                let mut combined = self.path.clone();
                                combined.set_file_name(PathBuf::from(txt));
                                combined
                            },
                        ));
                    }
                }
            }
            UiMessageData::Tree(TreeMessage::Expand { expand, .. }) => {
                if *expand {
                    // Look into internals of directory and build tree items.
                    let parent_path = ui
                        .node(message.destination())
                        .user_data_ref::<PathBuf>()
                        .unwrap()
                        .clone();
                    if let Ok(dir_iter) = std::fs::read_dir(&parent_path) {
                        for entry in dir_iter.flatten() {
                            let path = entry.path();
                            let build = if let Some(filter) = self.filter.as_mut() {
                                filter.0.borrow_mut().deref_mut().lock().unwrap()(&path)
                            } else {
                                true
                            };
                            if build {
                                build_tree(message.destination(), false, &path, &parent_path, ui);
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
            UiMessageData::TreeRoot(msg) => {
                if message.destination() == self.tree_root
                    && message.direction() == MessageDirection::FromWidget
                {
                    if let TreeRootMessage::Selected(selection) = msg {
                        if let Some(&first_selected) = selection.first() {
                            let mut path = ui
                                .node(first_selected)
                                .user_data_ref::<PathBuf>()
                                .unwrap()
                                .clone();

                            if let FileBrowserMode::Save { .. } = self.mode {
                                if path.is_file() {
                                    ui.send_message(TextBoxMessage::text(
                                        self.file_name,
                                        MessageDirection::ToWidget,
                                        path.file_name()
                                            .map(|f| f.to_string_lossy().to_string())
                                            .unwrap_or_default(),
                                    ));
                                } else {
                                    path = path.join(&self.file_name_value);
                                }
                            }

                            if self.path != path {
                                self.path = path.clone();

                                ui.send_message(TextBoxMessage::text(
                                    self.path_text,
                                    MessageDirection::ToWidget,
                                    path.to_string_lossy().to_string(),
                                ));

                                // Do response.
                                ui.send_message(FileBrowserMessage::path(
                                    self.handle,
                                    MessageDirection::FromWidget,
                                    path,
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

fn ignore_nonexistent_sub_dirs(path: &Path) -> PathBuf {
    let mut existing_path = path.to_owned();
    while !existing_path.exists() {
        if !existing_path.pop() {
            break;
        }
    }
    existing_path
}

fn find_tree<M: MessageData, C: Control<M, C>, P: AsRef<Path>>(
    node: Handle<UINode<M, C>>,
    path: &P,
    ui: &UserInterface<M, C>,
) -> Handle<UINode<M, C>> {
    let mut tree_handle = Handle::NONE;
    match ui.node(node) {
        UINode::Tree(tree) => {
            let tree_path = tree.user_data_ref::<PathBuf>().unwrap();
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
    let subtree = build_tree_item(path, parent_path, &mut ui.build_ctx());
    insert_subtree_in_parent(ui, parent, is_parent_root, subtree);
    subtree
}

fn insert_subtree_in_parent<M: MessageData, C: Control<M, C>>(
    ui: &mut UserInterface<M, C>,
    parent: Handle<UINode<M, C>>,
    is_parent_root: bool,
    tree: Handle<UINode<M, C>>,
) {
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
}

struct BuildResult<M: MessageData, C: Control<M, C>> {
    root_items: Vec<Handle<UINode<M, C>>>,
    path_item: Handle<UINode<M, C>>,
}

/// Builds entire file system tree to given final_path.
fn build_all<M: MessageData, C: Control<M, C>>(
    root: Option<&PathBuf>,
    final_path: &Path,
    mut filter: Option<Filter>,
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
    #[allow(unused_variables)]
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
        let item = build_tree_item(path, Path::new(""), ctx);
        root_items.push(item);
        item
    } else {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut parent = Handle::NONE;

            // Create items for disks.
            for disk in sysinfo::System::new_with_specifics(RefreshKind::new().with_disks_list())
                .disks()
                .iter()
                .map(|i| i.mount_point().to_string_lossy())
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
        }

        #[cfg(target_arch = "wasm32")]
        {
            Handle::NONE
        }
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
            for entry in dir_iter.flatten() {
                let path = entry.path();
                #[allow(clippy::blocks_in_if_conditions)]
                if filter.as_mut().map_or(true, |f| {
                    f.0.borrow_mut().deref_mut().lock().unwrap()(&path)
                }) {
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
    filter: Option<Filter>,
    root: Option<PathBuf>,
    mode: FileBrowserMode,
}

impl<M: MessageData, C: Control<M, C>> FileBrowserBuilder<M, C> {
    pub fn new(widget_builder: WidgetBuilder<M, C>) -> Self {
        Self {
            widget_builder,
            path: Default::default(),
            filter: None,
            root: None,
            mode: FileBrowserMode::Open,
        }
    }

    pub fn with_filter(mut self, filter: Filter) -> Self {
        self.filter = Some(filter);
        self
    }

    pub fn with_opt_filter(mut self, filter: Option<Filter>) -> Self {
        self.filter = filter;
        self
    }

    pub fn with_mode(mut self, mode: FileBrowserMode) -> Self {
        self.mode = mode;
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

    pub fn with_opt_root(mut self, root: Option<PathBuf>) -> Self {
        self.root = root;
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
        let scroll_viewer = ScrollViewerBuilder::new(
            WidgetBuilder::new()
                .on_row(match self.mode {
                    FileBrowserMode::Open => 1,
                    FileBrowserMode::Save { .. } => 2,
                })
                .on_column(0),
        )
        .with_content({
            tree_root = TreeRootBuilder::new(WidgetBuilder::new())
                .with_items(items)
                .build(ctx);
            tree_root
        })
        .build(ctx);

        let grid = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(
                    GridBuilder::new(
                        WidgetBuilder::new()
                            .with_child(
                                TextBuilder::new(
                                    WidgetBuilder::new()
                                        .on_row(0)
                                        .on_column(0)
                                        .with_vertical_alignment(VerticalAlignment::Center)
                                        .with_margin(Thickness::uniform(1.0)),
                                )
                                .with_text("Path:")
                                .build(ctx),
                            )
                            .with_child({
                                path_text = TextBoxBuilder::new(
                                    WidgetBuilder::new()
                                        // Disable path if we're in Save mode
                                        .with_enabled(matches!(self.mode, FileBrowserMode::Open))
                                        .on_row(0)
                                        .on_column(1)
                                        .with_margin(Thickness::uniform(2.0)),
                                )
                                .with_text_commit_mode(TextCommitMode::Immediate)
                                .with_vertical_text_alignment(VerticalAlignment::Center)
                                .with_text(self.path.to_string_lossy().as_ref())
                                .build(ctx);
                                path_text
                            }),
                    )
                    .add_row(Row::stretch())
                    .add_column(Column::strict(80.0))
                    .add_column(Column::stretch())
                    .build(ctx),
                )
                .with_child(scroll_viewer),
        )
        .add_column(Column::stretch())
        .add_rows(match self.mode {
            FileBrowserMode::Open => {
                vec![Row::strict(24.0), Row::stretch()]
            }
            FileBrowserMode::Save { .. } => {
                vec![Row::strict(24.0), Row::strict(24.0), Row::stretch()]
            }
        })
        .build(ctx);

        let file_name = match self.mode {
            FileBrowserMode::Save {
                ref default_file_name,
            } => {
                let file_name;
                let name_grid = GridBuilder::new(
                    WidgetBuilder::new()
                        .on_row(1)
                        .on_column(0)
                        .with_child(
                            TextBuilder::new(
                                WidgetBuilder::new()
                                    .on_row(0)
                                    .on_column(0)
                                    .with_vertical_alignment(VerticalAlignment::Center),
                            )
                            .with_text("File Name:")
                            .build(ctx),
                        )
                        .with_child({
                            file_name = TextBoxBuilder::new(
                                WidgetBuilder::new()
                                    .on_row(0)
                                    .on_column(1)
                                    .with_margin(Thickness::uniform(2.0)),
                            )
                            .with_text_commit_mode(TextCommitMode::Immediate)
                            .with_vertical_text_alignment(VerticalAlignment::Center)
                            .with_text(default_file_name.to_string_lossy())
                            .build(ctx);
                            file_name
                        }),
                )
                .add_row(Row::stretch())
                .add_column(Column::strict(80.0))
                .add_column(Column::stretch())
                .build(ctx);

                ctx.link(name_grid, grid);

                file_name
            }
            FileBrowserMode::Open => Default::default(),
        };

        let widget = self.widget_builder.with_child(grid).build();
        let the_path = match &self.root {
            Some(path) => path.clone(),
            _ => self.path.clone(),
        };
        let browser = FileBrowser {
            widget,
            tree_root,
            path_text,
            path: match self.mode {
                FileBrowserMode::Open => self.path,
                FileBrowserMode::Save {
                    ref default_file_name,
                } => self.path.join(default_file_name),
            },
            file_name_value: match self.mode {
                FileBrowserMode::Open => Default::default(),
                FileBrowserMode::Save {
                    ref default_file_name,
                } => default_file_name.clone(),
            },
            filter: self.filter,
            mode: self.mode,
            scroll_viewer,
            root: self.root,
            file_name,
            watcher: Rc::new(cell::Cell::new(None)),
        };
        let watcher = browser.watcher.clone();
        let filebrowser_node = UINode::FileBrowser(browser);
        let node = ctx.add_node(filebrowser_node);
        watcher.replace(setup_filebrowser_fs_watcher(
            ctx.ui.sender(),
            node,
            the_path,
        ));
        node
    }
}

fn setup_filebrowser_fs_watcher<M: MessageData, C: Control<M, C>>(
    ui_sender: mpsc::Sender<UiMessage<M, C>>,
    filebrowser_widget_handle: Handle<UINode<M, C>>,
    the_path: PathBuf,
) -> Option<(notify::RecommendedWatcher, thread::JoinHandle<()>)> {
    let (tx, rx) = mpsc::channel();
    match notify::watcher(tx, time::Duration::from_secs(1)) {
        Ok(mut watcher) => {
            #[allow(clippy::while_let_loop)]
            let watcher_conversion_thread = std::thread::spawn(move || loop {
                println!("Waiting for FS Watcher Event....");
                match rx.recv() {
                    Ok(event) => match event {
                        notify::DebouncedEvent::NoticeRemove(path)
                        | notify::DebouncedEvent::Remove(path) => {
                            println!("Sent Remove Message");
                            match ui_sender.send(FileBrowserMessage::remove(
                                filebrowser_widget_handle,
                                MessageDirection::ToWidget,
                                path,
                            )) {
                                Ok(_) => println!("Successfully sent Remove message"),
                                Err(_) => println!("Failed to Send Remove Message"),
                            }
                        }
                        notify::DebouncedEvent::Create(path) => {
                            println!("Sent Create Message");
                            let _ = ui_sender.send(FileBrowserMessage::add(
                                filebrowser_widget_handle,
                                MessageDirection::ToWidget,
                                path,
                            ));
                        }
                        notify::DebouncedEvent::Rescan | notify::DebouncedEvent::Error(_, _) => {
                            println!("Sent Rescan Message");
                            let _ = ui_sender.send(FileBrowserMessage::rescan(
                                filebrowser_widget_handle,
                                MessageDirection::ToWidget,
                            ));
                        }
                        _ => {
                            println!("Ignored FS Watcher Event");
                            ()
                        }
                    },
                    Err(_) => {
                        println!("Breaking out of FS Watcher Event Loop");
                        break;
                    }
                };
            });
            println!(
                "Starting FS Event watch on Path: {}",
                the_path.to_str().unwrap()
            );
            let _ = watcher.watch(the_path, notify::RecursiveMode::Recursive);
            Some((watcher, watcher_conversion_thread))
        }
        Err(_) => None,
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
                        browser.path.clone()
                    } else {
                        unreachable!();
                    };
                    ui.send_message(FileSelectorMessage::commit(
                        self.handle,
                        MessageDirection::ToWidget,
                        path,
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
                        FileSelectorMessage::Filter(filter) => {
                            ui.send_message(FileBrowserMessage::filter(
                                self.browser,
                                MessageDirection::ToWidget,
                                filter.clone(),
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
    filter: Option<Filter>,
    mode: FileBrowserMode,
    path: PathBuf,
    root: Option<PathBuf>,
}

impl<M: MessageData, C: Control<M, C>> FileSelectorBuilder<M, C> {
    pub fn new(window_builder: WindowBuilder<M, C>) -> Self {
        Self {
            window_builder,
            filter: None,
            mode: FileBrowserMode::Open,
            path: Default::default(),
            root: None,
        }
    }

    pub fn with_filter(mut self, filter: Filter) -> Self {
        self.filter = Some(filter);
        self
    }

    pub fn with_path<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.path = path.as_ref().to_owned();
        self
    }

    pub fn with_mode(mut self, mode: FileBrowserMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn with_root(mut self, root: PathBuf) -> Self {
        self.root = Some(root);
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
                                        .with_text(match &self.mode {
                                            FileBrowserMode::Open => "Open",
                                            FileBrowserMode::Save { .. } => "Save",
                                        })
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
                        )
                        .with_child({
                            browser = FileBrowserBuilder::new(WidgetBuilder::new().on_column(0))
                                .with_mode(self.mode)
                                .with_opt_filter(self.filter)
                                .with_path(self.path)
                                .with_opt_root(self.root)
                                .build(ctx);
                            browser
                        }),
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
