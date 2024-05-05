//! File browser is a tree view over file system. It allows to select file or folder.
//!
//! File selector is dialog window with file browser, it somewhat similar to standard
//! OS file selector.

use crate::{
    core::pool::Handle,
    core::{reflect::prelude::*, type_traits::prelude::*, visitor::prelude::*},
    define_constructor,
    file_browser::menu::ItemContextMenu,
    grid::{Column, GridBuilder, Row},
    message::{MessageDirection, UiMessage},
    scroll_viewer::{ScrollViewerBuilder, ScrollViewerMessage},
    text::{TextBuilder, TextMessage},
    text_box::{TextBoxBuilder, TextCommitMode},
    tree::{Tree, TreeBuilder, TreeMessage, TreeRoot, TreeRootBuilder, TreeRootMessage},
    widget::{Widget, WidgetBuilder, WidgetMessage},
    BuildContext, Control, RcUiNodeHandle, Thickness, UiNode, UserInterface, VerticalAlignment,
};
use core::time;
use std::{
    borrow::BorrowMut,
    cmp::Ordering,
    fmt::{Debug, Formatter},
    fs::DirEntry,
    ops::{Deref, DerefMut},
    path::{Component, Path, PathBuf, Prefix},
    sync::{
        mpsc::{self, Receiver},
        Arc,
    },
    thread,
};

mod menu;
mod selector;

use fyrox_core::parking_lot::Mutex;
use fyrox_core::uuid_provider;
use fyrox_graph::BaseSceneGraph;
use notify::Watcher;
pub use selector::*;
#[cfg(not(target_arch = "wasm32"))]
use sysinfo::{DiskExt, RefreshKind, SystemExt};

#[derive(Debug, Clone, PartialEq)]
pub enum FileBrowserMessage {
    Root(Option<PathBuf>),
    Path(PathBuf),
    Filter(Option<Filter>),
    Add(PathBuf),
    Remove(PathBuf),
    Rescan,
    Drop {
        dropped: Handle<UiNode>,
        path_item: Handle<UiNode>,
        path: PathBuf,
        /// Could be empty if a dropped widget is not a file browser item.
        dropped_path: PathBuf,
    },
}

impl FileBrowserMessage {
    define_constructor!(FileBrowserMessage:Root => fn root(Option<PathBuf>), layout: false);
    define_constructor!(FileBrowserMessage:Path => fn path(PathBuf), layout: false);
    define_constructor!(FileBrowserMessage:Filter => fn filter(Option<Filter>), layout: false);
    define_constructor!(FileBrowserMessage:Add => fn add(PathBuf), layout: false);
    define_constructor!(FileBrowserMessage:Remove => fn remove(PathBuf), layout: false);
    define_constructor!(FileBrowserMessage:Rescan => fn rescan(), layout: false);
    define_constructor!(FileBrowserMessage:Drop => fn drop(
        dropped: Handle<UiNode>,
        path_item: Handle<UiNode>,
        path: PathBuf,
        dropped_path: PathBuf),
        layout: false
    );
}

#[derive(Clone)]
#[allow(clippy::type_complexity)]
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

#[derive(Default, Clone, PartialEq, Eq, Hash, Debug, Visit, Reflect)]
pub enum FileBrowserMode {
    #[default]
    Open,
    Save {
        default_file_name: PathBuf,
    },
}

#[derive(Default, Visit, Reflect, ComponentProvider)]
pub struct FileBrowser {
    pub widget: Widget,
    pub tree_root: Handle<UiNode>,
    pub path_text: Handle<UiNode>,
    pub scroll_viewer: Handle<UiNode>,
    pub path: PathBuf,
    pub root: Option<PathBuf>,
    #[visit(skip)]
    #[reflect(hidden)]
    pub filter: Option<Filter>,
    pub mode: FileBrowserMode,
    pub file_name: Handle<UiNode>,
    pub file_name_value: PathBuf,
    #[visit(skip)]
    #[reflect(hidden)]
    pub fs_receiver: Option<Receiver<notify::Event>>,
    #[visit(skip)]
    #[reflect(hidden)]
    pub item_context_menu: RcUiNodeHandle,
    #[allow(clippy::type_complexity)]
    #[visit(skip)]
    #[reflect(hidden)]
    pub watcher: Option<(notify::RecommendedWatcher, thread::JoinHandle<()>)>,
}

impl Clone for FileBrowser {
    fn clone(&self) -> Self {
        Self {
            widget: self.widget.clone(),
            tree_root: self.tree_root,
            path_text: self.path_text,
            scroll_viewer: self.scroll_viewer,
            path: self.path.clone(),
            root: self.root.clone(),
            filter: self.filter.clone(),
            mode: self.mode.clone(),
            file_name: self.file_name,
            file_name_value: self.file_name_value.clone(),
            fs_receiver: None,
            item_context_menu: self.item_context_menu.clone(),
            watcher: None,
        }
    }
}

impl Debug for FileBrowser {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "FileBrowser")
    }
}

crate::define_widget_deref!(FileBrowser);

impl FileBrowser {
    fn rebuild_from_root(&mut self, ui: &mut UserInterface) {
        // Generate new tree contents.
        let result = build_all(
            self.root.as_ref(),
            &self.path,
            self.filter.clone(),
            self.item_context_menu.clone(),
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
            ui.send_message(TextMessage::text(
                self.path_text,
                MessageDirection::ToWidget,
                String::new(),
            ));
        }
    }
}

uuid_provider!(FileBrowser = "b7f4610e-4b0c-4671-9b4a-60bb45268928");

impl Control for FileBrowser {
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
                                    self.item_context_menu.clone(),
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

                            self.path.clone_from(path);

                            // Set value of text field.
                            ui.send_message(TextMessage::text(
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
                            let watcher_replacement = match self.watcher.take() {
                                Some((mut watcher, converter)) => {
                                    let current_root = match &self.root {
                                        Some(path) => path.clone(),
                                        None => self.path.clone(),
                                    };
                                    if current_root.exists() {
                                        let _ = watcher.unwatch(&current_root);
                                    }
                                    let new_root = match &root {
                                        Some(path) => path.clone(),
                                        None => self.path.clone(),
                                    };
                                    let _ =
                                        watcher.watch(&new_root, notify::RecursiveMode::Recursive);
                                    Some((watcher, converter))
                                }
                                None => None,
                            };
                            self.root.clone_from(root);
                            self.path = root.clone().unwrap_or_default();
                            self.rebuild_from_root(ui);
                            self.watcher = watcher_replacement;
                        }
                    }
                    FileBrowserMessage::Filter(filter) => {
                        let equal = match (&self.filter, filter) {
                            (Some(current), Some(new)) => std::ptr::eq(new, current),
                            _ => false,
                        };
                        if !equal {
                            self.filter.clone_from(filter);
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
                                if tree.is_expanded {
                                    build_tree(
                                        existing_parent_node,
                                        existing_parent_node == self.tree_root,
                                        path,
                                        parent_path,
                                        self.item_context_menu.clone(),
                                        ui,
                                    );
                                } else if !tree.always_show_expander {
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
                    FileBrowserMessage::Rescan | FileBrowserMessage::Drop { .. } => (),
                }
            }
        } else if let Some(TextMessage::Text(txt)) = message.data::<TextMessage>() {
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
                    .user_data_cloned::<PathBuf>()
                    .unwrap()
                    .clone();
                if let Ok(dir_iter) = std::fs::read_dir(&parent_path) {
                    let mut entries: Vec<_> = dir_iter.flatten().collect();
                    entries.sort_unstable_by(sort_dir_entries);
                    for entry in entries {
                        let path = entry.path();
                        let build = if let Some(filter) = self.filter.as_mut() {
                            filter.0.borrow_mut().deref_mut().lock()(&path)
                        } else {
                            true
                        };
                        if build {
                            build_tree(
                                message.destination(),
                                false,
                                &path,
                                &parent_path,
                                self.item_context_menu.clone(),
                                ui,
                            );
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
                    true,
                ));
            }
        } else if let Some(WidgetMessage::Drop(dropped)) = message.data() {
            if !message.handled() {
                if let Some(path) = ui.node(message.destination()).user_data_cloned::<PathBuf>() {
                    ui.send_message(FileBrowserMessage::drop(
                        self.handle,
                        MessageDirection::FromWidget,
                        *dropped,
                        message.destination(),
                        path.clone(),
                        ui.node(*dropped)
                            .user_data_cloned::<PathBuf>()
                            .unwrap_or_default(),
                    ));

                    message.set_handled(true);
                }
            }
        } else if let Some(TreeRootMessage::Selected(selection)) = message.data::<TreeRootMessage>()
        {
            if message.destination() == self.tree_root
                && message.direction() == MessageDirection::FromWidget
            {
                if let Some(&first_selected) = selection.first() {
                    let mut path = ui
                        .node(first_selected)
                        .user_data_cloned::<PathBuf>()
                        .unwrap()
                        .clone();

                    if let FileBrowserMode::Save { .. } = self.mode {
                        if path.is_file() {
                            ui.send_message(TextMessage::text(
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
                        self.path.clone_from(&path);

                        ui.send_message(TextMessage::text(
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

    fn update(&mut self, _dt: f32, ui: &mut UserInterface) {
        if let Ok(event) = self.fs_receiver.as_ref().unwrap().try_recv() {
            if event.need_rescan() {
                ui.send_message(FileBrowserMessage::rescan(
                    self.handle,
                    MessageDirection::ToWidget,
                ));
            } else {
                for path in event.paths.iter() {
                    match event.kind {
                        notify::EventKind::Remove(_) => {
                            ui.send_message(FileBrowserMessage::remove(
                                self.handle,
                                MessageDirection::ToWidget,
                                path.clone(),
                            ));
                        }
                        notify::EventKind::Create(_) => {
                            ui.send_message(FileBrowserMessage::add(
                                self.handle,
                                MessageDirection::ToWidget,
                                path.clone(),
                            ));
                        }
                        _ => (),
                    }
                }
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
        Some(filter) => !filter.0.borrow_mut().deref_mut().lock()(path),
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
        let tree_path = tree.user_data_cloned::<PathBuf>().unwrap();
        if tree_path == path.as_ref() {
            tree_handle = node;
        } else {
            for &item in &tree.items {
                let tree = find_tree(item, path, ui);
                if tree.is_some() {
                    tree_handle = tree;
                    break;
                }
            }
        }
    } else if let Some(root) = node_ref.cast::<TreeRoot>() {
        for &item in &root.items {
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
    menu: RcUiNodeHandle,
    expanded: bool,
    ctx: &mut BuildContext,
) -> Handle<UiNode> {
    let is_dir_empty = path
        .as_ref()
        .read_dir()
        .map_or(true, |mut f| f.next().is_none());
    TreeBuilder::new(
        WidgetBuilder::new()
            .with_user_data(Arc::new(Mutex::new(path.as_ref().to_owned())))
            .with_context_menu(menu),
    )
    .with_expanded(expanded)
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
    menu: RcUiNodeHandle,
    ui: &mut UserInterface,
) -> Handle<UiNode> {
    let subtree = build_tree_item(path, parent_path, menu, false, &mut ui.build_ctx());
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
    menu: RcUiNodeHandle,
    ctx: &mut BuildContext,
) -> BuildResult {
    let mut dest_path = PathBuf::new();
    if let Ok(canonical_final_path) = final_path.canonicalize() {
        if let Some(canonical_root) = root.and_then(|r| r.canonicalize().ok()) {
            if let Ok(stripped) = canonical_final_path.strip_prefix(canonical_root) {
                stripped.clone_into(&mut dest_path);
            }
        } else {
            dest_path = canonical_final_path;
        }
    }

    // There should be at least one component in the path. If the path is empty, this means that
    // it "points" to the current directory.
    if dest_path == PathBuf::default() {
        dest_path.push(".");
    }

    let dest_path_components = dest_path.components().collect::<Vec<Component>>();
    #[allow(unused_variables)]
    let dest_disk = dest_path_components.first().and_then(|c| {
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
        let item = build_tree_item(path, Path::new(""), menu.clone(), true, ctx);
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
                let disk_letter = disk.chars().next().unwrap() as u8;
                let is_disk_part_of_path = dest_disk.map_or(false, |d| d == disk_letter);

                let item =
                    build_tree_item(disk.as_ref(), "", menu.clone(), is_disk_part_of_path, ctx);

                if is_disk_part_of_path {
                    parent = item;
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

        let next_component = dest_path_components.get(i + 1);

        if let Some(next_component) = next_component {
            if matches!(component, Component::Prefix(_))
                && matches!(next_component, Component::RootDir)
            {
                continue;
            }
        }

        let next = next_component.map(|p| full_path.join(p));

        let mut new_parent = parent;
        if let Ok(dir_iter) = std::fs::read_dir(&full_path) {
            let mut entries: Vec<_> = dir_iter.flatten().collect();
            entries.sort_unstable_by(sort_dir_entries);
            for entry in entries {
                let path = entry.path();
                #[allow(clippy::blocks_in_conditions)]
                if filter
                    .as_mut()
                    .map_or(true, |f| f.0.borrow_mut().deref_mut().lock()(&path))
                {
                    let is_part_of_final_path = next.as_ref().map_or(false, |next| *next == path);

                    let item = build_tree_item(
                        &path,
                        &full_path,
                        menu.clone(),
                        is_part_of_final_path,
                        ctx,
                    );

                    if parent.is_some() {
                        Tree::add_item(parent, item, ctx);
                    } else {
                        root_items.push(item);
                    }

                    if is_part_of_final_path {
                        new_parent = item;
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
    show_path: bool,
}

impl FileBrowserBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            path: Default::default(),
            filter: None,
            root: None,
            mode: FileBrowserMode::Open,
            show_path: true,
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

    pub fn with_show_path(mut self, show_path: bool) -> Self {
        self.show_path = show_path;
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
        path.as_ref().clone_into(&mut self.path);
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
        let item_context_menu = RcUiNodeHandle::new(ItemContextMenu::build(ctx), ctx.sender());

        let BuildResult {
            root_items: items, ..
        } = build_all(
            self.root.as_ref(),
            self.path.as_path(),
            self.filter.clone(),
            item_context_menu.clone(),
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
                            .with_visibility(self.show_path)
                            .with_height(24.0)
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
                vec![Row::auto(), Row::stretch()]
            }
            FileBrowserMode::Save { .. } => {
                vec![Row::auto(), Row::strict(24.0), Row::stretch()]
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

        let widget = self
            .widget_builder
            .with_need_update(true)
            .with_child(grid)
            .build();

        let the_path = match &self.root {
            Some(path) => path.clone(),
            _ => self.path.clone(),
        };
        let (fs_sender, fs_receiver) = mpsc::channel();
        let browser = FileBrowser {
            fs_receiver: Some(fs_receiver),
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
            watcher: setup_filebrowser_fs_watcher(fs_sender, the_path),
            item_context_menu,
        };
        ctx.add_node(UiNode::new(browser))
    }
}

fn setup_filebrowser_fs_watcher(
    fs_sender: mpsc::Sender<notify::Event>,
    the_path: PathBuf,
) -> Option<(notify::RecommendedWatcher, thread::JoinHandle<()>)> {
    let (tx, rx) = mpsc::channel();
    match notify::RecommendedWatcher::new(
        tx,
        notify::Config::default().with_poll_interval(time::Duration::from_secs(1)),
    ) {
        Ok(mut watcher) => {
            #[allow(clippy::while_let_loop)]
            let watcher_conversion_thread = std::thread::spawn(move || loop {
                match rx.recv() {
                    Ok(event) => {
                        if let Ok(event) = event {
                            let _ = fs_sender.send(event);
                        }
                    }
                    Err(_) => {
                        break;
                    }
                };
            });
            let _ = watcher.watch(&the_path, notify::RecursiveMode::Recursive);
            Some((watcher, watcher_conversion_thread))
        }
        Err(_) => None,
    }
}

#[cfg(test)]
mod test {
    use crate::{
        core::pool::Handle,
        file_browser::{build_tree, find_tree},
        tree::TreeRootBuilder,
        widget::WidgetBuilder,
        RcUiNodeHandle, UserInterface,
    };
    use fyrox_core::algebra::Vector2;
    use fyrox_core::parking_lot::Mutex;
    use std::path::PathBuf;
    use std::sync::Arc;

    #[test]
    fn test_find_tree() {
        let mut ui = UserInterface::new(Vector2::new(100.0, 100.0));

        let root = TreeRootBuilder::new(
            WidgetBuilder::new().with_user_data(Arc::new(Mutex::new(PathBuf::from("test")))),
        )
        .build(&mut ui.build_ctx());

        let path = build_tree(
            root,
            true,
            "./test/path1",
            "./test",
            RcUiNodeHandle::new(Handle::new(0, 1), ui.sender()),
            &mut ui,
        );

        while ui.poll_message().is_some() {}

        // This passes.
        assert_eq!(find_tree(root, &"./test/path1", &ui), path);

        // This expected to fail
        // https://github.com/rust-lang/rust/issues/31374
        assert_eq!(find_tree(root, &"test/path1", &ui), Handle::NONE);
    }
}
