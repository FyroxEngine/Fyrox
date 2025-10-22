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

use crate::export::utils;
use cargo_metadata::Metadata;
use fyrox::core::log::Log;
use std::{ffi::OsStr, fs, path::Path, process::Stdio};

pub fn build_package(package_name: &str, enable_optimization: bool) -> std::process::Command {
    let mut process = utils::make_command("cargo");
    process
        .stderr(Stdio::piped())
        .arg("build")
        .arg("--package")
        .arg(package_name);
    if enable_optimization {
        process.arg("--release");
    }
    process
}

// TODO: This should be replaced with `--out-dir` flag to cargo when it is stabilized.
pub fn copy_binaries(
    metadata: &Metadata,
    package_name: &str,
    destination_folder: &Path,
) -> Result<(), String> {
    Log::info("Trying to copy the executable...");

    let mut binary_paths = vec![];
    for entry in fs::read_dir(metadata.target_directory.join("release"))
        .unwrap()
        .flatten()
    {
        if let Ok(file_metadata) = entry.metadata() {
            if !file_metadata.file_type().is_file() {
                continue;
            }
        }

        if let Some(stem) = entry.path().file_stem() {
            if stem == OsStr::new(package_name) {
                binary_paths.push(entry.path());
            }
        }
    }
    for path in binary_paths {
        if let Some(file_name) = path.file_name() {
            match fs::copy(&path, destination_folder.join(file_name)) {
                Ok(_) => {
                    Log::info(format!(
                        "{} was successfully copied to the {} folder.",
                        path.display(),
                        destination_folder.display()
                    ));
                }
                Err(err) => {
                    Log::warn(format!(
                        "Failed to copy {} file to the {} folder. Reason: {:?}",
                        path.display(),
                        destination_folder.display(),
                        err
                    ));
                }
            }
        }
    }

    Ok(())
}

pub fn run_build(destination_folder: &Path, package_name: &str) {
    #[allow(unused_mut)]
    let mut path = destination_folder.join(package_name);
    #[cfg(windows)]
    {
        path.set_extension("exe");
    }
    Log::verify(open::that_detached(path))
}
