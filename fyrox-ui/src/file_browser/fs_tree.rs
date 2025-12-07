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

use crate::file_browser::PathFilter;
use crate::{
    core::{err, pool::Handle},
    grid::{Column, GridBuilder, Row},
    image::ImageBuilder,
    resources::FOLDER_ICON,
    text::TextBuilder,
    tree::{Tree, TreeBuilder, TreeMessage, TreeRoot, TreeRootMessage},
    widget::WidgetBuilder,
    BuildContext, RcUiNodeHandle, Thickness, UiNode, UserInterface, VerticalAlignment,
};
use fyrox_graph::{BaseSceneGraph, SceneGraph};
use std::{
    borrow::Cow,
    cmp::Ordering,
    ffi::OsString,
    path::{Component, Path, PathBuf, Prefix},
};

#[derive(Clone, PartialEq)]
pub(super) struct TreeItemPath {
    path: PathBuf,
    is_root: bool,
}

impl TreeItemPath {
    pub fn non_root(path: PathBuf) -> Self {
        Self {
            path,
            is_root: false,
        }
    }

    pub fn root(path: PathBuf) -> Self {
        Self {
            path,
            is_root: true,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn is_root(&self) -> bool {
        self.is_root
    }

    pub fn into_path(self) -> PathBuf {
        self.path
    }
}

pub fn find_tree_item(node: Handle<UiNode>, path: &Path, ui: &UserInterface) -> Handle<UiNode> {
    let mut tree_handle = Handle::NONE;
    let node_ref = ui.node(node);

    if let Some(tree) = node_ref.cast::<Tree>() {
        let tree_path = tree.user_data_cloned::<TreeItemPath>();
        if tree_path.is_some_and(|p| p.path() == path) {
            tree_handle = node;
        } else {
            for &item in &tree.items {
                let tree = find_tree_item(item, path, ui);
                if tree.is_some() {
                    tree_handle = tree;
                    break;
                }
            }
        }
    } else if let Some(root) = node_ref.cast::<TreeRoot>() {
        let root_path = root.user_data_cloned::<TreeItemPath>();
        if root_path.is_some_and(|p| p.path() == path) {
            tree_handle = node;
        } else {
            for &item in &root.items {
                let tree = find_tree_item(item, path, ui);
                if tree.is_some() {
                    tree_handle = tree;
                    break;
                }
            }
        }
    } else {
        unreachable!()
    }
    tree_handle
}

pub fn build_tree_item<P: AsRef<Path>>(
    path: P,
    parent_path: P,
    menu: RcUiNodeHandle,
    expanded: bool,
    filter: &PathFilter,
    ctx: &mut BuildContext,
) -> Handle<UiNode> {
    let content = GridBuilder::new(
        WidgetBuilder::new()
            .with_child(if path.as_ref().is_dir() {
                ImageBuilder::new(
                    WidgetBuilder::new()
                        .with_width(16.0)
                        .with_height(16.0)
                        .on_column(0)
                        .with_margin(Thickness {
                            left: 4.0,
                            top: 1.0,
                            right: 1.0,
                            bottom: 1.0,
                        }),
                )
                .with_opt_texture(FOLDER_ICON.clone())
                .build(ctx)
            } else {
                Handle::NONE
            })
            .with_child(
                TextBuilder::new(
                    WidgetBuilder::new()
                        .with_margin(Thickness::left(4.0))
                        .on_column(1),
                )
                .with_text(
                    path.as_ref()
                        .to_string_lossy()
                        .replace(&parent_path.as_ref().to_string_lossy().to_string(), "")
                        .replace('\\', ""),
                )
                .with_vertical_text_alignment(VerticalAlignment::Center)
                .build(ctx),
            ),
    )
    .add_row(Row::stretch())
    .add_column(Column::auto())
    .add_column(Column::stretch())
    .build(ctx);

    let is_dir_not_empty = path.as_ref().read_dir().is_ok_and(|iter| {
        iter.flatten()
            .any(|entry| filter.supports_all(&entry.path()))
    });
    TreeBuilder::new(
        WidgetBuilder::new()
            .with_user_data_value(TreeItemPath::non_root(path.as_ref().to_owned()))
            .with_context_menu(menu),
    )
    .with_expanded(expanded)
    .with_always_show_expander(is_dir_not_empty)
    .with_content(content)
    .build(ctx)
}

pub fn build_tree<P: AsRef<Path>>(
    parent: Handle<UiNode>,
    path: P,
    parent_path: P,
    menu: RcUiNodeHandle,
    filter: &PathFilter,
    ui: &mut UserInterface,
) -> Handle<UiNode> {
    let subtree = build_tree_item(path, parent_path, menu, false, filter, &mut ui.build_ctx());
    if ui[parent].has_component::<TreeRoot>() {
        ui.send(parent, TreeRootMessage::AddItem(subtree));
    } else {
        ui.send(parent, TreeMessage::AddItem(subtree));
    }
    subtree
}

pub(super) fn sanitize_path(path: &Path) -> std::io::Result<PathBuf> {
    let canonical_path = path.canonicalize()?;
    let mut sanitized_path = PathBuf::with_capacity(canonical_path.capacity());
    for component in canonical_path.components() {
        if let Component::Prefix(prefix) = component {
            match prefix.kind() {
                Prefix::Verbatim(_) => {
                    // Skip
                }
                Prefix::VerbatimUNC(_, _) | Prefix::UNC(_, _) => {
                    return Err(std::io::Error::other(
                        "paths with UNC prefix aren't supported!",
                    ))
                }
                Prefix::VerbatimDisk(letter) | Prefix::Disk(letter) => {
                    sanitized_path.push(format!("{}:", char::from(letter)))
                }
                Prefix::DeviceNS(_) => {
                    return Err(std::io::Error::other(
                        "paths with device prefix aren't supported!",
                    ))
                }
            }
        } else {
            sanitized_path.push(component);
        }
    }
    Ok(sanitized_path)
}

struct SanitizedPath {
    path: PathBuf,
    sanitized_root: Option<PathBuf>,
    root_components_to_skip: usize,
}

impl SanitizedPath {
    fn new(path: &Path, root: Option<&PathBuf>) -> std::io::Result<Self> {
        let path = sanitize_path(path)?;
        if let Some(root) = root {
            let sanitized_root = sanitize_path(root)?;
            let root_components_to_skip = sanitized_root.components().count().saturating_sub(1);
            Ok(Self {
                path,
                sanitized_root: Some(sanitized_root),
                root_components_to_skip,
            })
        } else {
            Ok(Self {
                path,
                sanitized_root: None,
                root_components_to_skip: 0,
            })
        }
    }
}

pub(super) fn read_dir_entries(dir: &Path, filter: &PathFilter) -> std::io::Result<Vec<PathBuf>> {
    #[allow(clippy::ptr_arg)]
    fn sort_dir_entries(a: &PathBuf, b: &PathBuf) -> Ordering {
        let a_is_dir = a.is_dir();
        let b_is_dir = b.is_dir();

        if a_is_dir && !b_is_dir {
            Ordering::Less
        } else if !a_is_dir && b_is_dir {
            Ordering::Greater
        } else {
            fn lowercase_file_name(path: &Path) -> OsString {
                path.file_name()
                    .expect("file name must exist")
                    .to_ascii_lowercase()
            }
            lowercase_file_name(a).cmp(&lowercase_file_name(b))
        }
    }

    let mut entries = std::fs::read_dir(dir)?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| filter.supports_all(path))
        .collect::<Vec<_>>();
    entries.sort_unstable_by(sort_dir_entries);
    Ok(entries)
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) struct DisksProvider {
    sys: sysinfo::System,
}

#[cfg(not(target_arch = "wasm32"))]
impl DisksProvider {
    pub(super) fn new() -> Self {
        use sysinfo::{RefreshKind, SystemExt};
        Self {
            sys: sysinfo::System::new_with_specifics(RefreshKind::new().with_disks_list()),
        }
    }

    pub(super) fn iter(&self) -> impl Iterator<Item = (Cow<str>, u8)> {
        use sysinfo::{DiskExt, SystemExt};
        self.sys.disks().iter().map(|disk| {
            let mount_point = disk.mount_point().to_string_lossy();
            let disk_letter = mount_point.chars().next().unwrap() as u8;
            (mount_point, disk_letter)
        })
    }
}

#[cfg(target_arch = "wasm32")]
struct DisksProvider;

#[cfg(target_arch = "wasm32")]
impl DisksProvider {
    fn new() -> Self {
        Self
    }

    fn iter(&self) -> impl Iterator<Item = (Cow<str>, u8)> {
        std::iter::empty()
    }
}

struct RootsCollection {
    items: Vec<Handle<UiNode>>,
    root_item: Handle<UiNode>,
}

impl RootsCollection {
    fn new(
        path_components: &[Component],
        root: Option<&PathBuf>,
        menu: &RcUiNodeHandle,
        filter: &PathFilter,
        ctx: &mut BuildContext,
    ) -> Self {
        if root.is_none() {
            fn disk_letter(components: &[Component]) -> Option<u8> {
                if let Some(Component::Prefix(prefix)) = components.first() {
                    if let Prefix::Disk(disk_letter) | Prefix::VerbatimDisk(disk_letter) =
                        prefix.kind()
                    {
                        return Some(disk_letter);
                    }
                }
                None
            }

            let dest_disk = disk_letter(path_components);

            let mut items = Vec::new();
            let mut root_item = Handle::NONE;

            for (disk, disk_letter) in DisksProvider::new().iter() {
                let is_disk_part_of_path = dest_disk == Some(disk_letter);

                let item = build_tree_item(
                    disk.as_ref(),
                    "",
                    menu.clone(),
                    is_disk_part_of_path,
                    filter,
                    ctx,
                );

                if is_disk_part_of_path {
                    root_item = item;
                }

                items.push(item);
            }

            Self { items, root_item }
        } else {
            Self {
                items: Default::default(),
                root_item: Default::default(),
            }
        }
    }
}

pub fn build_single_folder(
    parent_path: &Path,
    tree_item: Handle<UiNode>,
    menu: RcUiNodeHandle,
    filter: &PathFilter,
    ui: &mut UserInterface,
) {
    let Ok(entries) = read_dir_entries(parent_path, filter) else {
        err!(
            "Unable to fetch FS content for path {}!",
            parent_path.display()
        );
        return;
    };

    for path in entries {
        build_tree(
            tree_item,
            path.as_path(),
            parent_path,
            menu.clone(),
            filter,
            ui,
        );
    }
}

pub fn tree_path(tree_handle: Handle<UiNode>, ui: &UserInterface) -> Option<TreeItemPath> {
    ui.try_get(tree_handle)
        .and_then(|n| n.user_data_cloned::<TreeItemPath>())
}

pub struct FsTree {
    pub root_items: Vec<Handle<UiNode>>,
    pub path_item: Handle<UiNode>,
    pub items_count: usize,
    pub sanitized_root: Option<PathBuf>,
}

impl FsTree {
    fn empty() -> Self {
        Self {
            root_items: Default::default(),
            path_item: Default::default(),
            items_count: 0,
            sanitized_root: Default::default(),
        }
    }

    /// Builds entire file system tree to given final_path.
    pub fn new(
        root: Option<&PathBuf>,
        path: &Path,
        filter: &PathFilter,
        menu: RcUiNodeHandle,
        ctx: &mut BuildContext,
    ) -> std::io::Result<Self> {
        let SanitizedPath {
            path,
            sanitized_root,
            root_components_to_skip,
        } = SanitizedPath::new(path, root)?;

        let dest_path_components = path.components().collect::<Vec<Component>>();

        let RootsCollection {
            items: mut root_items,
            root_item: mut parent,
        } = RootsCollection::new(&dest_path_components, root, &menu, filter, ctx);

        let mut path_item = Handle::NONE;

        // Try to build tree only for given path.
        let mut items_count = 0;
        let mut full_path = PathBuf::new();
        for (i, component) in dest_path_components.iter().enumerate() {
            // Concat parts of path one by one.
            full_path = full_path.join(component.as_os_str());

            if i < root_components_to_skip {
                continue;
            }

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
            if let Ok(dir_entries) = read_dir_entries(&full_path, filter) {
                for dir_path in dir_entries {
                    let is_part_of_final_path = next.as_ref().is_some_and(|next| *next == dir_path);

                    let item = build_tree_item(
                        &dir_path,
                        &full_path,
                        menu.clone(),
                        is_part_of_final_path,
                        filter,
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

                    if dir_path == path {
                        path_item = item;
                    }

                    items_count += 1;
                }
            }
            parent = new_parent;
        }

        Ok(Self {
            root_items,
            path_item,
            items_count,
            sanitized_root,
        })
    }

    pub fn new_or_empty(
        root: Option<&PathBuf>,
        path: &Path,
        filter: &PathFilter,
        menu: RcUiNodeHandle,
        ctx: &mut BuildContext,
    ) -> Self {
        match Self::new(root, path, filter, menu, ctx) {
            Ok(fs_tree) => fs_tree,
            Err(err) => {
                err!(
                    "Unable to rebuild FS tree for {} path! Reason: {}",
                    path.display(),
                    err
                );
                Self::empty()
            }
        }
    }
}
