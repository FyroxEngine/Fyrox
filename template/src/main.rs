//! Fyrox Project Template Generator.

use clap::{Parser, Subcommand};
use convert_case::{Case, Casing};
use regex::Regex;
use std::{
    collections::HashMap,
    fmt::Display,
    fs::{create_dir_all, remove_dir_all, File},
    io::{Read, Write},
    path::Path,
    process::{exit, Command},
};
use toml_edit::{table, value, Document};
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
    /// Updates project's engine version to specified. It could be latest stable version,
    /// nightly (latest from GitHub), or specific version in 'major.minor.patch' SemVer format.
    #[clap(arg_required_else_help = true)]
    Upgrade {
        #[clap(short, long)]
        version: String,
    },
}

// Ideally, this should be take from respective Cargo.toml of the engine and the editor.
// However, it does not seem to work with builds published to crates.io, because when
// the template generator is published, it does not have these Cargo.toml's available
// and to solve this we just hard code these values and pray for the best.
const CURRENT_ENGINE_VERSION: &str = "0.31.0";
const CURRENT_EDITOR_VERSION: &str = "0.18.0";

fn write_file<P: AsRef<Path>, S: AsRef<str>>(path: P, content: S) {
    let mut file = File::create(path.as_ref()).unwrap();
    file.write_all(content.as_ref().as_bytes())
        .unwrap_or_else(|x| {
            panic!(
                "Error happened while writing to file: {}.\nError:\n{}",
                path.as_ref().to_string_lossy(),
                x
            )
        });
}

fn write_file_binary<P: AsRef<Path>>(path: P, content: &[u8]) {
    let mut file = File::create(path.as_ref()).unwrap();
    file.write_all(content).unwrap_or_else(|x| {
        panic!(
            "Error happened while writing to file: {}.\nError:\n{}",
            path.as_ref().to_string_lossy(),
            x
        )
    });
}

#[derive(Debug)]
enum NameErrors {
    CargoReserved(String),
    Hyphen,
}

impl Display for NameErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CargoReserved(name) => write!(
                f,
                "The project name cannot be `{}` due to cargo's reserved keywords",
                name
            ),
            Self::Hyphen => write!(f, "The project name cannot contain `-`"),
        }
    }
}

fn check_name(name: &str) -> Result<&str, NameErrors> {
    const RESERVED_NAMES: [&str; 53] = [
        "abstract", "alignof", "as", "become", "box", "break", "const", "continue", "crate", "do",
        "else", "enum", "extern", "false", "final", "fn", "for", "if", "impl", "in", "let", "loop",
        "macro", "match", "mod", "move", "mut", "offsetof", "override", "priv", "proc", "pub",
        "pure", "ref", "return", "self", "sizeof", "static", "struct", "super", "test", "trait",
        "true", "type", "typeof", "try", "unsafe", "unsized", "use", "virtual", "where", "while",
        "yield",
    ];
    if RESERVED_NAMES.contains(&name) {
        return Err(NameErrors::CargoReserved(name.to_string()));
    }
    if name.contains('-') {
        return Err(NameErrors::Hyphen);
    }
    Ok(name)
}

fn init_game(base_path: &Path, name: &str) {
    Command::new("cargo")
        .args(["init", "--lib", "--vcs", "none"])
        .arg(base_path.join("game"))
        .output()
        .unwrap();

    let engine_version = CURRENT_ENGINE_VERSION;

    // Write Cargo.toml
    write_file(
        base_path.join("game/Cargo.toml"),
        format!(
            r#"
[package]
name = "{name}"
version = "0.1.0"
edition = "2021"

[dependencies]
fyrox = "{engine_version}""#,
        ),
    );

    // Write lib.rs
    write_file(
        base_path.join("game/src/lib.rs"),
        r#"//! Game project.
use fyrox::{
    core::pool::Handle,
    event::Event,
    event_loop::ControlFlow,
    gui::message::UiMessage,
    plugin::{Plugin, PluginConstructor, PluginContext, PluginRegistrationContext},
    scene::{Scene, loader::AsyncSceneLoader},
    core::log::Log
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
    loader: Option<AsyncSceneLoader>,
}

impl Game {
    pub fn new(override_scene: Handle<Scene>, context: PluginContext) -> Self {
        let mut loader = None;
        let scene = if override_scene.is_some() {
            override_scene
        } else {
            loader = Some(AsyncSceneLoader::begin_loading(
                "data/scene.rgs".into(),
                context.serialization_context.clone(),
                context.resource_manager.clone(),
            ));
            Default::default()
        };

        Self { scene, loader }
    }
}

impl Plugin for Game {
    fn on_deinit(&mut self, _context: PluginContext) {
        // Do a cleanup here.
    }

    fn update(&mut self, context: &mut PluginContext, _control_flow: &mut ControlFlow) {
         if let Some(loader) = self.loader.as_ref() {
            if let Some(result) = loader.fetch_result() {
                match result {
                    Ok(scene) => {
                        self.scene = context.scenes.add(scene);
                    }
                    Err(err) => Log::err(err),
                }
            }
        }

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
        .args(["init", "--bin", "--vcs", "none"])
        .arg(base_path.join("executor"))
        .output()
        .unwrap();

    let engine_version = CURRENT_ENGINE_VERSION;

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
fyrox = "{engine_version}"
{name} = {{ path = "../game" }}"#,
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

fn init_wasm_executor(base_path: &Path, name: &str) {
    Command::new("cargo")
        .args(["init", "--lib", "--vcs", "none"])
        .arg(base_path.join("executor-wasm"))
        .output()
        .unwrap();

    let engine_version = CURRENT_ENGINE_VERSION;

    // Write Cargo.toml
    write_file(
        base_path.join("executor-wasm/Cargo.toml"),
        format!(
            r#"
[package]
name = "executor-wasm"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
fyrox = "{engine_version}"
{name} = {{ path = "../game" }}"#,
        ),
    );

    // Write lib.rs
    write_file(
        base_path.join("executor-wasm/src/lib.rs"),
        format!(
            r#"//! Executor with your game connected to it as a plugin.
use fyrox::engine::executor::Executor;
use {}::GameConstructor;
use fyrox::core::wasm_bindgen::{{self, prelude::*}};

#[wasm_bindgen]
extern "C" {{
    #[wasm_bindgen(js_namespace = console)]
    fn error(msg: String);

    type Error;

    #[wasm_bindgen(constructor)]
    fn new() -> Error;

    #[wasm_bindgen(structural, method, getter)]
    fn stack(error: &Error) -> String;
}}

fn custom_panic_hook(info: &std::panic::PanicInfo) {{
    let mut msg = info.to_string();
    msg.push_str("\n\nStack:\n\n");
    let e = Error::new();
    let stack = e.stack();
    msg.push_str(&stack);
    msg.push_str("\n\n");
    error(msg);
}}

#[inline]
pub fn set_panic_hook() {{
    use std::sync::Once;
    static SET_HOOK: Once = Once::new();
    SET_HOOK.call_once(|| {{
        std::panic::set_hook(Box::new(custom_panic_hook));
    }});
}}

#[wasm_bindgen]
pub fn main() {{
    set_panic_hook();
    let mut executor = Executor::new();
    executor.add_plugin_constructor(GameConstructor);
    executor.run()
}}"#,
            name
        ),
    );

    // Write "entry" point stuff. This includes:
    //
    // - Index page with a "Start" button. The button is needed to solve sound issues in some browsers.
    //   Some browsers (mostly Chrome) prevent sound from playing until user click on something on the
    //   game page.
    // - Entry JavaScript code - basically a web launcher for your game.
    // - Styles - to make "Start" button to look decent.
    // - A readme file with build instructions.
    write_file_binary(
        base_path.join("executor-wasm/index.html"),
        include_bytes!("wasm/index.html"),
    );
    write_file_binary(
        base_path.join("executor-wasm/styles.css"),
        include_bytes!("wasm/styles.css"),
    );
    write_file_binary(
        base_path.join("executor-wasm/main.js"),
        include_bytes!("wasm/main.js"),
    );
    write_file_binary(
        base_path.join("executor-wasm/README.md"),
        include_bytes!("wasm/README.md"),
    );
}

fn init_editor(base_path: &Path, name: &str) {
    Command::new("cargo")
        .args(["init", "--bin", "--vcs", "none"])
        .arg(base_path.join("editor"))
        .output()
        .unwrap();

    let engine_version = CURRENT_ENGINE_VERSION;
    let editor_version = CURRENT_EDITOR_VERSION;

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
fyrox = "{engine_version}"
fyroxed_base = "{editor_version}"
{name} = {{ path = "../game" }}"#,
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

fn init_android_executor(base_path: &Path, name: &str) {
    Command::new("cargo")
        .args(["init", "--lib", "--vcs", "none"])
        .arg(base_path.join("executor-android"))
        .output()
        .unwrap();

    // Write Cargo.toml
    write_file(
        base_path.join("executor-android/Cargo.toml"),
        format!(
            r#"
[package]
name = "executor-android"
version = "0.1.0"
edition = "2021"

[package.metadata.android]
assets = "../data"
strip = "strip"

[lib]
crate-type = ["cdylib"]

[dependencies]
fyrox = "{}"
{} = {{ path = "../game" }}"#,
            CURRENT_ENGINE_VERSION, name,
        ),
    );

    // Write main.rs
    write_file(
        base_path.join("executor-android/src/lib.rs"),
        format!(
            r#"//! Android executor with your game connected to it as a plugin.
use fyrox::{{
    core::io, engine::executor::Executor, event_loop::EventLoopBuilder,
    platform::android::EventLoopBuilderExtAndroid,
}};
use {}::GameConstructor;

#[no_mangle]
fn android_main(app: fyrox::platform::android::activity::AndroidApp) {{
    io::ANDROID_APP
        .set(app.clone())
        .expect("ANDROID_APP cannot be set twice.");
    let event_loop = EventLoopBuilder::new().with_android_app(app).build();
    let mut executor = Executor::from_params(event_loop, Default::default());
    executor.add_plugin_constructor(GameConstructor);
    executor.run()
}}"#,
            name
        ),
    );

    write_file_binary(
        base_path.join("executor-android/README.md"),
        include_bytes!("android/README.md"),
    );
}

fn init_workspace(base_path: &Path) {
    Command::new("cargo")
        .args(["init", "--vcs", "git"])
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
members = ["editor", "executor", "executor-wasm", "executor-android", "game"]

# Optimize the engine in debug builds, but leave project's code non-optimized.
# By using this technique, you can still debug you code, but engine will be fully
# optimized and debug builds won't be terribly slow. With this option, you can
# compile your game in debug mode, which is much faster (at least x3), than release.
[profile.dev.package."*"]
opt-level = 3
"#,
    );

    // Write .gitignore
    write_file(
        base_path.join(".gitignore"),
        r#"
/target
*.log
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
    let file_name = base_path.join(script_file_stem.clone() + ".rs");

    if file_name.exists() {
        panic!("Script {} already exists!", script_name);
    }

    let script_uuid = Uuid::new_v4().to_string();

    write_file(
        file_name,
        format!(
            r#"
use fyrox::{{
    core::{{uuid::{{Uuid, uuid}}, visitor::prelude::*, reflect::prelude::*, TypeUuidProvider}},
    event::Event, impl_component_provider,
    script::{{ScriptContext, ScriptDeinitContext, ScriptTrait}},
}};

#[derive(Visit, Reflect, Default, Debug, Clone)]
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
        "Script {script_name} was added successfully! Do not forget to add it to your module tree by \
        adding:\n\tpub mod {script_file_stem};\nat either lib.rs or some other module.",
    );
}

fn main() {
    let args: Args = Args::parse();

    match args.command {
        Commands::Init { name, style } => {
            let name = check_name(&name);
            let name = match name {
                Ok(s) => s,
                Err(name_error) => {
                    println!("{}", name_error);
                    return;
                }
            };

            let base_path = Path::new(name);

            init_workspace(base_path);
            init_data(base_path, &style);
            init_game(base_path, name);
            init_editor(base_path, name);
            init_executor(base_path, name);
            init_wasm_executor(base_path, name);
            init_android_executor(base_path, name);

            println!("Project {} was generated successfully!", name);
            println!(
                "Navigate to {} directory and use one of the following commands:",
                name
            );
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
            init_script(&name);
        }
        Commands::Upgrade { version } => {
            let engine_version = CURRENT_ENGINE_VERSION;
            let editor_version = CURRENT_EDITOR_VERSION;

            let semver_regex = Regex::new(include_str!("regex")).unwrap();

            if version != "latest" && version != "nightly" && !semver_regex.is_match(&version) {
                println!(
                        "Invalid version: {version}. Please specify one of the following:\n\
                    \tnightly - uses latest nightly version of the engine from GitHub directly.\
                    \tlatest - uses latest stable version of the engine.\n\
                    \tmajor.minor.patch - uses specific stable version from crates.io (0.30.0 for example).",
                    );
                exit(1);
            }

            // Engine -> Editor version mapping.
            let editor_versions = [
                ("0.31.0".to_string(), "0.18.0".to_string()),
                ("0.30.0".to_string(), "0.17.0".to_string()),
                ("0.29.0".to_string(), "0.16.0".to_string()),
                ("0.28.0".to_string(), "0.15.0".to_string()),
                ("0.27.1".to_string(), "0.14.1".to_string()),
                ("0.27.0".to_string(), "0.14.0".to_string()),
                ("0.26.0".to_string(), "0.13.0".to_string()),
            ]
            .into_iter()
            .collect::<HashMap<String, String>>();

            for (path, is_editor) in [
                ("game/Cargo.toml", false),
                ("editor/Cargo.toml", true),
                ("executor/Cargo.toml", false),
                ("executor-android/Cargo.toml", false),
                ("executor-wasm/Cargo.toml", false),
            ] {
                // Some executors might be missing if user decided to not use them, so here we check if
                // manifest exists.
                if let Ok(mut file) = File::open(path) {
                    let mut toml = String::new();
                    if file.read_to_string(&mut toml).is_ok() {
                        drop(file);

                        if let Ok(mut document) = toml.parse::<Document>() {
                            if let Some(dependencies) = document
                                .get_mut("dependencies")
                                .and_then(|i| i.as_table_mut())
                            {
                                if version == "latest" {
                                    dependencies["fyrox"] = value(engine_version);
                                    if is_editor {
                                        dependencies["fyroxed_base"] = value(editor_version);
                                    }
                                } else if version == "nightly" {
                                    let mut table = table();
                                    table["git"] = value("https://github.com/FyroxEngine/Fyrox");

                                    dependencies["fyrox"] = table.clone();
                                    if is_editor {
                                        dependencies["fyroxed_base"] = table;
                                    }
                                } else {
                                    dependencies["fyrox"] = value(version.clone());
                                    if is_editor {
                                        if let Some(editor_version) = editor_versions.get(&version)
                                        {
                                            dependencies["fyroxed_base"] = value(editor_version);
                                        } else {
                                            println!("WARNING: matching editor version not found!");
                                        }
                                    }
                                }
                            }

                            let mut file = File::create(path).unwrap();
                            file.write_all(document.to_string().as_bytes()).unwrap();
                        }
                    }
                }
            }

            Command::new("cargo").args(["update"]).output().unwrap();

            println!("Fyrox version was successfully set to '{}'!", version);
        }
    }
}
