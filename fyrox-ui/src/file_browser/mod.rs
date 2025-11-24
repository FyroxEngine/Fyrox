// Copyright (c) 2019-present Dmitry Stepanov and Fyrox Engine contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

//! File browser is a tree view over file system. It allows to select file or folder.
//!
//! File selector is dialog window with file browser, it somewhat similar to standard
//! OS file selector.

use crate::{
    button::{ButtonBuilder, ButtonMessage},
    core::{
        parking_lot::Mutex, pool::Handle, reflect::prelude::*, type_traits::prelude::*,
        uuid_provider, visitor::prelude::*, SafeLock,
    },
    file_browser::menu::ItemContextMenu,
    grid::{Column, GridBuilder, Row},
    message::{MessageData, MessageDirection, UiMessage},
    scroll_viewer::{ScrollViewerBuilder, ScrollViewerMessage},
    text::{TextBuilder, TextMessage},
    text_box::{TextBoxBuilder, TextCommitMode},
    tree::{Tree, TreeMessage, TreeRootBuilder, TreeRootMessage},
    utils::make_simple_tooltip,
    widget::{Widget, WidgetBuilder, WidgetMessage},
    BuildContext, Control, RcUiNodeHandle, Thickness, UiNode, UserInterface, VerticalAlignment,
};
use core::time;
use fyrox_graph::{
    constructor::{ConstructorProvider, GraphNodeConstructor},
    BaseSceneGraph,
};
use notify::Watcher;
use std::{
    borrow::BorrowMut,
    fmt::{Debug, Formatter},
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
    sync::{
        mpsc::{self, Receiver},
        Arc,
    },
    thread,
};

mod fs_tree;
mod menu;
mod selector;

pub use selector::*;

#[derive(Debug, Clone, PartialEq)]
pub enum FileBrowserMessage {
    Root(Option<PathBuf>),
    Path(PathBuf),
    Filter(Option<Filter>),
    Add(PathBuf),
    Remove(PathBuf),
    FocusCurrentPath,
    Rescan,
    Drop {
        dropped: Handle<UiNode>,
        path_item: Handle<UiNode>,
        path: PathBuf,
        /// Could be empty if a dropped widget is not a file browser item.
        dropped_path: PathBuf,
    },
}
impl MessageData for FileBrowserMessage {}

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
#[reflect(derived_type = "UiNode")]
pub struct FileBrowser {
    pub widget: Widget,
    pub tree_root: Handle<UiNode>,
    pub home_dir: Handle<UiNode>,
    pub desktop_dir: Handle<UiNode>,
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
    pub root_title: Option<String>,
}

impl ConstructorProvider<UiNode, UserInterface> for FileBrowser {
    fn constructor() -> GraphNodeConstructor<UiNode, UserInterface> {
        GraphNodeConstructor::new::<Self>()
            .with_variant("File Browser", |ui| {
                FileBrowserBuilder::new(WidgetBuilder::new().with_name("File Browser"))
                    .build(&mut ui.build_ctx())
                    .into()
            })
            .with_group("File System")
    }
}

impl Clone for FileBrowser {
    fn clone(&self) -> Self {
        Self {
            widget: self.widget.clone(),
            tree_root: self.tree_root,
            home_dir: self.home_dir,
            desktop_dir: self.desktop_dir,
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
            root_title: self.root_title.clone(),
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
        let fs_tree = fs_tree::FsTree::new(
            self.root.as_ref(),
            &self.path,
            self.filter.as_ref(),
            self.item_context_menu.clone(),
            self.root_title.as_deref(),
            &mut ui.build_ctx(),
        );

        // Replace tree contents.
        ui.send(self.tree_root, TreeRootMessage::Items(fs_tree.root_items));

        if fs_tree.path_item.is_some() {
            // Select item of new path.
            ui.send(
                self.tree_root,
                TreeRootMessage::Select(vec![fs_tree.path_item]),
            );
            // Bring item of new path into view.
            ui.send(
                self.scroll_viewer,
                ScrollViewerMessage::BringIntoView(fs_tree.path_item),
            );
        } else {
            // Clear text field if path is invalid.
            ui.send(self.path_text, TextMessage::Text(String::new()));
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

                            let mut item = fs_tree::find_tree(self.tree_root, &existing_path, ui);

                            if item.is_none() {
                                // Generate new tree contents.
                                let fs_tree = fs_tree::FsTree::new(
                                    self.root.as_ref(),
                                    &existing_path,
                                    self.filter.as_ref(),
                                    self.item_context_menu.clone(),
                                    self.root_title.as_deref(),
                                    &mut ui.build_ctx(),
                                );

                                // Replace tree contents.
                                ui.send(self.tree_root, TreeRootMessage::Items(fs_tree.root_items));

                                item = fs_tree.path_item;
                            }

                            self.path.clone_from(path);

                            // Set value of text field.
                            ui.send(
                                self.path_text,
                                TextMessage::Text(path.to_string_lossy().to_string()),
                            );

                            // Path can be invalid, so we shouldn't do anything in such case.
                            if item.is_some() {
                                // Select item of new path.
                                ui.send(self.tree_root, TreeRootMessage::Select(vec![item]));

                                // Bring item of new path into view.
                                ui.send(
                                    self.scroll_viewer,
                                    ScrollViewerMessage::BringIntoView(item),
                                );
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
                        let existing_parent_node =
                            fs_tree::find_tree(self.tree_root, &parent_path, ui);
                        if existing_parent_node.is_some() {
                            if let Some(tree) = ui.node(existing_parent_node).cast::<Tree>() {
                                if tree.is_expanded {
                                    fs_tree::build_tree(
                                        existing_parent_node,
                                        existing_parent_node == self.tree_root,
                                        path,
                                        parent_path,
                                        self.item_context_menu.clone(),
                                        self.root_title.as_deref(),
                                        ui,
                                    );
                                } else if !tree.always_show_expander {
                                    ui.send(tree.handle(), TreeMessage::SetExpanderShown(true))
                                }
                            }
                        }
                    }
                    FileBrowserMessage::Remove(path) => {
                        let path =
                            make_fs_watcher_event_path_relative_to_tree_root(&self.root, path);
                        let node = fs_tree::find_tree(self.tree_root, &path, ui);
                        if node.is_some() {
                            let parent_path = parent_path(&path);
                            let parent_node = fs_tree::find_tree(self.tree_root, &parent_path, ui);
                            ui.send(parent_node, TreeMessage::RemoveItem(node))
                        }
                    }
                    FileBrowserMessage::Rescan | FileBrowserMessage::Drop { .. } => (),
                    FileBrowserMessage::FocusCurrentPath => {
                        if let Ok(canonical_path) = self.path.canonicalize() {
                            let item = fs_tree::find_tree(self.tree_root, &canonical_path, ui);
                            if item.is_some() {
                                // Select item of new path.
                                ui.send(self.tree_root, TreeRootMessage::Select(vec![item]));
                                ui.send(
                                    self.scroll_viewer,
                                    ScrollViewerMessage::BringIntoView(item),
                                );
                            }
                        }
                    }
                }
            }
        } else if let Some(TextMessage::Text(txt)) = message.data::<TextMessage>() {
            if message.direction() == MessageDirection::FromWidget {
                if message.destination() == self.path_text {
                    self.path = txt.into();
                } else if message.destination() == self.file_name {
                    self.file_name_value = txt.into();
                    ui.send(
                        self.handle,
                        FileBrowserMessage::Path({
                            let mut combined = self.path.clone();
                            combined.set_file_name(PathBuf::from(txt));
                            combined
                        }),
                    );
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
                fs_tree::build_single_folder(
                    &parent_path,
                    message.destination(),
                    self.item_context_menu.clone(),
                    self.root_title.as_deref(),
                    self.filter.as_ref(),
                    ui,
                )
            } else {
                // Nuke everything in collapsed item. This also will free some resources
                // and will speed up layout pass.
                ui.send(
                    message.destination(),
                    TreeMessage::SetItems {
                        items: vec![],
                        remove_previous: true,
                    },
                );
            }
        } else if let Some(WidgetMessage::Drop(dropped)) = message.data() {
            if !message.handled() {
                if let Some(path) = ui.node(message.destination()).user_data_cloned::<PathBuf>() {
                    ui.post(
                        self.handle,
                        FileBrowserMessage::Drop {
                            dropped: *dropped,
                            path_item: message.destination(),
                            path: path.clone(),
                            dropped_path: ui
                                .node(*dropped)
                                .user_data_cloned::<PathBuf>()
                                .unwrap_or_default(),
                        },
                    );
                    message.set_handled(true);
                }
            }
        } else if let Some(TreeRootMessage::Select(selection)) = message.data::<TreeRootMessage>() {
            if message.destination() == self.tree_root
                && message.direction() == MessageDirection::FromWidget
            {
                if let Some(&first_selected) = selection.first() {
                    if let Some(first_selected_ref) = ui.try_get_node(first_selected) {
                        let mut path = first_selected_ref
                            .user_data_cloned::<PathBuf>()
                            .unwrap()
                            .clone();

                        if let FileBrowserMode::Save { .. } = self.mode {
                            if path.is_file() {
                                ui.send(
                                    self.file_name,
                                    TextMessage::Text(
                                        path.file_name()
                                            .map(|f| f.to_string_lossy().to_string())
                                            .unwrap_or_default(),
                                    ),
                                );
                            } else {
                                path = path.join(&self.file_name_value);
                            }
                        }

                        if self.path != path {
                            self.path.clone_from(&path);

                            ui.send(
                                self.path_text,
                                TextMessage::Text(path.to_string_lossy().to_string()),
                            );

                            // Do response.
                            ui.post(self.handle, FileBrowserMessage::Path(path));
                        }
                    }
                }
            }
        } else if let Some(ButtonMessage::Click) = message.data() {
            if message.direction() == MessageDirection::FromWidget {
                #[cfg(not(target_arch = "wasm32"))]
                if message.destination() == self.desktop_dir {
                    let user_dirs = directories::UserDirs::new();
                    if let Some(desktop_dir) =
                        user_dirs.as_ref().and_then(|dirs| dirs.desktop_dir())
                    {
                        ui.send(
                            self.handle,
                            FileBrowserMessage::Path(desktop_dir.to_path_buf()),
                        );
                    }
                }

                #[cfg(not(target_arch = "wasm32"))]
                if message.destination() == self.home_dir {
                    let user_dirs = directories::UserDirs::new();
                    if let Some(home_dir) = user_dirs.as_ref().map(|dirs| dirs.home_dir()) {
                        ui.send(
                            self.handle,
                            FileBrowserMessage::Path(home_dir.to_path_buf()),
                        );
                    }
                }
            }
        }
    }

    fn update(&mut self, _dt: f32, ui: &mut UserInterface) {
        if let Ok(event) = self.fs_receiver.as_ref().unwrap().try_recv() {
            if event.need_rescan() {
                ui.send(self.handle, FileBrowserMessage::Rescan);
            } else {
                for path in event.paths.iter() {
                    match event.kind {
                        notify::EventKind::Remove(_) => {
                            ui.send(self.handle, FileBrowserMessage::Remove(path.clone()));
                        }
                        notify::EventKind::Create(_) => {
                            ui.send(self.handle, FileBrowserMessage::Add(path.clone()));
                        }
                        _ => (),
                    }
                }
            }
        }
    }

    fn accepts_drop(&self, widget: Handle<UiNode>, ui: &UserInterface) -> bool {
        ui.node(widget)
            .user_data
            .as_ref()
            .is_some_and(|data| data.safe_lock().downcast_ref::<PathBuf>().is_some())
    }
}

fn parent_path(path: &Path) -> PathBuf {
    let mut parent_path = path.to_owned();
    parent_path.pop();
    parent_path
}

fn filtered_out(filter: &mut Option<Filter>, path: &Path) -> bool {
    match filter.as_mut() {
        Some(filter) => !filter.0.borrow_mut().deref_mut().safe_lock()(path),
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

fn make_fs_watcher_event_path_relative_to_tree_root(
    root: &Option<PathBuf>,
    path: &Path,
) -> PathBuf {
    match root {
        Some(ref root) => {
            let remove_prefix = if root == Path::new(".") {
                std::env::current_dir().unwrap()
            } else {
                root.clone()
            };
            PathBuf::from("./").join(path.strip_prefix(remove_prefix).unwrap_or(path))
        }
        None => path.to_owned(),
    }
}

pub struct FileBrowserBuilder {
    widget_builder: WidgetBuilder,
    path: PathBuf,
    filter: Option<Filter>,
    root: Option<PathBuf>,
    mode: FileBrowserMode,
    show_path: bool,
    root_title: Option<String>,
}

impl FileBrowserBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            path: "./".into(),
            filter: None,
            root: None,
            mode: FileBrowserMode::Open,
            show_path: true,
            root_title: None,
        }
    }

    pub fn with_root_title(mut self, root_title: Option<String>) -> Self {
        self.root_title = root_title;
        self
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

        let fs_tree::FsTree {
            root_items: items, ..
        } = fs_tree::FsTree::new(
            self.root.as_ref(),
            self.path.as_path(),
            self.filter.as_ref(),
            item_context_menu.clone(),
            self.root_title.as_deref(),
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

        let home_dir;
        let desktop_dir;
        let grid = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(
                    GridBuilder::new(
                        WidgetBuilder::new()
                            .with_visibility(self.show_path)
                            .with_height(24.0)
                            .with_child({
                                home_dir = ButtonBuilder::new(
                                    WidgetBuilder::new()
                                        .on_column(0)
                                        .with_width(24.0)
                                        .with_tooltip(make_simple_tooltip(ctx, "Home Folder"))
                                        .with_margin(Thickness::uniform(1.0)),
                                )
                                .with_text("H")
                                .build(ctx);
                                home_dir
                            })
                            .with_child({
                                desktop_dir = ButtonBuilder::new(
                                    WidgetBuilder::new()
                                        .on_column(1)
                                        .with_width(24.0)
                                        .with_tooltip(make_simple_tooltip(ctx, "Desktop Folder"))
                                        .with_margin(Thickness::uniform(1.0)),
                                )
                                .with_text("D")
                                .build(ctx);
                                desktop_dir
                            })
                            .with_child({
                                path_text = TextBoxBuilder::new(
                                    WidgetBuilder::new()
                                        // Disable path if we're in Save mode
                                        .with_enabled(matches!(self.mode, FileBrowserMode::Open))
                                        .on_row(0)
                                        .on_column(2)
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
                    .add_column(Column::auto())
                    .add_column(Column::auto())
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
            .build(ctx);

        let the_path = match &self.root {
            Some(path) => path.clone(),
            _ => self.path.clone(),
        };
        let (fs_sender, fs_receiver) = mpsc::channel();
        let browser = FileBrowser {
            fs_receiver: Some(fs_receiver),
            widget,
            tree_root,
            home_dir,
            desktop_dir,
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
            root_title: self.root_title,
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
    use crate::file_browser::{fs_tree, FileBrowserBuilder};
    use crate::test::test_widget_deletion;
    use crate::{
        core::pool::Handle, tree::TreeRootBuilder, widget::WidgetBuilder, RcUiNodeHandle,
        UserInterface,
    };
    use fyrox_core::algebra::Vector2;
    use fyrox_core::parking_lot::Mutex;
    use std::path::PathBuf;
    use std::sync::Arc;

    #[test]
    fn test_deletion() {
        test_widget_deletion(|ctx| FileBrowserBuilder::new(WidgetBuilder::new()).build(ctx));
    }

    #[test]
    fn test_find_tree() {
        let mut ui = UserInterface::new(Vector2::new(100.0, 100.0));

        let root = TreeRootBuilder::new(
            WidgetBuilder::new().with_user_data(Arc::new(Mutex::new(PathBuf::from("test")))),
        )
        .build(&mut ui.build_ctx());

        let path = fs_tree::build_tree(
            root,
            true,
            "./test/path1",
            "./test",
            RcUiNodeHandle::new(Handle::new(0, 1), ui.sender()),
            None,
            &mut ui,
        );

        while ui.poll_message().is_some() {}

        // This passes.
        assert_eq!(fs_tree::find_tree(root, &"./test/path1", &ui), path);

        // This expected to fail
        // https://github.com/rust-lang/rust/issues/31374
        assert_eq!(fs_tree::find_tree(root, &"test/path1", &ui), Handle::NONE);
    }
}
