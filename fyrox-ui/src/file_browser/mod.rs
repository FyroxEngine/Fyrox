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
        log::Log, ok_or_return, pool::Handle, reflect::prelude::*, type_traits::prelude::*,
        visitor::prelude::*, SafeLock,
    },
    file_browser::menu::ItemContextMenu,
    grid::{Column, GridBuilder, Row},
    message::{MessageData, UiMessage},
    scroll_viewer::{ScrollViewerBuilder, ScrollViewerMessage},
    text::TextMessage,
    text_box::{TextBoxBuilder, TextCommitMode},
    tree::{Tree, TreeMessage, TreeRootBuilder, TreeRootMessage},
    utils::make_simple_tooltip,
    widget::{Widget, WidgetBuilder, WidgetMessage},
    BuildContext, Control, RcUiNodeHandle, Thickness, UiNode, UserInterface, VerticalAlignment,
};
use core::time;
use fyrox_core::{err, ok_or_continue, some_or_return};
use fyrox_graph::{
    constructor::{ConstructorProvider, GraphNodeConstructor},
    BaseSceneGraph,
};
use notify::{Event, Watcher};
use std::{
    fmt::{Debug, Formatter},
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
    sync::mpsc::Sender,
};

mod field;
mod filter;
mod fs_tree;
mod menu;
mod selector;

#[cfg(test)]
mod test;

pub use field::*;
pub use filter::*;
pub use selector::*;

#[derive(Debug, Clone, PartialEq)]
pub enum FileBrowserMessage {
    Root(Option<PathBuf>),
    Path(PathBuf),
    Filter(PathFilter),
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

#[derive(Debug, Clone, PartialEq)]
enum FsEventMessage {
    Add(PathBuf),
    Remove(PathBuf),
}
impl MessageData for FsEventMessage {}

#[derive(Default, Visit, Reflect, ComponentProvider, TypeUuidProvider)]
#[type_uuid(id = "b7f4610e-4b0c-4671-9b4a-60bb45268928")]
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
    pub filter: PathFilter,
    #[visit(skip)]
    #[reflect(hidden)]
    pub item_context_menu: RcUiNodeHandle,
    #[allow(clippy::type_complexity)]
    #[visit(skip)]
    #[reflect(hidden)]
    pub watcher: Option<notify::RecommendedWatcher>,
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

fn parent_path(path: &Path) -> PathBuf {
    let mut parent_path = path.to_owned();
    parent_path.pop();
    parent_path
}

impl FileBrowser {
    fn select_and_bring_into_view(&self, item: Handle<UiNode>, ui: &UserInterface) {
        ui.send(self.tree_root, TreeRootMessage::Select(vec![item]));
        ui.send(self.scroll_viewer, ScrollViewerMessage::BringIntoView(item));
    }

    fn rebuild_fs_tree(&mut self, ui: &mut UserInterface) {
        let fs_tree = fs_tree::FsTree::new_or_empty(
            self.root.as_ref(),
            &self.path,
            &self.filter,
            self.item_context_menu.clone(),
            &mut ui.build_ctx(),
        );

        ui.send(self.tree_root, TreeRootMessage::Items(fs_tree.root_items));

        if fs_tree.path_item.is_some() {
            self.select_and_bring_into_view(fs_tree.path_item, ui);
        }
    }

    /// Tries to set a new path. This method keeps only the valid part of the supplied path. For
    /// example, if the path `foo/bar/baz` is supplied and only `foo/bar` exists, then the `foo/bar`
    /// will be set. This method also does path normalization, which requires FS access, and the actual
    /// path will be absolute even if the input path was relative.
    fn set_path(&mut self, path: &Path, ui: &UserInterface) -> bool {
        fn discard_nonexistent_sub_dirs(path: &Path) -> PathBuf {
            let mut potentially_existing_path = path.to_owned();
            while !potentially_existing_path.exists() {
                if !potentially_existing_path.pop() {
                    break;
                }
            }
            potentially_existing_path
        }

        let existing_part = discard_nonexistent_sub_dirs(path);

        match fs_tree::sanitize_path(&existing_part) {
            Ok(existing_sanitized_path) => {
                if self.path != existing_sanitized_path {
                    self.path = existing_sanitized_path;
                    ui.send(
                        self.path_text,
                        TextMessage::Text(self.path.to_string_lossy().to_string()),
                    );
                    true
                } else {
                    false
                }
            }
            Err(err) => {
                err!(
                    "Unable to set existing part {} of the path {}. Reason {:?}",
                    existing_part.display(),
                    path.display(),
                    err
                );
                false
            }
        }
    }

    /// Same as [`Self::set_path`], but also rebuilds the file system tree to the given path.
    /// This method keeps only the valid part of the supplied path. For example, if the path
    /// `foo/bar/baz` is supplied and only `foo/bar` exists, then the tree will be built only to
    /// that path.
    fn set_path_and_rebuild_tree(&mut self, path: &Path, ui: &mut UserInterface) -> bool {
        if !self.set_path(path, ui) {
            return false;
        }
        let existing_item = fs_tree::find_tree_item(self.tree_root, &self.path, ui);
        if existing_item.is_some() {
            self.select_and_bring_into_view(existing_item, ui)
        } else {
            self.rebuild_fs_tree(ui)
        }
        true
    }

    fn set_root(&mut self, root: &Option<PathBuf>, ui: &mut UserInterface) {
        let watcher_replacement = match self.watcher.take() {
            Some(mut watcher) => {
                let current_root = match &self.root {
                    Some(path) => path.clone(),
                    None => self.path.clone(),
                };
                if current_root.exists() {
                    Log::verify(watcher.unwatch(&current_root));
                }
                let new_root = match &root {
                    Some(path) => path.clone(),
                    None => self.path.clone(),
                };
                Log::verify(watcher.watch(&new_root, notify::RecursiveMode::Recursive));
                Some(watcher)
            }
            None => None,
        };
        self.root.clone_from(root);
        self.set_path(&root.clone().unwrap_or_default(), ui);
        self.rebuild_fs_tree(ui);
        for button in [self.home_dir, self.desktop_dir] {
            ui.send(button, WidgetMessage::Visibility(self.root.is_none()));
        }
        self.watcher = watcher_replacement;
    }

    fn on_file_added(&mut self, path: &Path, ui: &mut UserInterface) {
        if !self.filter.passes(path) {
            return;
        }

        let parent_path = parent_path(path);
        let existing_parent_node = fs_tree::find_tree_item(self.tree_root, &parent_path, ui);
        if existing_parent_node.is_none() {
            return;
        }

        let tree = some_or_return!(ui.node(existing_parent_node).cast::<Tree>());
        if tree.is_expanded {
            fs_tree::build_tree(
                existing_parent_node,
                existing_parent_node == self.tree_root,
                path,
                &parent_path,
                self.item_context_menu.clone(),
                ui,
            );
        } else if !tree.always_show_expander {
            ui.send(tree.handle(), TreeMessage::SetExpanderShown(true))
        }
    }

    fn on_file_removed(&mut self, path: &Path, ui: &mut UserInterface) {
        let tree_item = fs_tree::find_tree_item(self.tree_root, &path, ui);
        if tree_item.is_some() {
            let parent_path = parent_path(path);
            let parent_tree = fs_tree::find_tree_item(self.tree_root, &parent_path, ui);
            ui.send(parent_tree, TreeMessage::RemoveItem(tree_item))
        }
    }

    fn handle_fs_event_message(&mut self, msg: &FsEventMessage, ui: &mut UserInterface) {
        match msg {
            FsEventMessage::Add(path) => self.on_file_added(path, ui),
            FsEventMessage::Remove(path) => self.on_file_removed(path, ui),
        }
    }

    fn on_file_browser_message(
        &mut self,
        message: &UiMessage,
        message_data: &FileBrowserMessage,
        ui: &mut UserInterface,
    ) {
        match message_data {
            FileBrowserMessage::Path(path) => {
                if self.set_path_and_rebuild_tree(path, ui) {
                    ui.send_message(UiMessage::from_widget(
                        message.destination(),
                        FileBrowserMessage::Path(self.path.clone()),
                    ));
                }
            }
            FileBrowserMessage::Root(root) => {
                if &self.root != root {
                    self.set_root(root, ui)
                }
            }
            FileBrowserMessage::Filter(filter) => {
                if &self.filter != filter {
                    self.filter.clone_from(filter);
                    self.rebuild_fs_tree(ui);
                }
            }
            FileBrowserMessage::Rescan | FileBrowserMessage::Drop { .. } => (),
            FileBrowserMessage::FocusCurrentPath => {
                let item = fs_tree::find_tree_item(self.tree_root, &self.path, ui);
                if item.is_some() {
                    // Select item of new path.
                    ui.send(self.tree_root, TreeRootMessage::Select(vec![item]));
                    ui.send(self.scroll_viewer, ScrollViewerMessage::BringIntoView(item));
                }
            }
        }
    }

    fn on_sub_tree_expanded(
        &mut self,
        sub_tree: Handle<UiNode>,
        expand: bool,
        ui: &mut UserInterface,
    ) {
        if expand {
            // Look into internals of directory and build tree items.
            if let Some(parent_path) = fs_tree::tree_path(sub_tree, ui) {
                fs_tree::build_single_folder(
                    &parent_path,
                    sub_tree,
                    self.item_context_menu.clone(),
                    &self.filter,
                    ui,
                )
            }
        } else {
            // Nuke everything in collapsed item. This also will free some resources
            // and will speed up layout pass.
            ui.send(
                sub_tree,
                TreeMessage::SetItems {
                    items: vec![],
                    remove_previous: true,
                },
            );
        }
    }

    fn on_sub_tree_selected(&mut self, sub_tree: Handle<UiNode>, ui: &UserInterface) {
        let path = some_or_return!(fs_tree::tree_path(sub_tree, ui));
        if self.path != path {
            // Here we trust the content of the tree items.
            self.path.clone_from(&path);

            ui.send(
                self.path_text,
                TextMessage::Text(path.to_string_lossy().to_string()),
            );

            // Do response.
            ui.post(self.handle, FileBrowserMessage::Path(path));
        }
    }

    fn on_drop(
        &self,
        what_dropped: Handle<UiNode>,
        where_dropped: Handle<UiNode>,
        ui: &UserInterface,
    ) {
        let path = some_or_return!(fs_tree::tree_path(where_dropped, ui));
        ui.post(
            self.handle,
            FileBrowserMessage::Drop {
                dropped: what_dropped,
                path_item: where_dropped,
                path: path.clone(),
                dropped_path: fs_tree::tree_path(what_dropped, ui).unwrap_or_default(),
            },
        );
    }

    fn on_desktop_dir_clicked(&self, ui: &UserInterface) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let user_dirs = directories::UserDirs::new();
            if let Some(desktop_dir) = user_dirs.as_ref().and_then(|dirs| dirs.desktop_dir()) {
                ui.send(
                    self.handle,
                    FileBrowserMessage::Path(desktop_dir.to_path_buf()),
                );
            }
        }
    }

    fn on_home_dir_clicked(&self, ui: &UserInterface) {
        #[cfg(not(target_arch = "wasm32"))]
        {
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

impl Control for FileBrowser {
    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);
        if let Some(msg) = message.data_for::<FsEventMessage>(self.handle) {
            self.handle_fs_event_message(msg, ui);
        } else if let Some(message_data) = message.data_for::<FileBrowserMessage>(self.handle) {
            self.on_file_browser_message(message, message_data, ui)
        } else if let Some(TreeMessage::Expand { expand, .. }) = message.data() {
            self.on_sub_tree_expanded(message.destination(), *expand, ui)
        } else if let Some(WidgetMessage::Drop(dropped)) = message.data() {
            if !message.handled() {
                self.on_drop(*dropped, message.destination(), ui);
                message.set_handled(true);
            }
        } else if let Some(TreeRootMessage::Select(selection)) = message.data_from(self.tree_root) {
            if let Some(&first_selected) = selection.first() {
                self.on_sub_tree_selected(first_selected, ui)
            }
        } else if let Some(ButtonMessage::Click) = message.data_from(self.desktop_dir) {
            self.on_desktop_dir_clicked(ui)
        } else if let Some(ButtonMessage::Click) = message.data_from(self.home_dir) {
            self.on_home_dir_clicked(ui)
        }
    }

    fn accepts_drop(&self, widget: Handle<UiNode>, ui: &UserInterface) -> bool {
        ui.node(widget)
            .user_data
            .as_ref()
            .is_some_and(|data| data.safe_lock().downcast_ref::<PathBuf>().is_some())
    }
}

pub struct FileBrowserBuilder {
    widget_builder: WidgetBuilder,
    path: PathBuf,
    filter: PathFilter,
    root: Option<PathBuf>,
    show_path: bool,
}

impl FileBrowserBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            path: "./".into(),
            filter: PathFilter::AllPass,
            root: None,
            show_path: true,
        }
    }

    pub fn with_filter(mut self, filter: PathFilter) -> Self {
        self.filter = filter;
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
        } = fs_tree::FsTree::new_or_empty(
            self.root.as_ref(),
            self.path.as_path(),
            &self.filter,
            item_context_menu.clone(),
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
                                        .with_visibility(self.root.is_none())
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
                                        .with_visibility(self.root.is_none())
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
                                        .on_row(0)
                                        .on_column(2)
                                        .with_margin(Thickness::uniform(2.0)),
                                )
                                .with_editable(false)
                                .with_text_commit_mode(TextCommitMode::Immediate)
                                .with_vertical_text_alignment(VerticalAlignment::Center)
                                .with_text(
                                    fs_tree::sanitize_path(&self.path)
                                        .ok()
                                        .map(|p| p.to_string_lossy().to_string())
                                        .unwrap_or_default(),
                                )
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
        .add_rows(vec![Row::auto(), Row::stretch()])
        .build(ctx);

        let widget = self
            .widget_builder
            .with_need_update(true)
            .with_child(grid)
            .build(ctx);

        let the_path = match &self.root {
            Some(path) => path.clone(),
            _ => self.path.clone(),
        };
        let browser = FileBrowser {
            widget,
            tree_root,
            home_dir,
            desktop_dir,
            path_text,
            path: self.path,
            filter: self.filter,
            scroll_viewer,
            root: self.root,
            watcher: None,
            item_context_menu,
        };
        let file_browser_handle = ctx.add_node(UiNode::new(browser));
        let sender = ctx.sender();
        ctx[file_browser_handle]
            .cast_mut::<FileBrowser>()
            .unwrap()
            .watcher = setup_file_browser_fs_watcher(sender, file_browser_handle, the_path);
        file_browser_handle
    }
}

struct EventReceiver {
    file_browser_handle: Handle<UiNode>,
    sender: Sender<UiMessage>,
}

impl EventReceiver {
    fn send(&self, message: impl MessageData) {
        Log::verify(
            self.sender
                .send(UiMessage::for_widget(self.file_browser_handle, message)),
        )
    }
}

impl notify::EventHandler for EventReceiver {
    fn handle_event(&mut self, event: notify::Result<Event>) {
        let event = ok_or_return!(event);

        if event.need_rescan() {
            self.send(FileBrowserMessage::Rescan);
            return;
        }

        for path in event.paths.iter() {
            let path = ok_or_continue!(std::path::absolute(path));

            match event.kind {
                notify::EventKind::Remove(_) => {
                    self.send(FsEventMessage::Remove(path.clone()));
                }
                notify::EventKind::Create(_) => {
                    self.send(FsEventMessage::Add(path.clone()));
                }
                _ => (),
            }
        }
    }
}

fn setup_file_browser_fs_watcher(
    sender: Sender<UiMessage>,
    file_browser_handle: Handle<UiNode>,
    the_path: PathBuf,
) -> Option<notify::RecommendedWatcher> {
    let handler = EventReceiver {
        file_browser_handle,
        sender,
    };
    let config = notify::Config::default().with_poll_interval(time::Duration::from_secs(1));
    match notify::RecommendedWatcher::new(handler, config) {
        Ok(mut watcher) => {
            Log::verify(watcher.watch(&the_path, notify::RecursiveMode::Recursive));
            Some(watcher)
        }
        Err(_) => None,
    }
}
