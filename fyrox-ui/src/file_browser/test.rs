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
    core::{algebra::Vector2, parking_lot::Mutex, pool::Handle},
    file_browser::{
        fs_tree::{self, read_dir_entries, DisksProvider},
        FileBrowserBuilder, FileBrowserMessage, PathFilter,
    },
    test::{test_widget_deletion, UserInterfaceTestingExtension},
    tree::{Tree, TreeRoot, TreeRootBuilder},
    widget::WidgetBuilder,
    RcUiNodeHandle, UiNode, UserInterface,
};
use fyrox_graph::SceneGraph;
use std::{
    fs::File,
    path::{Path, PathBuf},
    sync::Arc,
};

fn create_dir(path: impl AsRef<Path>) {
    std::fs::create_dir_all(path).unwrap()
}

fn write_empty_file(path: impl AsRef<Path>) {
    if let Some(parent) = path.as_ref().parent() {
        if !parent.exists() {
            create_dir(parent)
        }
    }
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

fn find_by_path(path: impl AsRef<Path>, ui: &UserInterface) -> Handle<UiNode> {
    ui.find_from_root(&mut |n| {
        n.user_data_cloned::<PathBuf>()
            .is_some_and(|p| p == path.as_ref())
    })
    .map(|(h, _)| h)
    .unwrap_or_default()
}

fn write_test_tree(root: &Path) -> Vec<PathBuf> {
    clean_or_create(root);
    let paths = [
        root.join("file1"),
        root.join("file2"),
        root.join("file3"),
        root.join("subdir").join("file1"),
        root.join("subdir").join("file2"),
        root.join("subdir").join("file3"),
    ];
    write_empty_files(&paths);
    paths.to_vec()
}

fn count_tree_items(ui: &UserInterface) -> usize {
    let mut count = 0;
    for node in ui.nodes() {
        if node.has_component::<Tree>() {
            count += 1;
        }
    }
    count
}

fn count_tree_roots(ui: &UserInterface) -> usize {
    let mut count = 0;
    for node in ui.nodes() {
        if node.has_component::<TreeRoot>() {
            count += 1;
        }
    }
    count
}

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
        &mut ui,
    );

    while ui.poll_message().is_some() {}

    // This passes.
    assert_eq!(fs_tree::find_tree_item(root, &"./test/path1", &ui), path);

    // This expected to fail
    // https://github.com/rust-lang/rust/issues/31374
    assert_eq!(
        fs_tree::find_tree_item(root, &"test/path1", &ui),
        Handle::NONE
    );
}

#[test]
fn test_dir_fetching() {
    let path = Path::new("./test_dir_fetching");
    clean_or_create(path);
    let folders = [path.join("dir1"), path.join("dir2"), path.join("dir3")];
    let files = [path.join("file1"), path.join("file2"), path.join("file3")];
    create_dirs(&folders);
    write_empty_files(&files);
    let entries = read_dir_entries(path, &PathFilter::AllPass).unwrap();
    assert_eq!(entries[0..3], folders);
    assert_eq!(entries[3..6], files);
    let files_copy = files.clone();
    let folders_copy = folders.clone();
    let entries = read_dir_entries(
        path,
        &PathFilter::new(move |path| path != files_copy[0] && path != folders_copy[2]),
    )
    .unwrap();
    assert_eq!(entries[0..2], folders[0..2]);
    assert_eq!(entries[2..4], files[1..3]);
}

#[test]
fn test_fs_tree_with_root() {
    let root = PathBuf::from("./test_fs_tree_with_root");
    let paths = write_test_tree(&root);
    let screen_size = Vector2::repeat(1000.0);
    let mut ui = UserInterface::new(screen_size);
    let ctx = &mut ui.build_ctx();
    FileBrowserBuilder::new(WidgetBuilder::new())
        .with_root(root)
        .with_path(paths.last().unwrap())
        .build(ctx);
    ui.poll_all_messages();
    for path in paths {
        let path = fs_tree::sanitize_path(&path).unwrap();
        assert!(find_by_path(path, &ui).is_some());
    }
    for (mount_point, _) in DisksProvider::new().iter() {
        assert!(find_by_path(mount_point.to_string(), &ui).is_none());
    }
}

#[test]
fn test_fs_tree_with_root_and_empty_tree() {
    let root = PathBuf::from("./test_fs_tree_with_root_empty");
    clean_or_create(&root);
    let screen_size = Vector2::repeat(1000.0);
    let mut ui = UserInterface::new(screen_size);
    let ctx = &mut ui.build_ctx();
    FileBrowserBuilder::new(WidgetBuilder::new())
        .with_root(root)
        .build(ctx);
    ui.poll_all_messages();
    assert_eq!(count_tree_items(&ui), 0);
    assert_eq!(count_tree_roots(&ui), 1);
    for (mount_point, _) in DisksProvider::new().iter() {
        assert!(find_by_path(mount_point.to_string(), &ui).is_none());
    }
}

#[test]
fn test_fs_tree_without_root() {
    let root = PathBuf::from("./test_fs_tree_without_root");
    let paths = write_test_tree(&root);
    let screen_size = Vector2::repeat(1000.0);
    let mut ui = UserInterface::new(screen_size);
    let ctx = &mut ui.build_ctx();
    let browser = FileBrowserBuilder::new(WidgetBuilder::new())
        .with_path(paths.last().unwrap())
        .build(ctx);
    ui.poll_all_messages();
    for path in &paths {
        let path = fs_tree::sanitize_path(&path).unwrap();
        assert!(find_by_path(path, &ui).is_some());
    }
    for (mount_point, _) in DisksProvider::new().iter() {
        assert!(find_by_path(mount_point.to_string(), &ui).is_some());
    }
    for path in &paths {
        ui.send(browser, FileBrowserMessage::Path(path.clone()));
        let mut response_count = 0;
        while let Some(message) = ui.poll_message() {
            if let Some(FileBrowserMessage::Path(response_path)) = message.data_from(browser) {
                response_count += 1;
                assert_eq!(path, response_path);
            }
        }
        assert_eq!(response_count, 1);
        for path in &paths {
            let path = fs_tree::sanitize_path(&path).unwrap();
            assert!(find_by_path(path, &ui).is_some());
        }
    }
}

#[test]
fn test_fs_tree_invalid_path() {
    let screen_size = Vector2::repeat(1000.0);
    let mut ui = UserInterface::new(screen_size);
    let ctx = &mut ui.build_ctx();
    FileBrowserBuilder::new(WidgetBuilder::new())
        .with_path("foo/bar/baz")
        .build(ctx);
    ui.poll_all_messages();
    assert_eq!(count_tree_items(&ui), 0);
    assert_eq!(count_tree_roots(&ui), 1);
}
