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

use crate::{
    core::{err, parking_lot::Mutex, pool::Handle, SafeLock},
    file_browser::Filter,
    grid::{Column, GridBuilder, Row},
    image::ImageBuilder,
    resources::FOLDER_ICON,
    text::TextBuilder,
    tree::{Tree, TreeBuilder, TreeMessage, TreeRoot, TreeRootMessage},
    widget::WidgetBuilder,
    BuildContext, RcUiNodeHandle, Thickness, UiNode, UserInterface, VerticalAlignment,
};
use fyrox_graph::BaseSceneGraph;
use std::{
    borrow::Cow,
    cmp::Ordering,
    ffi::OsString,
    path::{Component, Path, PathBuf, Prefix},
    sync::Arc,
};

pub fn find_tree<P: AsRef<Path>>(
    node: Handle<UiNode>,
    path: &P,
    ui: &UserInterface,
) -> Handle<UiNode> {
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

pub fn build_tree_item<P: AsRef<Path>>(
    path: P,
    parent_path: P,
    menu: RcUiNodeHandle,
    expanded: bool,
    ctx: &mut BuildContext,
    root_title: Option<&str>,
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
                    if let Some(root_title) = root_title.filter(|_| path.as_ref() == Path::new("."))
                    {
                        root_title.to_string()
                    } else {
                        path.as_ref()
                            .to_string_lossy()
                            .replace(&parent_path.as_ref().to_string_lossy().to_string(), "")
                            .replace('\\', "")
                    },
                )
                .with_vertical_text_alignment(VerticalAlignment::Center)
                .build(ctx),
            ),
    )
    .add_row(Row::stretch())
    .add_column(Column::auto())
    .add_column(Column::stretch())
    .build(ctx);

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
    .with_content(content)
    .build(ctx)
}

pub fn build_tree<P: AsRef<Path>>(
    parent: Handle<UiNode>,
    is_parent_root: bool,
    path: P,
    parent_path: P,
    menu: RcUiNodeHandle,
    root_title: Option<&str>,
    ui: &mut UserInterface,
) -> Handle<UiNode> {
    let subtree = build_tree_item(
        path,
        parent_path,
        menu,
        false,
        &mut ui.build_ctx(),
        root_title,
    );
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
        ui.send(parent, TreeRootMessage::AddItem(tree));
    } else {
        ui.send(parent, TreeMessage::AddItem(tree));
    }
}

fn disk_letter(components: &[Component]) -> Option<u8> {
    if let Some(Component::Prefix(prefix)) = components.first() {
        if let Prefix::Disk(disk_letter) | Prefix::VerbatimDisk(disk_letter) = prefix.kind() {
            return Some(disk_letter);
        }
    }
    None
}

fn sanitize_path(in_path: &Path, root: Option<&PathBuf>) -> PathBuf {
    let mut out_path = PathBuf::new();

    if let Ok(canonical_in_path) = in_path.canonicalize() {
        if let Some(canonical_root) = root.and_then(|r| r.canonicalize().ok()) {
            if let Ok(stripped) = canonical_in_path.strip_prefix(canonical_root) {
                stripped.clone_into(&mut out_path);
            }
        } else {
            out_path = canonical_in_path;
        }
    }

    // There should be at least one component in the path. If the path is empty, this means that
    // it "points" to the current directory.
    if out_path.as_os_str().is_empty() {
        out_path.push(".");
    }

    // Relative paths must always start from CurDir component (./), otherwise the root dir will be ignored
    // and the tree will be incorrect.
    if !out_path.is_absolute() {
        out_path = Path::new(".").join(out_path);
    }

    out_path
}

fn read_dir_entries(dir: &Path, filter: Option<&Filter>) -> std::io::Result<Vec<PathBuf>> {
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

    fn passes_filter(path: &Path, filter: Option<&Filter>) -> bool {
        filter.is_none_or(|filter| filter.0.safe_lock()(path))
    }

    let mut entries = std::fs::read_dir(dir)?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| passes_filter(path, filter))
        .collect::<Vec<_>>();
    entries.sort_unstable_by(sort_dir_entries);
    Ok(entries)
}

#[cfg(not(target_arch = "wasm32"))]
struct DisksProvider {
    sys: sysinfo::System,
}

#[cfg(not(target_arch = "wasm32"))]
impl DisksProvider {
    fn new() -> Self {
        use sysinfo::{RefreshKind, SystemExt};
        Self {
            sys: sysinfo::System::new_with_specifics(RefreshKind::new().with_disks_list()),
        }
    }

    fn iter(&self) -> impl Iterator<Item = (Cow<str>, u8)> {
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
        root_title: Option<&str>,
        ctx: &mut BuildContext,
    ) -> Self {
        if let Some(root) = root {
            let path = if std::env::current_dir().is_ok_and(|dir| &dir == root) {
                Path::new(".")
            } else {
                root.as_path()
            };

            let root_item =
                build_tree_item(path, Path::new(""), menu.clone(), true, ctx, root_title);
            Self {
                items: vec![root_item],
                root_item,
            }
        } else {
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
                    ctx,
                    root_title,
                );

                if is_disk_part_of_path {
                    root_item = item;
                }

                items.push(item);
            }

            Self { items, root_item }
        }
    }
}

pub fn build_single_folder(
    parent_path: &Path,
    tree_item: Handle<UiNode>,
    menu: RcUiNodeHandle,
    root_title: Option<&str>,
    filter: Option<&Filter>,
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
            false,
            path.as_path(),
            parent_path,
            menu.clone(),
            root_title,
            ui,
        );
    }
}

pub struct FsTree {
    pub root_items: Vec<Handle<UiNode>>,
    pub path_item: Handle<UiNode>,
}

impl FsTree {
    /// Builds entire file system tree to given final_path.
    pub fn new(
        root: Option<&PathBuf>,
        final_path: &Path,
        filter: Option<&Filter>,
        menu: RcUiNodeHandle,
        root_title: Option<&str>,
        ctx: &mut BuildContext,
    ) -> Self {
        let dest_path = sanitize_path(final_path, root);

        let dest_path_components = dest_path.components().collect::<Vec<Component>>();

        let RootsCollection {
            items: root_items,
            root_item: mut parent,
        } = RootsCollection::new(&dest_path_components, root, &menu, root_title, ctx);

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
            if let Ok(entries) = read_dir_entries(&full_path, filter) {
                for path in entries {
                    let is_part_of_final_path = next.as_ref().is_some_and(|next| *next == path);

                    let item = build_tree_item(
                        &path,
                        &full_path,
                        menu.clone(),
                        is_part_of_final_path,
                        ctx,
                        root_title,
                    );

                    Tree::add_item(parent, item, ctx);

                    if is_part_of_final_path {
                        new_parent = item;
                    }

                    if path == dest_path {
                        path_item = item;
                    }
                }
            }
            parent = new_parent;
        }

        Self {
            root_items,
            path_item,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::file_browser::{fs_tree::read_dir_entries, Filter};
    use std::{
        fs::File,
        path::{Path, PathBuf},
    };

    fn create_dir(path: impl AsRef<Path>) {
        std::fs::create_dir_all(path).unwrap()
    }

    fn write_empty_file(path: impl AsRef<Path>) {
        File::create(path).unwrap();
    }

    fn create_dirs(paths: &[PathBuf]) {
        for path in paths {
            create_dir(path)
        }
    }

    fn write_empty_files(paths: &[PathBuf]) {
        for path in paths {
            write_empty_file(path)
        }
    }

    fn clean_or_create(path: impl AsRef<Path>) {
        if path.as_ref().exists() {
            std::fs::remove_dir_all(path).unwrap();
        } else {
            create_dir(path);
        }
    }

    #[test]
    fn test_dir_fetching() {
        let path = Path::new("./test_dir_fetching");
        clean_or_create(path);
        let folders = [path.join("dir1"), path.join("dir2"), path.join("dir3")];
        let files = [path.join("file1"), path.join("file2"), path.join("file3")];
        create_dirs(&folders);
        write_empty_files(&files);
        let entries = read_dir_entries(path, None).unwrap();
        assert_eq!(entries[0..3], folders);
        assert_eq!(entries[3..6], files);
        let files_copy = files.clone();
        let folders_copy = folders.clone();
        let entries = read_dir_entries(
            path,
            Some(&Filter::new(move |path| {
                path != files_copy[0] && path != folders_copy[2]
            })),
        )
        .unwrap();
        assert_eq!(entries[0..2], folders[0..2]);
        assert_eq!(entries[2..4], files[1..3]);
    }
}
