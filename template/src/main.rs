//! Fyrox Project Template Generator.

use clap::Parser;
use std::{
    fs::{remove_dir_all, File},
    io::Write,
    path::Path,
    process::Command,
};

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long, default_value = "my_game")]
    name: String,
}

fn write_file<P: AsRef<Path>, S: AsRef<str>>(path: P, content: S) {
    let mut file = File::create(path).unwrap();
    file.write_all(content.as_ref().as_bytes()).unwrap();
}

fn init_game(base_path: &Path, args: &Args) {
    Command::new("cargo")
        .args(&["init", "--lib", "--vcs", "none"])
        .arg(base_path.join("game"))
        .output()
        .unwrap();

    // Write Cargo.toml
    write_file(
        base_path.join("game/Cargo.toml"),
        format!(
            r#"
[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[dependencies]
fyrox = "0.26""#,
            args.name,
        ),
    );

    // Write lib.rs
    write_file(
        base_path.join("game/src/lib.rs"),
        r#"//! Game project.
use fyrox::{
    core::{
        pool::Handle,
        uuid::{uuid, Uuid},
    },
    event::Event,
    plugin::{Plugin, PluginContext, PluginRegistrationContext},
    scene::{Scene, node::TypeUuidProvider},
};

pub struct Game {
    scene: Handle<Scene>,
}

impl TypeUuidProvider for Game {
    // Returns unique plugin id for serialization needs.
    fn type_uuid() -> Uuid {
        // Ideally this should be unique per-project.
        uuid!("cb358b1c-fc23-4c44-9e59-0a9671324196")
    }
}

impl Game {
    pub fn new() -> Self {
        Self {
            scene: Default::default(),
        }
    }

    fn set_scene(&mut self, scene: Handle<Scene>, _context: PluginContext) {
        self.scene = scene;

        // Do additional actions with scene here.
    }
}

impl Plugin for Game {
    fn on_register(&mut self, _context: PluginRegistrationContext) {
        // Register your scripts here.
    }

    fn on_standalone_init(&mut self, context: PluginContext) {
        self.set_scene(context.scenes.add(Scene::new()), context);
    }

    fn on_enter_play_mode(&mut self, scene: Handle<Scene>, context: PluginContext) {
        // Obtain scene from the editor.
        self.set_scene(scene, context);
    }

    fn on_leave_play_mode(&mut self, context: PluginContext) {
        self.set_scene(Handle::NONE, context)
    }

    fn update(&mut self, _context: &mut PluginContext) {
        // Add your global update code here.
    }

    fn id(&self) -> Uuid {
        Self::type_uuid()
    }

    fn on_os_event(&mut self, _event: &Event<()>, _context: PluginContext) {
        // Do something on OS event here.
    }

    fn on_unload(&mut self, _context: &mut PluginContext) {
        // Do a cleanup here.
    }
}
"#,
    );
}

fn init_executor(base_path: &Path, args: &Args) {
    Command::new("cargo")
        .args(&["init", "--bin", "--vcs", "none"])
        .arg(base_path.join("executor"))
        .output()
        .unwrap();

    // Write Cargo.toml
    write_file(
        base_path.join("executor/Cargo.toml"),
        format!(
            r#"
[package]
name = "executor"
version = "0.1.0"
edition = "2021"

[dependencies]
fyrox = "0.26"
{} = {{ path = "../game" }}"#,
            args.name,
        ),
    );

    // Write main.rs
    write_file(
        base_path.join("executor/src/main.rs"),
        format!(
            r#"//! Executor with your game connected to it as a plugin.
use fyrox::engine::executor::Executor;
use {}::Game;

fn main() {{
    let mut executor = Executor::new();
    executor.add_plugin(Game::new());
    executor.run()
}}"#,
            args.name
        ),
    );
}

fn init_editor(base_path: &Path, args: &Args) {
    Command::new("cargo")
        .args(&["init", "--bin", "--vcs", "none"])
        .arg(base_path.join("editor"))
        .output()
        .unwrap();

    // Write Cargo.toml
    write_file(
        base_path.join("editor/Cargo.toml"),
        format!(
            r#"
[package]
name = "editor"
version = "0.1.0"
edition = "2021"

[dependencies]
fyrox = "0.26"
fyroxed_base = "0.13"
{} = {{ path = "../game" }}"#,
            args.name,
        ),
    );

    write_file(
        base_path.join("editor/src/main.rs"),
        format!(
            r#"//! Editor with your game connected to it as a plugin.
use fyrox::event_loop::EventLoop;
use fyroxed_base::{{Editor, StartupData}};
use {}::Game;

fn main() {{
    let event_loop = EventLoop::new();
    let mut editor = Editor::new(
        &event_loop,
        Some(StartupData {{
            working_directory: Default::default(),
            // Set this to `"path/to/your/scene.rgs".into()` to force the editor to load the scene on startup.
            scene: Default::default(),
        }}),
    );
    editor.add_game_plugin(Game::new());
    editor.run(event_loop)
}}
"#,
            args.name
        ),
    );
}

fn init_workspace(base_path: &Path) {
    Command::new("cargo")
        .args(&["init", "--vcs", "git"])
        .arg(base_path)
        .output()
        .unwrap();

    let src_path = base_path.join("src");
    if src_path.exists() {
        remove_dir_all(src_path).unwrap();
    }

    // Write Cargo.toml
    write_file(
        base_path.join("Cargo.toml"),
        r#"
[workspace]
members = ["editor", "executor", "game"]"#,
    );
}

fn main() {
    let args = Args::parse();

    if args.name.contains('-') {
        panic!("The project name cannot contain `-`.")
    }

    let base_path = Path::new(&args.name);

    init_workspace(base_path);
    init_game(base_path, &args);
    init_editor(base_path, &args);
    init_executor(base_path, &args);

    println!("Project {} was generated successfully!", args.name);
    println!(
        "Navigate to {} directory and use one of the following commands:",
        args.name
    );
    println!("\tRun the Editor: cargo run --package editor --release");
    println!("\tRun the Executor: cargo run --package executor --release");
}
