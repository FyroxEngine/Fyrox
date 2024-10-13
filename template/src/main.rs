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

use clap::{Parser, Subcommand};
use std::path::Path;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Initializes a new game project of given name and style.
    #[clap(arg_required_else_help = true)]
    Init {
        #[clap(short, long, default_value = "my_game")]
        name: String,

        #[clap(short, long, default_value = "3d")]
        style: String,

        #[clap(long, default_value = "git")]
        vcs: String,

        #[clap(long, default_value = "false")]
        overwrite: bool,
    },
    /// Adds a script with given name. The name will be capitalized.
    #[clap(arg_required_else_help = true)]
    Script {
        #[clap(short, long, default_value = "MyScript")]
        name: String,
    },
    /// Updates project's engine version to specified. It could be latest stable version,
    /// nightly (latest from GitHub), or specific version in 'major.minor.patch' SemVer format.
    #[clap(arg_required_else_help = true)]
    Upgrade {
        #[clap(short, long)]
        version: String,
        /// If set, specifies path to the engine to `../Fyrox/*` folder. Could be useful for development
        /// purposes. This option works only if `version` is set to `latest`.
        #[clap(long, default_value = "false")]
        local: bool,
    },
}

fn main() {
    let args: Args = Args::parse();

    match args.command {
        Commands::Init {
            name,
            style,
            vcs,
            overwrite,
        } => {
            fyrox_template_core::init_project(Path::new("./"), &name, &style, &vcs, overwrite)
                .unwrap();

            println!("Project {name} was generated successfully!");
            println!("Navigate to {name} directory and use one of the following commands:");
            println!("\tRun the Editor: cargo run --package editor --release");
            println!("\tRun the Executor: cargo run --package executor --release");
            println!(
                "\tFor WebAssembly builds - see instructions at README.md in executor-wasm folder"
            );
            println!(
                "\tFor Android builds - see instructions at README.md in executor-android folder"
            );
        }
        Commands::Script { name } => {
            fyrox_template_core::init_script(Path::new("./"), &name).unwrap();

            println!(
                "Script {name} was added successfully! Do not forget to add it to your module tree!",
            );
        }
        Commands::Upgrade { version, local } => {
            fyrox_template_core::upgrade_project(Path::new("./"), &version, local).unwrap();

            println!("Fyrox version was successfully set to '{version}'!");
        }
    }
}
