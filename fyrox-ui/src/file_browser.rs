//! File browser is a tree view over file system. It allows to select file or folder.
//!
//! File selector is dialog window with file browser, it somewhat similar to standard
//! OS file selector.

use crate::{
    button::{ButtonBuilder, ButtonMessage},
    core::{algebra::Vector2, pool::Handle},
    define_constructor,
    draw::DrawingContext,
    grid::{Column, GridBuilder, Row},
    message::{MessageDirection, OsEvent, UiMessage},
    scroll_viewer::{ScrollViewerBuilder, ScrollViewerMessage},
    stack_panel::StackPanelBuilder,
    text::TextBuilder,
    text_box::{TextBoxBuilder, TextBoxMessage, TextCommitMode},
    tree::{Tree, TreeBuilder, TreeMessage, TreeRoot, TreeRootBuilder, TreeRootMessage},
    widget::{Widget, WidgetBuilder},
    window::{Window, WindowBuilder, WindowMessage, WindowTitle},
    BuildContext, Control, HorizontalAlignment, NodeHandleMapping, Orientation, Thickness, UiNode,
    UserInterface, VerticalAlignment,
};
use core::time;
use std::{
    any::{Any, TypeId},
    borrow::BorrowMut,
    cell,
    cmp::Ordering,
    fmt::{Debug, Formatter},
    fs::DirEntry,
    ops::{Deref, DerefMut},
    path::{Component, Path, PathBuf, Prefix},
    rc::Rc,
    sync::mpsc::{Receiver, Sender},
    sync::{mpsc, Arc, Mutex},
    thread,
};

use notify::Watcher;
#[cfg(not(target_arch = "wasm32"))]
use sysinfo::{DiskExt, RefreshKind, SystemExt};

#[derive(Debug, Clone, PartialEq)]
pub enum FileSelectorMessage {
    Root(Option<PathBuf>),
    Path(PathBuf),
    Commit(PathBuf),
    Cancel,
    Filter(Option<Filter>),
}

impl FileSelectorMessage {
    define_constructor!(FileSelectorMessage:Commit => fn commit(PathBuf), layout: false);
    define_constructor!(FileSelectorMessage:Root => fn root(Option<PathBuf>), layout: false);
    define_constructor!(FileSelectorMessage:Path => fn path(PathBuf), layout: false);
    define_constructor!(FileSelectorMessage:Cancel => fn cancel(), layout: false);
    define_constructor!(FileSelectorMessage:Filter => fn filter(Option<Filter>), layout: false);
}

#[derive(Debug, Clone, PartialEq)]
pub enum FileBrowserMessage {
    Root(Option<PathBuf>),
    Path(PathBuf),
    Filter(Option<Filter>),
    Add(PathBuf),
    Remove(PathBuf),
    Rescan,
}

impl FileBrowserMessage {
    define_constructor!(FileBrowserMessage:Root => fn root(Option<PathBuf>), layout: false);
    define_constructor!(FileBrowserMessage:Path => fn path(PathBuf), layout: false);
    define_constructor!(FileBrowserMessage:Filter => fn filter(Option<Filter>), layout: false);
    define_constructor!(FileBrowserMessage:Add => fn add(PathBuf), layout: false);
    define_constructor!(FileBrowserMessage:Remove => fn remove(PathBuf), layout: false);
    define_constructor!(FileBrowserMessage:Rescan => fn rescan(), layout: false);
}

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
pub struct FileBrowser {
    widget: Widget,
    tree_root: Handle<UiNode>,
    path_text: Handle<UiNode>,
    scroll_viewer: Handle<UiNode>,
    path: PathBuf,
    root: Option<PathBuf>,
    filter: Option<Filter>,
    mode: FileBrowserMode,
    file_name: Handle<UiNode>,
    file_name_value: PathBuf,
    fs_receiver: Rc<Receiver<notify::DebouncedEvent>>,
    #[allow(clippy::type_complexity)]
    watcher: Rc<cell::Cell<Option<(notify::RecommendedWatcher, thread::JoinHandle<()>)>>>,
}

crate::define_widget_deref!(FileBrowser);

impl FileBrowser {
    fn rebuild_from_root(&mut self, ui: &mut UserInterface) {
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

impl Control for FileBrowser {
    fn query_component(&self, type_id: TypeId) -> Option<&dyn Any> {
        if type_id == TypeId::of::<Self>() {
            Some(self)
        } else {
            None
        }
    }

    fn resolve(&mut self, node_map: &NodeHandleMapping) {
        node_map.resolve(&mut self.tree_root);
        node_map.resolve(&mut self.path_text);
        node_map.resolve(&mut self.scroll_viewer);
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(msg) = message.data::<FileBrowserMessage>() {
            if message.destination() == self.handle() {
                match msg {
                    FileBrowserMessage::Path(path) => {
                        if message.direction() == MessageDirection::ToWidget && &self.path != path {
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
                                    if current_root.exists() {
                                        let _ = watcher.unwatch(current_root);
                                    }
                                    let new_root = match &root {
                                        Some(path) => path.clone(),
                                        None => self.path.clone(),
                                    };
                                    let _ =
                                        watcher.watch(new_root, notify::RecursiveMode::Recursive);
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
                        let path =
                            make_fs_watcher_event_path_relative_to_tree_root(&self.root, path);
                        if filtered_out(&mut self.filter, &path) {
                            return;
                        }
                        let parent_path = parent_path(&path);
                        let existing_parent_node = find_tree(self.tree_root, &parent_path, ui);
                        if existing_parent_node.is_some() {
                            if let Some(tree) = ui.node(existing_parent_node).cast::<Tree>() {
                                if tree.expanded() {
                                    build_tree(
                                        existing_parent_node,
                                        existing_parent_node == self.tree_root,
                                        path,
                                        parent_path,
                                        ui,
                                    );
                                } else if !tree.expander_shown() {
                                    ui.send_message(TreeMessage::set_expander_shown(
                                        tree.handle(),
                                        MessageDirection::ToWidget,
                                        true,
                                    ))
                                }
                            }
                        }
                    }
                    FileBrowserMessage::Remove(path) => {
                        let path =
                            make_fs_watcher_event_path_relative_to_tree_root(&self.root, path);
                        let node = find_tree(self.tree_root, &path, ui);
                        if node.is_some() {
                            let parent_path = parent_path(&path);
                            let parent_node = find_tree(self.tree_root, &parent_path, ui);
                            ui.send_message(TreeMessage::remove_item(
                                parent_node,
                                MessageDirection::ToWidget,
                                node,
                            ))
                        }
                    }
                    FileBrowserMessage::Rescan => (),
                }
            }
        } else if let Some(TextBoxMessage::Text(txt)) = message.data::<TextBoxMessage>() {
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
        } else if let Some(TreeMessage::Expand { expand, .. }) = message.data::<TreeMessage>() {
            if *expand {
                // Look into internals of directory and build tree items.
                let parent_path = ui
                    .node(message.destination())
                    .user_data_ref::<PathBuf>()
                    .unwrap()
                    .clone();
                if let Ok(dir_iter) = std::fs::read_dir(&parent_path) {
                    let mut entries: Vec<_> = dir_iter.flatten().collect();
                    entries.sort_unstable_by(sort_dir_entries);
                    for entry in entries {
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
        } else if let Some(TreeRootMessage::Selected(selection)) = message.data::<TreeRootMessage>()
        {
            if message.destination() == self.tree_root
                && message.direction() == MessageDirection::FromWidget
            {
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

    fn update(&mut self, _dt: f32, sender: &Sender<UiMessage>) {
        if let Ok(event) = self.fs_receiver.try_recv() {
            match event {
                notify::DebouncedEvent::Remove(path) => {
                    let _ = sender.send(FileBrowserMessage::remove(
                        self.handle,
                        MessageDirection::ToWidget,
                        path,
                    ));
                }
                notify::DebouncedEvent::Create(path) => {
                    let _ = sender.send(FileBrowserMessage::add(
                        self.handle,
                        MessageDirection::ToWidget,
                        path,
                    ));
                }
                notify::DebouncedEvent::Rescan | notify::DebouncedEvent::Error(_, _) => {
                    let _ = sender.send(FileBrowserMessage::rescan(
                        self.handle,
                        MessageDirection::ToWidget,
                    ));
                }
                notify::DebouncedEvent::NoticeRemove(_) => (),
                notify::DebouncedEvent::NoticeWrite(_) => (),
                notify::DebouncedEvent::Write(_) => (),
                notify::DebouncedEvent::Chmod(_) => (),
                notify::DebouncedEvent::Rename(_, _) => (),
            }
        }
    }
}

fn parent_path(path: &Path) -> PathBuf {
    let mut parent_path = path.to_owned();
    parent_path.pop();
    parent_path
}

fn filtered_out(filter: &mut Option<Filter>, path: &Path) -> bool {
    match filter.as_mut() {
        Some(filter) => !filter.0.borrow_mut().deref_mut().lock().unwrap()(path),
        None => false,
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

fn sort_dir_entries(a: &DirEntry, b: &DirEntry) -> Ordering {
    let a_is_dir = a.path().is_dir();
    let b_is_dir = b.path().is_dir();

    if a_is_dir && !b_is_dir {
        Ordering::Less
    } else if !a_is_dir && b_is_dir {
        Ordering::Greater
    } else {
        a.file_name()
            .to_ascii_lowercase()
            .cmp(&b.file_name().to_ascii_lowercase())
    }
}

fn make_fs_watcher_event_path_relative_to_tree_root(
    root: &Option<PathBuf>,
    path: &Path,
) -> PathBuf {
    match root {
        Some(ref root) => {
            let remove_prefix = if *root == PathBuf::from(".") {
                std::env::current_dir().unwrap()
            } else {
                root.clone()
            };
            PathBuf::from("./").join(path.strip_prefix(remove_prefix).unwrap_or(path))
        }
        None => path.to_owned(),
    }
}

fn find_tree<P: AsRef<Path>>(node: Handle<UiNode>, path: &P, ui: &UserInterface) -> Handle<UiNode> {
    let mut tree_handle = Handle::NONE;
    let node_ref = ui.node(node);

    if let Some(tree) = node_ref.cast::<Tree>() {
        let tree_path = tree.user_data_ref::<PathBuf>().unwrap();
        if tree_path == path.as_ref() {
            tree_handle = node;
        } else {
            for &item in tree.items() {
                let tree = find_tree(item, path, ui);
                if tree.is_some() {
                    tree_handle = tree;
                    break;
                }
            }
        }
    } else if let Some(root) = node_ref.cast::<TreeRoot>() {
        for &item in root.items() {
            let tree = find_tree(item, path, ui);
            if tree.is_some() {
                tree_handle = tree;
                break;
            }
        }
    } else {
        unreachable!()
    }
    tree_handle
}

fn build_tree_item<P: AsRef<Path>>(
    path: P,
    parent_path: P,
    ctx: &mut BuildContext,
) -> Handle<UiNode> {
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
                        .replace('\\', ""),
                )
                .with_vertical_text_alignment(VerticalAlignment::Center)
                .build(ctx),
        )
        .build(ctx)
}

fn build_tree<P: AsRef<Path>>(
    parent: Handle<UiNode>,
    is_parent_root: bool,
    path: P,
    parent_path: P,
    ui: &mut UserInterface,
) -> Handle<UiNode> {
    let subtree = build_tree_item(path, parent_path, &mut ui.build_ctx());
    insert_subtree_in_parent(ui, parent, is_parent_root, subtree);
    subtree
}

fn insert_subtree_in_parent(
    ui: &mut UserInterface,
    parent: Handle<UiNode>,
    is_parent_root: bool,
    tree: Handle<UiNode>,
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

struct BuildResult {
    root_items: Vec<Handle<UiNode>>,
    path_item: Handle<UiNode>,
}

/// Builds entire file system tree to given final_path.
fn build_all(
    root: Option<&PathBuf>,
    final_path: &Path,
    mut filter: Option<Filter>,
    ctx: &mut BuildContext,
) -> BuildResult {
    let mut dest_path = PathBuf::new();
    if let Ok(canonical_final_path) = final_path.canonicalize() {
        if let Some(canonical_root) = root.and_then(|r| r.canonicalize().ok()) {
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
            let mut entries: Vec<_> = dir_iter.flatten().collect();
            entries.sort_unstable_by(sort_dir_entries);
            for entry in entries {
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

pub struct FileBrowserBuilder {
    widget_builder: WidgetBuilder,
    path: PathBuf,
    filter: Option<Filter>,
    root: Option<PathBuf>,
    mode: FileBrowserMode,
}

impl FileBrowserBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
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

    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
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
        let (fs_sender, fs_receiver) = mpsc::channel();
        let browser = FileBrowser {
            fs_receiver: Rc::new(fs_receiver),
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
        let filebrowser_node = UiNode::new(browser);
        let node = ctx.add_node(filebrowser_node);
        watcher.replace(setup_filebrowser_fs_watcher(fs_sender, the_path));
        node
    }
}

fn setup_filebrowser_fs_watcher(
    fs_sender: mpsc::Sender<notify::DebouncedEvent>,
    the_path: PathBuf,
) -> Option<(notify::RecommendedWatcher, thread::JoinHandle<()>)> {
    let (tx, rx) = mpsc::channel();
    match notify::watcher(tx, time::Duration::from_secs(1)) {
        Ok(mut watcher) => {
            #[allow(clippy::while_let_loop)]
            let watcher_conversion_thread = std::thread::spawn(move || loop {
                match rx.recv() {
                    Ok(event) => {
                        let _ = fs_sender.send(event);
                    }
                    Err(_) => {
                        break;
                    }
                };
            });
            let _ = watcher.watch(the_path, notify::RecursiveMode::Recursive);
            Some((watcher, watcher_conversion_thread))
        }
        Err(_) => None,
    }
}

/// File selector is a modal window that allows you to select a file (or directory) and commit or
/// cancel selection.
#[derive(Clone)]
pub struct FileSelector {
    window: Window,
    browser: Handle<UiNode>,
    ok: Handle<UiNode>,
    cancel: Handle<UiNode>,
}

impl Deref for FileSelector {
    type Target = Widget;

    fn deref(&self) -> &Self::Target {
        &self.window
    }
}

impl DerefMut for FileSelector {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.window
    }
}

// File selector extends Window widget so it delegates most of calls
// to inner window.
impl Control for FileSelector {
    fn query_component(&self, type_id: TypeId) -> Option<&dyn Any> {
        self.window.query_component(type_id).or_else(|| {
            if type_id == TypeId::of::<Self>() {
                Some(self)
            } else {
                None
            }
        })
    }

    fn resolve(&mut self, node_map: &NodeHandleMapping) {
        self.window.resolve(node_map);
        node_map.resolve(&mut self.ok);
        node_map.resolve(&mut self.cancel);
    }

    fn measure_override(&self, ui: &UserInterface, available_size: Vector2<f32>) -> Vector2<f32> {
        self.window.measure_override(ui, available_size)
    }

    fn arrange_override(&self, ui: &UserInterface, final_size: Vector2<f32>) -> Vector2<f32> {
        self.window.arrange_override(ui, final_size)
    }

    fn draw(&self, drawing_context: &mut DrawingContext) {
        self.window.draw(drawing_context)
    }

    fn update(&mut self, dt: f32, sender: &Sender<UiMessage>) {
        self.window.update(dt, sender);
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.window.handle_routed_message(ui, message);

        if let Some(ButtonMessage::Click) = message.data::<ButtonMessage>() {
            if message.destination() == self.ok {
                let path = ui
                    .node(self.browser)
                    .cast::<FileBrowser>()
                    .expect("self.browser must be FileBrowser")
                    .path
                    .clone();

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
        } else if let Some(msg) = message.data::<FileSelectorMessage>() {
            if message.destination() == self.handle {
                match msg {
                    FileSelectorMessage::Commit(_) | FileSelectorMessage::Cancel => ui
                        .send_message(WindowMessage::close(
                            self.handle,
                            MessageDirection::ToWidget,
                        )),
                    FileSelectorMessage::Path(path) => ui.send_message(FileBrowserMessage::path(
                        self.browser,
                        MessageDirection::ToWidget,
                        path.clone(),
                    )),
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
    }

    fn preview_message(&self, ui: &UserInterface, message: &mut UiMessage) {
        self.window.preview_message(ui, message);
    }

    fn handle_os_event(
        &mut self,
        self_handle: Handle<UiNode>,
        ui: &mut UserInterface,
        event: &OsEvent,
    ) {
        self.window.handle_os_event(self_handle, ui, event);
    }
}

pub struct FileSelectorBuilder {
    window_builder: WindowBuilder,
    filter: Option<Filter>,
    mode: FileBrowserMode,
    path: PathBuf,
    root: Option<PathBuf>,
}

impl FileSelectorBuilder {
    pub fn new(window_builder: WindowBuilder) -> Self {
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

    pub fn build(mut self, ctx: &mut BuildContext) -> Handle<UiNode> {
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

        ctx.add_node(UiNode::new(file_selector))
    }
}

#[cfg(test)]
mod test {
    use crate::{
        core::pool::Handle,
        file_browser::{build_tree, find_tree},
        tree::TreeRootBuilder,
        widget::WidgetBuilder,
        UserInterface,
    };
    use fyrox_core::algebra::Vector2;
    use std::{path::PathBuf, rc::Rc};

    #[test]
    fn test_find_tree() {
        let mut ui = UserInterface::new(Vector2::new(100.0, 100.0));

        let root = TreeRootBuilder::new(
            WidgetBuilder::new().with_user_data(Rc::new(PathBuf::from("test"))),
        )
        .build(&mut ui.build_ctx());

        let path = build_tree(root, true, "./test/path1", "./test", &mut ui);

        while ui.poll_message().is_some() {}

        // This passes.
        assert_eq!(find_tree(root, &"./test/path1", &ui), path);

        // This expected to fail
        // https://github.com/rust-lang/rust/issues/31374
        assert_eq!(find_tree(root, &"test/path1", &ui), Handle::NONE);
    }
}
