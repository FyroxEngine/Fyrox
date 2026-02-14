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

//! Command line interface (CLI) for the project exporter.

use clap::Parser;
use fyrox_build_tools::export::{ExportOptions, TargetPlatform};
use fyrox_resource::{core::task::TaskPool, io::FsResourceIo, manager::ResourceManager};
use std::{path::PathBuf, sync::Arc};

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// The target platform to build the game to. Must be one of: pc, android, wasm. Keep in mind,
    /// that you must also set the appropriate `build_target` parameter if you're using cross
    /// compilation (for example, creating a WebAssembly or Android build from PC).
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

fn main() {
    let args: Args = Args::parse();

    let options = ExportOptions {
        target_platform: match args.target_platform.as_ref() {
            "android" => TargetPlatform::Android,
            "pc" => TargetPlatform::PC,
            "wasm" => TargetPlatform::WebAssembly,
            _ => panic!("unknown target platform!"),
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

    let resource_manager = ResourceManager::new(Arc::new(FsResourceIo), Arc::new(TaskPool::new()));
    fyrox_build_tools::export::export(options, Default::default(), resource_manager).unwrap();
}
