//! Fyrox Project Template Generator.

use clap::{Parser, Subcommand};
use convert_case::{Case, Casing};
use regex::Regex;
use std::{
    collections::HashMap,
    fmt::Display,
    fs::{create_dir_all, read_dir, remove_dir_all, File},
    io::{Read, Write},
    path::Path,
    process::{exit, Command},
};
use toml_edit::{table, value, DocumentMut};
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

// Ideally, this should be take from respective Cargo.toml of the engine and the editor.
// However, it does not seem to work with builds published to crates.io, because when
// the template generator is published, it does not have these Cargo.toml's available
// and to solve this we just hard code these values and pray for the best.
const CURRENT_ENGINE_VERSION: &str = "0.33.0";
const CURRENT_EDITOR_VERSION: &str = "0.20.0";
const CURRENT_SCRIPTS_VERSION: &str = "0.2.0";

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
    StartsWithNumber,
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
            Self::StartsWithNumber => write!(f, "The project name cannot start with a number"),
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
    if name.chars().next().unwrap_or(' ').is_ascii_digit() {
        return Err(NameErrors::StartsWithNumber);
    }
    Ok(name)
}

fn init_game(base_path: &Path, name: &str) {
    Command::new("cargo")
        .args(["init", "--lib", "--vcs", "none"])
        .arg(base_path.join("game"))
        .output()
        .unwrap();

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
fyrox = {{workspace = true}}

[features]
default = ["fyrox/default"]
dylib-engine = ["fyrox/dylib"]
"#,
        ),
    );

    // Write lib.rs
    write_file(
        base_path.join("game/src/lib.rs"),
        r#"//! Game project.
use fyrox::{
    core::pool::Handle, core::visitor::prelude::*,
    event::Event,
    gui::message::UiMessage,
    plugin::{Plugin, PluginContext, PluginRegistrationContext},
    scene::Scene,
};
use std::path::Path;

// Re-export the engine.
pub use fyrox;

#[derive(Default, Visit)]
pub struct Game {
    scene: Handle<Scene>,
}

impl Plugin for Game {
    fn register(&self, _context: PluginRegistrationContext) {
        // Register your scripts here.
    }
    
    fn init(&mut self, scene_path: Option<&str>, context: PluginContext) {
        context
            .async_scene_loader
            .request(scene_path.unwrap_or("data/scene.rgs"));
    }

    fn on_deinit(&mut self, _context: PluginContext) {
        // Do a cleanup here.
    }

    fn update(&mut self, _context: &mut PluginContext) {
        // Add your global update code here.
    }

    fn on_os_event(
        &mut self,
        _event: &Event<()>,
        _context: PluginContext,
    ) {
        // Do something on OS event here.
    }

    fn on_ui_message(
        &mut self,
        _context: &mut PluginContext,
        _message: &UiMessage,
    ) {
        // Handle UI events here.
    }

    fn on_scene_begin_loading(&mut self, path: &Path, ctx: &mut PluginContext) {
        if self.scene.is_some() {
            ctx.scenes.remove(self.scene);
        }
    }

    fn on_scene_loaded(
        &mut self,
        path: &Path,
        scene: Handle<Scene>,
        data: &[u8],
        context: &mut PluginContext,
    ) {
        self.scene = scene;
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
fyrox = {{ workspace = true }}
{name} = {{ path = "../game", optional = true }}

[features]
default = ["{name}"]
dylib = []
"#,
        ),
    );

    // Write main.rs
    write_file(
        base_path.join("executor/src/main.rs"),
        format!(
            r#"//! Executor with your game connected to it as a plugin.
use fyrox::engine::executor::Executor;

fn main() {{
    let mut executor = Executor::new();
   
    // Dynamic linking with hot reloading.
    #[cfg(feature = "dylib")]
    {{
        #[cfg(target_os = "windows")]
        let file_name = "game_dylib.dll";
        #[cfg(target_os = "linux")]
        let file_name = "libgame_dylib.so";
        #[cfg(target_os = "macos")]
        let file_name = "libgame_dylib.dylib";
        executor.add_dynamic_plugin(file_name, true, true).unwrap();
    }}

    // Static linking.
    #[cfg(not(feature = "dylib"))]
    {{
        use {name}::Game;
        executor.add_plugin(Game::default());
    }}  
   
    executor.run()
}}"#,
        ),
    );
}

fn init_wasm_executor(base_path: &Path, name: &str) {
    Command::new("cargo")
        .args(["init", "--lib", "--vcs", "none"])
        .arg(base_path.join("executor-wasm"))
        .output()
        .unwrap();

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
fyrox = {{workspace = true}}
{name} = {{ path = "../game" }}"#,
        ),
    );

    // Write lib.rs
    write_file(
        base_path.join("executor-wasm/src/lib.rs"),
        format!(
            r#"//! Executor with your game connected to it as a plugin.
use fyrox::engine::executor::Executor;
use {name}::Game;
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
    executor.add_plugin(Game::default());
    executor.run()
}}"#,
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
fyrox = {{ workspace = true }}
fyroxed_base = {{ workspace = true }}
{name} = {{ path = "../game", optional = true }}

[features]
default = ["{name}", "fyroxed_base/default"]
dylib = ["fyroxed_base/dylib_engine"]
"#,
        ),
    );

    write_file(
        base_path.join("editor/src/main.rs"),
        format!(
            r#"//! Editor with your game connected to it as a plugin.
use fyroxed_base::{{fyrox::event_loop::EventLoop, Editor, StartupData}};

fn main() {{
    let event_loop = EventLoop::new().unwrap();
    let mut editor = Editor::new(
        Some(StartupData {{
            working_directory: Default::default(),
            scenes: vec!["data/scene.rgs".into()],
        }}),
    );
    
     // Dynamic linking with hot reloading.
    #[cfg(feature = "dylib")]
    {{
        #[cfg(target_os = "windows")]
        let file_name = "game_dylib.dll";
        #[cfg(target_os = "linux")]
        let file_name = "libgame_dylib.so";
        #[cfg(target_os = "macos")]
        let file_name = "libgame_dylib.dylib";
        editor.add_dynamic_plugin(file_name, true, true).unwrap();
    }}

    // Static linking.
    #[cfg(not(feature = "dylib"))]
    {{
        use {name}::Game;
        editor.add_game_plugin(Game::default());
    }}
    
    editor.run(event_loop)
}}
"#,
        ),
    );
}

fn init_game_dylib(base_path: &Path, name: &str) {
    Command::new("cargo")
        .args(["init", "--lib", "--vcs", "none"])
        .arg(base_path.join("game-dylib"))
        .output()
        .unwrap();

    // Write Cargo.toml
    write_file(
        base_path.join("game-dylib/Cargo.toml"),
        format!(
            r#"
[package]
name = "game_dylib"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
{name} = {{ path = "../game", default-features = false }}

[features]
default = ["{name}/default"]
dylib-engine = ["{name}/dylib-engine"]
"#,
        ),
    );

    // Write lib.rs
    write_file(
        base_path.join("game-dylib/src/lib.rs"),
        format!(
            r#"//! Wrapper for hot-reloadable plugin.
use {name}::{{fyrox::plugin::Plugin, Game}};

#[no_mangle]
pub fn fyrox_plugin() -> Box<dyn Plugin> {{
    Box::new(Game::default())
}}
"#,
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

[package.metadata.android.sdk]
min_sdk_version = 26
target_sdk_version = 30
max_sdk_version = 29

[lib]
crate-type = ["cdylib"]

[dependencies]
fyrox = {{ workspace = true }}
{} = {{ path = "../game" }}"#,
            name,
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
use {name}::Game;

#[no_mangle]
fn android_main(app: fyrox::platform::android::activity::AndroidApp) {{
    io::ANDROID_APP
        .set(app.clone())
        .expect("ANDROID_APP cannot be set twice.");
    let event_loop = EventLoopBuilder::new().with_android_app(app).build().unwrap();
    let mut executor = Executor::from_params(event_loop, Default::default());
    executor.add_plugin(Game::default());
    executor.run()
}}"#,
        ),
    );

    write_file_binary(
        base_path.join("executor-android/README.md"),
        include_bytes!("android/README.md"),
    );
}

fn init_workspace(base_path: &Path, vcs: &str) {
    Command::new("cargo")
        .args(["init", "--vcs", vcs])
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
        format!(
            r#"
[workspace]
members = ["editor", "executor", "executor-wasm", "executor-android", "game", "game-dylib"]
resolver = "2"

[workspace.dependencies.fyrox]
version = "{CURRENT_ENGINE_VERSION}"
default-features = false
[workspace.dependencies.fyroxed_base]
version = "{CURRENT_EDITOR_VERSION}"
default-features = false

# Separate build profiles for hot reloading. These profiles ensures that build artifacts for
# hot reloading will be placed into their own folders and does not interfere with standard (static)
# linking.
[profile.dev-hot-reload]
inherits = "dev"
[profile.release-hot-reload]
inherits = "release"

# Optimize the engine in debug builds, but leave project's code non-optimized.
# By using this technique, you can still debug you code, but engine will be fully
# optimized and debug builds won't be terribly slow. With this option, you can
# compile your game in debug mode, which is much faster (at least x3), than release.
[profile.dev.package."*"]
opt-level = 3
"#,
        ),
    );

    if vcs == "git" {
        // Write .gitignore
        write_file(
            base_path.join(".gitignore"),
            r#"
/target
*.log
"#,
        );
    }
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
    let mut base_path = Path::new("game/src/");
    if !base_path.exists() {
        eprintln!("game/src directory does not exists! Fallback to root directory...");
        base_path = Path::new("");
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
    core::{{visitor::prelude::*, reflect::prelude::*, type_traits::prelude::*}},
    event::Event, script::{{ScriptContext, ScriptDeinitContext, ScriptTrait}},
}};

#[derive(Visit, Reflect, Default, Debug, Clone, TypeUuidProvider, ComponentProvider)]
#[type_uuid(id = "{id}")]
#[visit(optional)]
pub struct {name} {{
    // Add fields here.
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
        Commands::Init {
            name,
            style,
            vcs,
            overwrite,
        } => {
            let name = check_name(&name);
            let name = match name {
                Ok(s) => s,
                Err(name_error) => {
                    println!("{}", name_error);
                    return;
                }
            };

            let base_path = Path::new(name);

            // Check the path is empty / doesn't already exist (To prevent overriding)
            if !overwrite
                && base_path.exists()
                && read_dir(base_path)
                    .expect("Failed to check if path is not empty")
                    .next()
                    .is_some()
            {
                println!(
                    "Non-empty folder named {} already exists, provide --overwrite to create the project anyway",
                    base_path.display()
                );
                return;
            }

            init_workspace(base_path, &vcs);
            init_data(base_path, &style);
            init_game(base_path, name);
            init_game_dylib(base_path, name);
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
        Commands::Upgrade { version, local } => {
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

            // Engine -> (Editor, Scripts) version mapping.
            let editor_versions = [
                (
                    "0.33.0".to_string(),
                    ("0.20.0".to_string(), Some("0.2.0".to_string())),
                ),
                (
                    "0.32.0".to_string(),
                    ("0.19.0".to_string(), Some("0.1.0".to_string())),
                ),
                ("0.31.0".to_string(), ("0.18.0".to_string(), None)),
                ("0.30.0".to_string(), ("0.17.0".to_string(), None)),
                ("0.29.0".to_string(), ("0.16.0".to_string(), None)),
                ("0.28.0".to_string(), ("0.15.0".to_string(), None)),
                ("0.27.1".to_string(), ("0.14.1".to_string(), None)),
                ("0.27.0".to_string(), ("0.14.0".to_string(), None)),
                ("0.26.0".to_string(), ("0.13.0".to_string(), None)),
            ]
            .into_iter()
            .collect::<HashMap<_, _>>();

            // Open workspace manifest.
            let workspace_manifest_path = "Cargo.toml";
            if let Ok(mut file) = File::open(workspace_manifest_path) {
                let mut toml = String::new();
                if file.read_to_string(&mut toml).is_ok() {
                    drop(file);

                    if let Ok(mut document) = toml.parse::<DocumentMut>() {
                        if let Some(workspace) =
                            document.get_mut("workspace").and_then(|i| i.as_table_mut())
                        {
                            if let Some(dependencies) = workspace
                                .get_mut("dependencies")
                                .and_then(|i| i.as_table_mut())
                            {
                                if version == "latest" {
                                    if local {
                                        let mut engine_table = table();
                                        engine_table["path"] = value("../Fyrox/fyrox");
                                        dependencies["fyrox"] = engine_table;

                                        let mut editor_table = table();
                                        editor_table["path"] = value("../Fyrox/editor");
                                        dependencies["fyroxed_base"] = editor_table;

                                        if dependencies.contains_key("fyrox_scripts") {
                                            let mut scripts_table = table();
                                            scripts_table["path"] = value("../Fyrox/fyrox-scripts");
                                            dependencies["fyrox_scripts"] = scripts_table;
                                        }
                                    } else {
                                        dependencies["fyrox"] = value(CURRENT_ENGINE_VERSION);
                                        dependencies["fyroxed_base"] =
                                            value(CURRENT_EDITOR_VERSION);
                                        if dependencies.contains_key("fyrox_scripts") {
                                            dependencies["fyrox_scripts"] =
                                                value(CURRENT_SCRIPTS_VERSION);
                                        }
                                    }
                                } else if version == "nightly" {
                                    let mut table = table();
                                    table["git"] = value("https://github.com/FyroxEngine/Fyrox");

                                    dependencies["fyrox"] = table.clone();
                                    dependencies["fyroxed_base"] = table.clone();
                                } else {
                                    dependencies["fyrox"] = value(version.clone());
                                    if let Some((editor_version, scripts_version)) =
                                        editor_versions.get(&version)
                                    {
                                        dependencies["fyroxed_base"] = value(editor_version);
                                        if let Some(scripts_version) = scripts_version {
                                            if dependencies.contains_key("fyrox_scripts") {
                                                dependencies["fyrox_scripts"] =
                                                    value(scripts_version);
                                            }
                                        }
                                    } else {
                                        println!(
                                            "WARNING: matching editor/scripts version not found!"
                                        );
                                    }
                                }
                            }
                        }

                        let mut file = File::create(workspace_manifest_path).unwrap();
                        file.write_all(document.to_string().as_bytes()).unwrap();
                    }
                }
            }

            Command::new("cargo").args(["update"]).output().unwrap();

            println!("Fyrox version was successfully set to '{}'!", version);
        }
    }
}
