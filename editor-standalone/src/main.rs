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

use clap::Parser;
use fyrox::core::log::Log;
use fyrox::event_loop::EventLoop;
use fyroxed_base::{Editor, StartupData};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Project root directory
    #[arg(short, long)]
    project_directory: Option<String>,

    /// List of scenes to load
    #[arg(short, long)]
    scenes: Option<Vec<String>>,
}

fn main() {
    Log::set_file_name("fyrox.log");

    let args = Args::parse();
    let startup_data = if let Some(proj_dir) = args.project_directory {
        Some(StartupData {
            working_directory: proj_dir.into(),
            scenes: args
                .scenes
                .unwrap_or_default()
                .iter()
                .map(Into::into)
                .collect(),
        })
    } else {
        None
    };

    Editor::new(startup_data).run(EventLoop::new().unwrap())
}
