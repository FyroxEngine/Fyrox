use clap::Parser;
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
