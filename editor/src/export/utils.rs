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

use crate::fyrox::core::platform::TargetPlatform;
use cargo_metadata::Metadata;
use fyrox::core::log::Log;
use std::path::Path;
use std::process::Stdio;
use std::{fs, io};

pub fn copy_dir_ex<F, C>(
    src: impl AsRef<Path>,
    dst: impl AsRef<Path>,
    filter: &F,
    copy: &mut C,
) -> io::Result<()>
where
    F: Fn(&Path) -> bool,
    C: FnMut(&Path, &Path) -> io::Result<()>,
{
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let path = entry.path();
        if !filter(&path) {
            continue;
        }
        if ty.is_dir() {
            copy_dir_ex(path, dst.as_ref().join(entry.file_name()), filter, copy)?;
        } else {
            let from = path;
            let to = dst.as_ref().join(entry.file_name());
            copy(&from, &to)?;
            Log::info(format!(
                "{} successfully cloned to {}",
                from.display(),
                to.display()
            ))
        }
    }
    Ok(())
}

pub fn copy_dir<F>(
    src_dir: impl AsRef<Path>,
    dst_dir: impl AsRef<Path>,
    filter: &F,
) -> io::Result<()>
where
    F: Fn(&Path) -> bool,
{
    copy_dir_ex(src_dir, dst_dir, filter, &mut |src_file, dst_file| {
        fs::copy(src_file, dst_file)?;
        Ok(())
    })
}

pub fn make_command(program: &str) -> std::process::Command {
    let mut command = std::process::Command::new(program);
    // Remove the `RUSTFLAGS` environment variable, which could be added to the child process
    // implicitly. It is very important if the editor is running in hot-reloading mode, which
    // requires to have custom `RUSTFLAGS=-C prefer-dynamic=yes` environment variable set. This
    // variable also forces any child `cargo` processes to generate binaries with dynamic
    // linking to the standard library which is a major issue on some platforms. See this issue
    // https://github.com/FyroxEngine/Fyrox/issues/679 for more info.
    command.env_remove("RUSTFLAGS");
    command
}

pub fn read_metadata() -> Result<Metadata, String> {
    match make_command("cargo")
        .arg("metadata")
        .stdout(Stdio::piped())
        .spawn()
    {
        Ok(handle) => match handle.wait_with_output() {
            Ok(output) => match serde_json::from_slice::<Metadata>(&output.stdout) {
                Ok(metadata) => Ok(metadata),
                Err(err) => Err(format!(
                    "Unable to parse workspace metadata. Reason {err:?}"
                )),
            },
            Err(err) => Err(format!("Unable to fetch project metadata. Reason {err:?}")),
        },
        Err(err) => Err(format!("Unable to fetch project metadata. Reason {err:?}")),
    }
}

pub fn prepare_build_dir(path: &Path) -> Result<(), String> {
    if path.exists() {
        Log::info("Trying to delete previous build...");

        if let Err(err) = fs::remove_dir_all(path) {
            return Err(format!(
                "Unable to remove previous build at destination path! Reason: {err:?}"
            ));
        }
    }

    // Create the new clean folder.
    if let Err(err) = fs::create_dir_all(path) {
        return Err(format!(
            "Unable to create build directory at destination path! Reason: {err:?}"
        ));
    }

    Ok(())
}

pub fn is_installed(program: &str) -> bool {
    if let Ok(mut handle) = make_command(program)
        // Assuming that `help` command is always present.
        .arg("--help")
        .spawn()
    {
        if let Ok(code) = handle.wait() {
            if code.code().unwrap_or(1) == 0 {
                return true;
            }
        }
    }

    false
}

pub fn cargo_install(crate_name: &str) -> Result<(), String> {
    Log::info(format!("Trying to install {crate_name}..."));

    let mut process = make_command("cargo");
    match process
        .stderr(Stdio::piped())
        .arg("install")
        .arg(crate_name)
        .spawn()
    {
        Ok(handle) => match handle.wait_with_output() {
            Ok(output) => {
                if output.status.code().unwrap_or(1) == 0 {
                    Log::info(format!("{crate_name} installed successfully!"));

                    Ok(())
                } else {
                    Err(String::from_utf8_lossy(&output.stderr).to_string())
                }
            }
            Err(err) => Err(format!("Unable to install {crate_name}. Reason: {err:?}")),
        },
        Err(err) => Err(format!("Unable to install {crate_name}. Reason: {err:?}")),
    }
}

pub fn install_build_target(target: &str) -> Result<(), String> {
    Log::info(format!("Trying to install {target} build target..."));

    let mut process = make_command("rustup");
    match process
        .stderr(Stdio::piped())
        .arg("target")
        .arg("add")
        .arg(target)
        .spawn()
    {
        Ok(handle) => match handle.wait_with_output() {
            Ok(output) => {
                if output.status.code().unwrap_or(1) == 0 {
                    Log::info(format!("{target} target installed successfully!"));

                    Ok(())
                } else {
                    Err(String::from_utf8_lossy(&output.stderr).to_string())
                }
            }
            Err(err) => Err(format!(
                "Unable to install {target} target. Reason: {err:?}"
            )),
        },
        Err(err) => Err(format!(
            "Unable to install {target} target. Reason: {err:?}"
        )),
    }
}

pub fn configure_build_environment(
    target_platform: TargetPlatform,
    build_target: &str,
) -> Result<(), String> {
    match target_platform {
        TargetPlatform::PC => {
            // Assume that rustup have installed the correct toolchain.
            Ok(())
        }
        TargetPlatform::WebAssembly => {
            // Check if the user have `wasm-pack` installed.
            if !is_installed("wasm-pack") {
                cargo_install("wasm-pack")?;
            }
            install_build_target(build_target)
        }
        TargetPlatform::Android => {
            if !is_installed("cargo-apk") {
                cargo_install("cargo-apk")?;
            }
            install_build_target(build_target)
        }
    }
}
