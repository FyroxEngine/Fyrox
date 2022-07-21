//! Fyrox Project Template Generator.

use clap::Parser;
use std::fs::create_dir_all;
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

    #[clap(short, long, default_value = "3d")]
    style: String,
}

fn write_file<P: AsRef<Path>, S: AsRef<str>>(path: P, content: S) {
    let mut file = File::create(path).unwrap();
    file.write_all(content.as_ref().as_bytes()).unwrap();
}

fn write_file_binary<P: AsRef<Path>>(path: P, content: &[u8]) {
    let mut file = File::create(path).unwrap();
    file.write_all(content).unwrap();
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
fyrox = "0.27""#,
            args.name,
        ),
    );

    // Write lib.rs
    write_file(
        base_path.join("game/src/lib.rs"),
        r#"//! Game project.
use fyrox::{
    core::{
        futures::executor::block_on,
        pool::Handle,
        uuid::{uuid, Uuid},
    },
    event::Event,
    event_loop::ControlFlow,
    gui::message::UiMessage,
    plugin::{Plugin, PluginConstructor, PluginContext, PluginRegistrationContext},
    scene::{node::TypeUuidProvider, Scene, SceneLoader},
};

pub struct GameConstructor;

impl TypeUuidProvider for GameConstructor {
    fn type_uuid() -> Uuid {
        uuid!("f615ac42-b259-4a23-bb44-407d753ac178")
    }
}

impl PluginConstructor for GameConstructor {
    fn register(&self, _context: PluginRegistrationContext) {
        // Register your scripts here.
    }

    fn create_instance(
        &self,
        override_scene: Handle<Scene>,
        context: PluginContext,
    ) -> Box<dyn Plugin> {
        Box::new(Game::new(override_scene, context))
    }
}

pub struct Game {
    scene: Handle<Scene>,
}

impl Game {
    pub fn new(override_scene: Handle<Scene>, context: PluginContext) -> Self {
        let scene = if override_scene.is_some() {
            override_scene
        } else {
            // Load a scene from file if there is no override scene specified.
            let scene = block_on(
                block_on(SceneLoader::from_file(
                    "data/scene.rgs",
                    context.serialization_context.clone(),
                ))
                .unwrap()
                .finish(context.resource_manager.clone()),
            );

            context.scenes.add(scene)
        };

        Self { scene }
    }
}

impl Plugin for Game {
    fn on_deinit(&mut self, _context: PluginContext) {
        // Do a cleanup here.
    }

    fn update(&mut self, _context: &mut PluginContext, _control_flow: &mut ControlFlow) {
        // Add your global update code here.
    }

    fn id(&self) -> Uuid {
        GameConstructor::type_uuid()
    }

    fn on_os_event(
        &mut self,
        _event: &Event<()>,
        _context: PluginContext,
        _control_flow: &mut ControlFlow,
    ) {
        // Do something on OS event here.
    }

    fn on_ui_message(
        &mut self,
        _context: &mut PluginContext,
        _message: &UiMessage,
        _control_flow: &mut ControlFlow,
    ) {
        // Handle UI events here.
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
fyrox = "0.27"
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
use {}::GameConstructor;

fn main() {{
    let mut executor = Executor::new();
    executor.add_plugin_constructor(GameConstructor);
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
fyrox = "0.27"
fyroxed_base = "0.14"
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
use {}::GameConstructor;

fn main() {{
    let event_loop = EventLoop::new();
    let mut editor = Editor::new(
        &event_loop,
        Some(StartupData {{
            working_directory: Default::default(),
            scene: "data/scene.rgs".into(),
        }}),
    );
    editor.add_game_plugin(GameConstructor);
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

fn init_data(base_path: &Path, style: &str) {
    let data_path = base_path.join("data");
    create_dir_all(&data_path).unwrap();

    let scene_path = data_path.join("scene.rgs");
    match style {
        "2d" => write_file_binary(scene_path, include_bytes!("../2d.rgs")),
        "3d" => write_file_binary(scene_path, include_bytes!("../3d.rgs")),
        _ => println!("Unknown style: {}. Use either `2d` or `3d`", style),
    }
}

fn main() {
    let args = Args::parse();

    if args.name.contains('-') {
        panic!("The project name cannot contain `-`.")
    }

    let base_path = Path::new(&args.name);

    init_workspace(base_path);
    init_data(base_path, &args.style);
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
