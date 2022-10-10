//! Fyrox Project Template Generator.

use clap::{Parser, Subcommand};
use convert_case::{Case, Casing};
use std::{
    fs::{create_dir_all, remove_dir_all, File},
    io::Write,
    path::Path,
    process::Command,
};
use uuid::Uuid;

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
    },
    /// Adds a script with given name. The name will be capitalized.
    #[clap(arg_required_else_help = true)]
    Script {
        #[clap(short, long, default_value = "MyScript")]
        name: String,
    },
}

fn write_file<P: AsRef<Path>, S: AsRef<str>>(path: P, content: S) {
    let mut file = File::create(path).unwrap();
    file.write_all(content.as_ref().as_bytes()).unwrap();
}

fn write_file_binary<P: AsRef<Path>>(path: P, content: &[u8]) {
    let mut file = File::create(path).unwrap();
    file.write_all(content).unwrap();
}

fn init_game(base_path: &Path, name: &str) {
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
fyrox = "0.28""#,
            name,
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
    },
    event::Event,
    event_loop::ControlFlow,
    gui::message::UiMessage,
    plugin::{Plugin, PluginConstructor, PluginContext, PluginRegistrationContext},
    scene::{Scene, SceneLoader},
};

pub struct GameConstructor;

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

fn init_executor(base_path: &Path, name: &str) {
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
fyrox = "0.28"
{} = {{ path = "../game" }}"#,
            name,
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
            name
        ),
    );
}

fn init_editor(base_path: &Path, name: &str) {
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
fyrox = "0.28"
fyroxed_base = "0.15"
{} = {{ path = "../game" }}"#,
            name,
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
            name
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
members = ["editor", "executor", "game"]

# Optimize the engine in debug builds, but leave project's code non-optimized.
# By using this technique, you can still debug you code, but engine will be fully
# optimized and debug builds won't be terribly slow. With this option, you can 
# compile your game in debug mode, which is much faster (at least x3), than release.
[profile.dev.package."*"]
opt-level = 3
"#,
    );
}

fn init_data(base_path: &Path, style: &str) {
    let data_path = base_path.join("data");
    create_dir_all(&data_path).unwrap();

    let scene_path = data_path.join("scene.rgs");
    match style {
        "2d" => write_file_binary(scene_path, include_bytes!("2d.rgs")),
        "3d" => write_file_binary(scene_path, include_bytes!("3d.rgs")),
        _ => println!("Unknown style: {}. Use either `2d` or `3d`", style),
    }
}

fn init_script(raw_name: &str) {
    let base_path = Path::new("game/src/");
    if !base_path.exists() {
        panic!("game/src directory does not exists!")
    }

    let script_file_stem = raw_name.to_case(Case::Snake);
    let script_name = raw_name.to_case(Case::UpperCamel);
    let file_name = base_path.join(script_file_stem + ".rs");

    if file_name.exists() {
        panic!("Script {} already exists!", script_name);
    }

    let script_uuid = Uuid::new_v4().to_string();

    write_file(
        file_name,
        format!(
            r#"
use fyrox::{{
    core::{{inspect::prelude::*, uuid::{{Uuid, uuid}}, visitor::prelude::*, reflect::Reflect}},
    engine::resource_manager::ResourceManager,
    event::Event, impl_component_provider,
    scene::{{node::TypeUuidProvider}},
    script::{{ScriptContext, ScriptDeinitContext, ScriptTrait}},
}};

#[derive(Visit, Reflect, Inspect, Default, Debug, Clone)]
pub struct {name} {{
    // Add fields here.
}}

impl_component_provider!({name});

impl TypeUuidProvider for {name} {{
    fn type_uuid() -> Uuid {{
        uuid!("{id}")
    }}
}}

impl ScriptTrait for {name} {{
    fn on_init(&mut self, context: &mut ScriptContext) {{
        // Put initialization logic here.
    }}

    fn on_start(&mut self, context: &mut ScriptContext) {{
        // There should be a logic that depends on other scripts in scene.
        // It is called right after **all** scripts were initialized.
    }}

    fn on_deinit(&mut self, context: &mut ScriptDeinitContext) {{
        // Put de-initialization logic here.
    }}

    fn on_os_event(&mut self, event: &Event<()>, context: &mut ScriptContext) {{
        // Respond to OS events here.
    }}

    fn on_update(&mut self, context: &mut ScriptContext) {{
        // Put object logic here.
    }}

    fn restore_resources(&mut self, resource_manager: ResourceManager) {{
        // Restore resource handles here.
    }}

    fn id(&self) -> Uuid {{
        Self::type_uuid()
    }}
}}
    "#,
            name = script_name,
            id = script_uuid
        ),
    );

    println!(
        "Script {} was added successfully! Do not forget to add it to your module tree!",
        script_name
    );
}

fn main() {
    let args: Args = Args::parse();

    match args.command {
        Commands::Init { name, style } => {
            if name.contains('-') {
                panic!("The project name cannot contain `-`.")
            }

            let base_path = Path::new(&name);

            init_workspace(base_path);
            init_data(base_path, &style);
            init_game(base_path, &name);
            init_editor(base_path, &name);
            init_executor(base_path, &name);

            println!("Project {} was generated successfully!", name);
            println!(
                "Navigate to {} directory and use one of the following commands:",
                name
            );
            println!("\tRun the Editor: cargo run --package editor --release");
            println!("\tRun the Executor: cargo run --package executor --release");
        }
        Commands::Script { name } => {
            init_script(&name);
        }
    }
}
