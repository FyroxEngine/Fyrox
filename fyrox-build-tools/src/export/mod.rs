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

use cargo_metadata::camino::Utf8Path;
use clap::Parser;
pub use fyrox_core::platform::TargetPlatform;
use fyrox_core::{
    err,
    log::{Log, MessageKind},
    reflect::prelude::*,
};
use fyrox_resource::manager::ResourceManager;
use std::{
    io::{BufRead, BufReader},
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

pub mod android;
pub mod asset;
pub mod pc;
pub mod utils;
pub mod wasm;

pub struct BuildOutput {
    pub child_processes: Vec<std::process::Child>,
}

pub type BuildResult = Result<BuildOutput, String>;

#[derive(Reflect, Debug, Clone)]
pub struct ExportOptions {
    #[reflect(hidden)]
    pub target_platform: TargetPlatform,
    pub destination_folder: PathBuf,
    pub include_used_assets: bool,
    pub assets_folders: Vec<PathBuf>,
    pub ignored_extensions: Vec<String>,
    #[reflect(hidden)]
    pub build_target: String,
    pub run_after_build: bool,
    pub open_destination_folder: bool,
    pub convert_assets: bool,
    pub enable_optimization: bool,
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            target_platform: Default::default(),
            destination_folder: "./build/".into(),
            assets_folders: vec!["./data/".into()],
            include_used_assets: false,
            ignored_extensions: vec!["log".to_string()],
            build_target: "default".to_string(),
            run_after_build: false,
            open_destination_folder: true,
            convert_assets: true,
            enable_optimization: true,
        }
    }
}

pub fn build_package(
    package_name: &str,
    build_target: &str,
    package_dir_path: &Utf8Path,
    target_platform: TargetPlatform,
    cancel_flag: Arc<AtomicBool>,
    enable_optimization: bool,
) -> Result<(), String> {
    utils::configure_build_environment(target_platform, build_target)?;

    let mut process = match target_platform {
        TargetPlatform::PC => pc::build_package(package_name, enable_optimization),
        TargetPlatform::WebAssembly => wasm::build_package(package_dir_path, enable_optimization),
        TargetPlatform::Android => {
            android::build_package(package_name, build_target, enable_optimization)
        }
    };

    let mut handle = match process.spawn() {
        Ok(handle) => handle,
        Err(err) => {
            return Err(format!("Failed to build the game. Reason: {err:?}"));
        }
    };

    let mut stderr = handle.stderr.take().unwrap();

    // Spin until the build is finished.
    loop {
        if cancel_flag.load(Ordering::Relaxed) {
            Log::verify(handle.kill());
            Log::warn("Build was cancelled.");
            return Ok(());
        }

        for line in BufReader::new(&mut stderr).lines().take(10).flatten() {
            Log::writeln(MessageKind::Information, line);
        }

        match handle.try_wait() {
            Ok(status) => {
                if let Some(status) = status {
                    let code = status.code().unwrap_or(1);
                    if code != 0 {
                        return Err("Failed to build the game.".to_string());
                    } else {
                        Log::info("The game was built successfully.");
                        break;
                    }
                }
            }
            Err(err) => {
                return Err(format!("Failed to build the game. Reason: {err:?}"));
            }
        }

        std::thread::sleep(Duration::from_millis(500));
    }

    Ok(())
}

pub fn export(
    export_options: ExportOptions,
    cancel_flag: Arc<AtomicBool>,
    resource_manager: ResourceManager,
) -> BuildResult {
    Log::info("Building the game...");

    utils::prepare_build_dir(&export_options.destination_folder)?;
    let metadata = utils::read_metadata()?;

    let package_name = match export_options.target_platform {
        TargetPlatform::PC => "executor",
        TargetPlatform::WebAssembly => "executor-wasm",
        TargetPlatform::Android => "executor-android",
    };

    let Some(package) = metadata
        .packages
        .iter()
        .find(|p| p.name.as_ref() == package_name)
    else {
        return Err(format!(
            "The project does not have `{package_name}` package."
        ));
    };

    let package_dir_path = package.manifest_path.as_path().parent().unwrap();

    let mut temp_folders = Vec::new();

    // Copy assets
    match export_options.target_platform {
        TargetPlatform::PC | TargetPlatform::WebAssembly => {
            Log::info("Trying to copy the assets...");

            for folder in export_options.assets_folders {
                Log::info(format!(
                    "Trying to copy assets from {} to {}...",
                    folder.display(),
                    export_options.destination_folder.display()
                ));

                Log::verify(asset::copy_and_convert_assets(
                    &folder,
                    export_options.destination_folder.join(&folder),
                    export_options.target_platform,
                    &|_| true,
                    &resource_manager,
                    export_options.convert_assets,
                ));
            }
        }
        TargetPlatform::Android => android::copy_assets(
            &export_options,
            package,
            package_dir_path,
            &mut temp_folders,
            &resource_manager,
            export_options.convert_assets,
        )?,
    }

    build_package(
        package_name,
        &export_options.build_target,
        package_dir_path,
        export_options.target_platform,
        cancel_flag,
        export_options.enable_optimization,
    )?;

    match export_options.target_platform {
        TargetPlatform::PC => {
            pc::copy_binaries(&metadata, package_name, &export_options.destination_folder)?
        }
        TargetPlatform::WebAssembly => wasm::copy_binaries(
            package_dir_path.as_std_path(),
            &export_options.destination_folder,
        )?,
        TargetPlatform::Android => {
            android::copy_binaries(&metadata, package_name, &export_options.destination_folder)?
        }
    }

    // Remove all temp folders.
    for temp_folder in temp_folders {
        Log::verify(std::fs::remove_dir_all(temp_folder));
    }

    let mut child_processes = Vec::new();

    if let Ok(destination_folder) = export_options.destination_folder.canonicalize() {
        if export_options.run_after_build {
            match export_options.target_platform {
                TargetPlatform::PC => pc::run_build(&destination_folder, package_name),
                TargetPlatform::WebAssembly => match wasm::run_build(&destination_folder) {
                    Ok(child_process) => {
                        child_processes.push(child_process);
                    }
                    Err(err) => {
                        err!("Unable to run build. Reason: {:?}", err);
                    }
                },
                TargetPlatform::Android => android::run_build(package_name, &destination_folder),
            }
        }

        if export_options.open_destination_folder {
            Log::verify(open::that_detached(destination_folder));
        }
    }

    Ok(BuildOutput { child_processes })
}

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct CliExportOptions {
    /// The target platform to build the game to. Must be one of: pc, android, wasm. Keep in mind,
    /// that you must also set the appropriate `build_target` parameter if you're using cross
    /// compilation (for example, creating a WebAssembly or Android build from PC).
    #[clap(long, default_value = "pc")]
    pub target_platform: String,

    /// The name of the build target.
    ///
    /// The default value forces to compile the game to the default target of the current toolchain.
    /// Usually this parameter can be left unchanged, unless you need cross-compilation to some
    /// specific platform and architecture (see below).
    ///
    /// WebAssembly builds requires this parameter to be wasm32-unknown-unknown.
    ///
    /// Android builds require one of the following: armv7-linux-androideabi for 32-bit and
    /// aarch64-linux-android for 64-bit.
    ///
    /// The full list of build targets can be found here
    /// https://doc.rust-lang.org/nightly/rustc/platform-support.html
    #[clap(long, default_value = "default")]
    pub build_target: String,

    /// The destination folder for the build.
    #[clap(long, default_value = "./build/")]
    pub destination_folder: PathBuf,

    /// A flag, that defines whether the project exporter should include only used assets in the
    /// final build or not. If specified, then this flag essentially forces the exporter to scan
    /// all the assets for cross-links and if there's at least one usage then such asset will be
    /// included in the final build. This option could be useful if your project has a lot of
    /// "dangling" resources, and you don't want to search all the used resources yourself.
    ///
    /// Use this option carefully, because it won't include assets that you manually load from code
    /// bypassing the resource manager. In this case, the project manager will simply ignore such
    /// "unknown" files.
    #[clap(long, default_value = "false")]
    pub include_used_assets: bool,

    // TODO: This is should be checked for usefulness.
    #[clap(long, default_value = "./data/")]
    pub assets_folders: Vec<PathBuf>,

    /// Specifies a set of file extensions that should be ignored. Each extension must be separated
    /// by a comma. For example: log,txt,iml
    #[clap(long, default_value = "log")]
    pub ignored_extensions: Vec<String>,

    /// If specified, the exporter will try to run the exported project after the successful build.
    #[clap(short, long, default_value = "false")]
    pub run_after_build: bool,

    /// If specified, the exporter will try to open the build folder in the default file manager
    /// of your OS after the successful build.
    #[clap(short, long, default_value = "false")]
    pub open_destination_folder: bool,

    /// If specified, the exporter will try to convert all supported assets to their "shipping"
    /// version. For example, native game scenes and UIs will be converted from ASCII to binary
    /// if this option is specified.
    #[clap(short, long, default_value = "true")]
    pub convert_assets: bool,

    /// If specified, enables all possible optimizations for the build.
    #[clap(short, long, default_value = "true")]
    pub enable_optimization: bool,
}

pub fn cli_export(resource_manager: ResourceManager) {
    let args = CliExportOptions::parse();

    let options = ExportOptions {
        target_platform: match args.target_platform.as_ref() {
            "android" => TargetPlatform::Android,
            "pc" => TargetPlatform::PC,
            "wasm" => TargetPlatform::WebAssembly,
            _ => panic!("unknown target platform {}!", args.target_platform),
        },
        destination_folder: args.destination_folder,
        include_used_assets: args.include_used_assets,
        assets_folders: args.assets_folders,
        ignored_extensions: args.ignored_extensions,
        build_target: args.build_target,
        run_after_build: args.run_after_build,
        open_destination_folder: args.open_destination_folder,
        convert_assets: args.convert_assets,
        enable_optimization: args.enable_optimization,
    };

    export(options, Default::default(), resource_manager).unwrap();
}
