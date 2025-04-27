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
use cargo_metadata::camino::Utf8Path;
use fyrox::core::log::Log;
use std::{ffi::OsStr, path::Path, process::Stdio};

pub fn build_package(package_dir_path: &Utf8Path) -> std::process::Command {
    let mut process = utils::make_command("wasm-pack");
    process
        .stderr(Stdio::piped())
        .arg("build")
        .arg(package_dir_path)
        .arg("--target")
        .arg("web");
    process
}

pub fn copy_binaries(package_dir_path: &Path, destination_folder: &Path) -> Result<(), String> {
    Log::info("Trying to copy the executable...");

    utils::copy_dir(package_dir_path, destination_folder, &|path: &Path| {
        if path.is_file() {
            if path.file_name() == Some(OsStr::new("Cargo.toml"))
                || path.file_name() == Some(OsStr::new("README.md"))
                || path.file_name() == Some(OsStr::new(".gitignore"))
            {
                return false;
            }
        } else if path.is_dir() && path.file_name() == Some(OsStr::new("target")) {
            return false;
        }

        true
    })
    .map_err(|e| e.to_string())
}

pub fn run_build(destination_folder: &Path) {
    if !utils::is_installed("basic-http-server") {
        Log::verify(utils::cargo_install("basic-http-server"));
    }

    Log::verify(
        utils::make_command("basic-http-server")
            .arg("--addr")
            .arg("127.0.0.1:4000")
            .current_dir(destination_folder)
            .spawn(),
    );

    Log::verify(open::that_detached("http://127.0.0.1:4000"));
}
