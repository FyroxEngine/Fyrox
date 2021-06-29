//! The purpose of this file is using directories-rs library ProjectDirs functions with a little
//! bit of customization. This includes checking for existence and adding file to the end of the path.

use directories::ProjectDirs;
use std::path::PathBuf;

use crate::TEST_EXISTENCE;

fn project_dir() -> ProjectDirs {
    ProjectDirs::from("org", "rg3dengine", "rusty-editor").unwrap()
}

pub fn working_config_dir(filename: &str) -> PathBuf {
    if *TEST_EXISTENCE {
        config_dir(filename)
    } else {
        debug_dir(filename)
    }
}

pub fn working_data_dir(filename: &str) -> PathBuf {
    if *TEST_EXISTENCE {
        data_dir(filename)
    } else {
        debug_dir(filename)
    }
}

fn debug_dir(filename: &str) -> PathBuf {
    std::env::current_dir().unwrap().join(filename)
}

/// These are only used for creating this directories and checking inside
/// TEST_EXISTENCE constant. Because normal ones is using TEST_EXISTENCE constant value,
/// but the constant haven't returned the value to yet, so it crashes.
pub fn config_dir(filename: &str) -> PathBuf {
    project_dir().config_dir().join(filename)
}

pub fn data_dir(filename: &str) -> PathBuf {
    project_dir().data_dir().join(filename)
}

pub fn resources_dir(filename: &str) -> PathBuf {
    PathBuf::from("/usr/lib/rusty-editor/").join(filename)
}
