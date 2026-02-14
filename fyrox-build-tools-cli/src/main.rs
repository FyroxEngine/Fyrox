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

//! Fyrox Project Template Generator command line interface.

use clap::Parser;
use fyrox_build_tools::export::{ExportOptions, TargetPlatform};
use fyrox_resource::{core::task::TaskPool, io::FsResourceIo, manager::ResourceManager};
use std::{path::PathBuf, sync::Arc};

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    pub target_platform: String,
    pub destination_folder: PathBuf,
    pub include_used_assets: bool,
    pub assets_folders: Vec<PathBuf>,
    pub ignored_extensions: Vec<String>,
    pub build_target: String,
    pub run_after_build: bool,
    pub open_destination_folder: bool,
    pub convert_assets: bool,
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
