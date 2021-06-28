extern crate directories;

use directories::ProjectDirs;
use std::path::PathBuf;

use crate::TEST_EXISTENCE;

/*
The purpose of this file is using directories-rs library
ProjectDirs functions with a little bit of customization.
This includes checking for existance and adding file to the end of the path.
*/

pub fn config_dir(filename: &str) -> PathBuf {
    if *TEST_EXISTENCE {
        let project_dir = ProjectDirs::from("org", "rg3dengine", "rusty-editor");
        let dirs = project_dir.unwrap().config_dir().join(filename);

        dirs
    } else {
        debug_dir(filename)
    }
}

pub fn other_dir(filename: &str) -> PathBuf {
    if *TEST_EXISTENCE {
        let project_dir = ProjectDirs::from("org", "rg3dengine", "rusty-editor");
        let dirs = project_dir.unwrap().data_dir().join(filename);

        dirs
    } else {
        debug_dir(filename)
    }
}

fn debug_dir(filename: &str) -> PathBuf {
    let dirs = std::env::current_dir().unwrap().join(filename);

    dirs
}

//these are only used for creating this directories and checking inside
//TEST_EXISTENCE constant. Because normal ones is using TEST_EXISTENCE constant value,
//but the constant haven't returned the value to yet, so it crashes.
pub fn config_dir_test(filename: &str) -> PathBuf {
    let project_dir = ProjectDirs::from("org", "rg3dengine", "rusty-editor");
    let dirs = project_dir.unwrap().config_dir().join(filename);

    dirs
}

pub fn other_dir_test(filename: &str) -> PathBuf {
    let project_dir = ProjectDirs::from("org", "rg3dengine", "rusty-editor");
    let dirs = project_dir.unwrap().data_dir().join(filename);

    dirs
}

pub fn resources_dir_test(filename: &str) -> PathBuf {
    let dirs = "/usr/lib/rusty-editor/";

    let mut path = PathBuf::new();
    path.push(dirs);
    path.push(filename);

    path
}
