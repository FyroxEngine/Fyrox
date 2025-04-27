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

use crate::export::{utils, ExportOptions};
use cargo_metadata::{camino::Utf8Path, Metadata, Package};
use fyrox::core::log::Log;
use std::{
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
    process::Stdio,
};

pub fn build_package(package_name: &str, build_target: &str) -> std::process::Command {
    let mut process = utils::make_command("cargo-apk");
    process
        .stderr(Stdio::piped())
        .arg("apk")
        .arg("build")
        .arg("--package")
        .arg(package_name)
        .arg("--target")
        .arg(build_target)
        .arg("--release");
    process
}

pub fn copy_binaries(
    metadata: &Metadata,
    package_name: &str,
    destination_folder: &Path,
) -> Result<(), String> {
    Log::info("Trying to copy the apk...");

    let mut binary_paths = vec![];
    for entry in fs::read_dir(metadata.target_directory.join("release/apk"))
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

pub fn run_build(package_name: &str, destination_folder: &Path) {
    if let Ok(adb) = utils::make_command("adb")
        .current_dir(destination_folder)
        .arg("install")
        .arg(format!("{package_name}.apk"))
        .spawn()
    {
        match adb.wait_with_output() {
            Ok(_) => {
                let compatible_package_name = package_name.replace('-', "_");
                Log::verify(
                    utils::make_command("adb")
                        .arg("shell")
                        .arg("am")
                        .arg("start")
                        .arg("-n")
                        .arg(format!(
                            "rust.{compatible_package_name}/android.app.NativeActivity"
                        ))
                        .spawn(),
                );
            }
            Err(err) => Log::err(format!("ADB error: {err:?}")),
        }
    }
}

pub fn copy_assets(
    export_options: &ExportOptions,
    package: &Package,
    package_dir_path: &Utf8Path,
    temp_folders: &mut Vec<PathBuf>,
) -> Result<(), String> {
    // Asset management on Android is quite annoying, because all other target platforms
    // uses the workspace manifest path as a root directory and all paths in code/assets
    // stored relatively to it. On Android, however, all your assets must be in unified
    // assets storage. This means that, if we simply specify assets folder to be `../data`
    // (relative to `executor-android`), it will put all the assets in the storage, but
    // their path will become relative to the storage. For example, in your code you can
    // reference an asset like this: `data/my/textures/foo.jpg` and when the build script for
    // Android will clone the assets from `data` folder, your asset will have this
    // actual path `my/textures/foo.jpg`. In other words, `data` is stripped from the path.
    //
    // To solve this, we just copy the entire assets folder to a temporary folder set in
    // the manifest of `executor-android` and then cargo-apk will pack these assets and the
    // paths to assets will become valid.
    //
    // It could very well be possible that I'm missing something, and this could be fixed in
    // a much easier way.
    if let Some(assets) = package
        .metadata
        .get("android")
        .and_then(|v| v.get("assets"))
        .and_then(|v| v.as_str())
    {
        let temp_assets_storage = package_dir_path.join(assets).as_std_path().to_path_buf();

        Log::info(format!(
            "Trying to copy the assets to a temporary storage {}...",
            temp_assets_storage.display()
        ));

        if !temp_assets_storage.exists() {
            Log::verify(std::fs::create_dir_all(&temp_assets_storage));
        }

        temp_folders.push(temp_assets_storage.clone());

        for folder in &export_options.assets_folders {
            Log::info(format!(
                "Trying to copy assets from {} to {}...",
                folder.display(),
                temp_assets_storage.display()
            ));

            Log::verify(utils::copy_dir(
                folder,
                temp_assets_storage.join(folder),
                &|_| true,
            ));
        }

        Ok(())
    } else {
        Err("Android executor must specify assets folder in \
                    [package.metadata.android] section"
            .to_string())
    }
}
