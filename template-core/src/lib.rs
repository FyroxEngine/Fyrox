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

//! Fyrox Project Template Generator.

use convert_case::{Case, Casing};
use regex::Regex;
use std::{
    collections::HashMap,
    fmt::Display,
    fs::{create_dir_all, read_dir, remove_dir_all, File},
    io::{Read, Write},
    path::Path,
    process::Command,
};
use toml_edit::{table, value, DocumentMut};
use uuid::Uuid;

// Ideally, this should be take from respective Cargo.toml of the engine and the editor.
// However, it does not seem to work with builds published to crates.io, because when
// the template generator is published, it does not have these Cargo.toml's available
// and to solve this we just hard code these values and pray for the best.
const CURRENT_ENGINE_VERSION: &str = "0.34.0";
const CURRENT_EDITOR_VERSION: &str = "0.21.0";
const CURRENT_SCRIPTS_VERSION: &str = "0.3.0";

fn write_file<P: AsRef<Path>, S: AsRef<str>>(path: P, content: S) -> Result<(), String> {
    let mut file = File::create(path.as_ref()).map_err(|e| e.to_string())?;
    file.write_all(content.as_ref().as_bytes()).map_err(|x| {
        format!(
            "Error happened while writing to file: {}.\nError:\n{}",
            path.as_ref().to_string_lossy(),
            x
        )
    })
}

fn write_file_binary<P: AsRef<Path>>(path: P, content: &[u8]) -> Result<(), String> {
    let mut file = File::create(path.as_ref()).map_err(|e| e.to_string())?;
    file.write_all(content).map_err(|x| {
        format!(
            "Error happened while writing to file: {}.\nError:\n{}",
            path.as_ref().to_string_lossy(),
            x
        )
    })
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
                "The project name cannot be `{name}` due to cargo's reserved keywords"
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

fn init_game(base_path: &Path, name: &str) -> Result<(), String> {
    Command::new("cargo")
        .args(["init", "--lib", "--vcs", "none"])
        .arg(base_path.join("game"))
        .output()
        .map_err(|e| e.to_string())?;

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
    )?;

    // Write lib.rs
    write_file(
        base_path.join("game/src/lib.rs"),
        r#"//! Game project.
use fyrox::{
    core::pool::Handle, core::visitor::prelude::*, core::reflect::prelude::*,
    event::Event,
    gui::message::UiMessage,
    plugin::{Plugin, PluginContext, PluginRegistrationContext},
    scene::Scene,
};
use std::path::Path;

// Re-export the engine.
pub use fyrox;

#[derive(Default, Visit, Reflect, Debug)]
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

    fn on_scene_begin_loading(&mut self, _path: &Path, ctx: &mut PluginContext) {
        if self.scene.is_some() {
            ctx.scenes.remove(self.scene);
        }
    }

    fn on_scene_loaded(
        &mut self,
        _path: &Path,
        scene: Handle<Scene>,
        _data: &[u8],
        _context: &mut PluginContext,
    ) {
        self.scene = scene;
    }
}
"#,
    )
}

fn init_executor(base_path: &Path, name: &str) -> Result<(), String> {
    Command::new("cargo")
        .args(["init", "--bin", "--vcs", "none"])
        .arg(base_path.join("executor"))
        .output()
        .map_err(|e| e.to_string())?;

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
dylib = ["fyrox/dylib"]
"#,
        ),
    )?;

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
    )
}

fn init_wasm_executor(base_path: &Path, name: &str) -> Result<(), String> {
    Command::new("cargo")
        .args(["init", "--lib", "--vcs", "none"])
        .arg(base_path.join("executor-wasm"))
        .output()
        .map_err(|e| e.to_string())?;

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
    )?;

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
    )?;

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
    )?;
    write_file_binary(
        base_path.join("executor-wasm/styles.css"),
        include_bytes!("wasm/styles.css"),
    )?;
    write_file_binary(
        base_path.join("executor-wasm/main.js"),
        include_bytes!("wasm/main.js"),
    )?;
    write_file_binary(
        base_path.join("executor-wasm/README.md"),
        include_bytes!("wasm/README.md"),
    )
}

fn init_editor(base_path: &Path, name: &str) -> Result<(), String> {
    Command::new("cargo")
        .args(["init", "--bin", "--vcs", "none"])
        .arg(base_path.join("editor"))
        .output()
        .map_err(|e| e.to_string())?;

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
    )?;

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
    )
}

fn init_game_dylib(base_path: &Path, name: &str) -> Result<(), String> {
    Command::new("cargo")
        .args(["init", "--lib", "--vcs", "none"])
        .arg(base_path.join("game-dylib"))
        .output()
        .map_err(|e| e.to_string())?;

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
    )?;

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
    )
}

fn init_android_executor(base_path: &Path, name: &str) -> Result<(), String> {
    Command::new("cargo")
        .args(["init", "--lib", "--vcs", "none"])
        .arg(base_path.join("executor-android"))
        .output()
        .map_err(|e| e.to_string())?;

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
# This folder is used as a temporary storage for assets. Project exporter will clone everything 
# from data folder to this folder and cargo-apk will create the apk with these assets.
assets = "assets"
strip = "strip"

[package.metadata.android.sdk]
min_sdk_version = 26
target_sdk_version = 30
max_sdk_version = 29

[package.metadata.android.signing.release]
path = "release.keystore"
keystore_password = "fyrox-template"

[lib]
crate-type = ["cdylib"]

[dependencies]
fyrox = {{ workspace = true }}
{name} = {{ path = "../game" }}"#,
        ),
    )?;

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
    )?;

    write_file_binary(
        base_path.join("executor-android/README.md"),
        include_bytes!("android/README.md"),
    )?;
    write_file_binary(
        base_path.join("executor-android/release.keystore"),
        include_bytes!("android/release.keystore"),
    )?;
    create_dir_all(base_path.join("executor-android/assets")).map_err(|e| e.to_string())
}

fn init_workspace(base_path: &Path, vcs: &str) -> Result<(), String> {
    Command::new("cargo")
        .args(["init", "--vcs", vcs])
        .arg(base_path)
        .output()
        .map_err(|e| e.to_string())?;

    let src_path = base_path.join("src");
    if src_path.exists() {
        remove_dir_all(src_path).map_err(|e| e.to_string())?;
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
    )?;

    if vcs == "git" {
        // Write .gitignore
        write_file(
            base_path.join(".gitignore"),
            r#"
/target
*.log
"#,
        )?;
    }

    Ok(())
}

fn init_data(base_path: &Path, style: &str) -> Result<(), String> {
    let data_path = base_path.join("data");
    create_dir_all(&data_path).map_err(|e| e.to_string())?;

    let scene_path = data_path.join("scene.rgs");
    match style {
        "2d" => write_file_binary(scene_path, include_bytes!("2d.rgs")),
        "3d" => write_file_binary(scene_path, include_bytes!("3d.rgs")),
        _ => Err(format!("Unknown style: {style}. Use either `2d` or `3d`")),
    }
}

pub fn init_script(root_path: &Path, raw_name: &str) -> Result<(), String> {
    let mut base_path = root_path.join("game/src/");
    if !base_path.exists() {
        eprintln!("game/src directory does not exists! Fallback to root directory...");
        base_path = root_path.to_path_buf();
    }

    let script_file_stem = raw_name.to_case(Case::Snake);
    let script_name = raw_name.to_case(Case::UpperCamel);
    let file_name = base_path.join(script_file_stem.clone() + ".rs");

    if file_name.exists() {
        return Err(format!("Script {script_name} already exists!"));
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
#[type_uuid(id = "{script_uuid}")]
#[visit(optional)]
pub struct {script_name} {{
    // Add fields here.
}}

impl ScriptTrait for {script_name} {{
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
    "#
        ),
    )
}

pub fn init_project(
    root_path: &Path,
    name: &str,
    style: &str,
    vcs: &str,
    overwrite: bool,
) -> Result<(), String> {
    let name = check_name(name);
    let name = match name {
        Ok(s) => s,
        Err(name_error) => {
            println!("{name_error}");
            return Err(name_error.to_string());
        }
    };

    let base_path = root_path.join(name);
    let base_path = &base_path;

    // Check the path is empty / doesn't already exist (To prevent overriding)
    if !overwrite
        && base_path.exists()
        && read_dir(base_path)
            .expect("Failed to check if path is not empty")
            .next()
            .is_some()
    {
        return Err(format!(
            "Non-empty folder named {} already exists, provide --overwrite to create the project anyway",
            base_path.display()
        ));
    }

    init_workspace(base_path, vcs)?;
    init_data(base_path, style)?;
    init_game(base_path, name)?;
    init_game_dylib(base_path, name)?;
    init_editor(base_path, name)?;
    init_executor(base_path, name)?;
    init_wasm_executor(base_path, name)?;
    init_android_executor(base_path, name)
}

pub fn upgrade_project(root_path: &Path, version: &str, local: bool) -> Result<(), String> {
    let semver_regex = Regex::new(include_str!("regex")).map_err(|e| e.to_string())?;

    if version != "latest" && version != "nightly" && !semver_regex.is_match(version) {
        return Err(format!(
            "Invalid version: {version}. Please specify one of the following:\n\
                    \tnightly - uses latest nightly version of the engine from GitHub directly.\
                    \tlatest - uses latest stable version of the engine.\n\
                    \tmajor.minor.patch - uses specific stable version from crates.io (0.30.0 for example).",
        ));
    }

    // Engine -> (Editor, Scripts) version mapping.
    let editor_versions = [
        (
            "0.34.0".to_string(),
            ("0.21.0".to_string(), Some("0.3.0".to_string())),
        ),
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
    let workspace_manifest_path = root_path.join("Cargo.toml");
    if let Ok(mut file) = File::open(&workspace_manifest_path) {
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
                                dependencies["fyroxed_base"] = value(CURRENT_EDITOR_VERSION);
                                if dependencies.contains_key("fyrox_scripts") {
                                    dependencies["fyrox_scripts"] = value(CURRENT_SCRIPTS_VERSION);
                                }
                            }
                        } else if version == "nightly" {
                            let mut table = table();
                            table["git"] = value("https://github.com/FyroxEngine/Fyrox");

                            dependencies["fyrox"] = table.clone();
                            dependencies["fyroxed_base"] = table.clone();
                        } else {
                            dependencies["fyrox"] = value(version);
                            if let Some((editor_version, scripts_version)) =
                                editor_versions.get(version)
                            {
                                dependencies["fyroxed_base"] = value(editor_version);
                                if let Some(scripts_version) = scripts_version {
                                    if dependencies.contains_key("fyrox_scripts") {
                                        dependencies["fyrox_scripts"] = value(scripts_version);
                                    }
                                }
                            } else {
                                println!("WARNING: matching editor/scripts version not found!");
                            }
                        }
                    }
                }

                let mut file = File::create(workspace_manifest_path).map_err(|e| e.to_string())?;
                file.write_all(document.to_string().as_bytes())
                    .map_err(|e| e.to_string())?;
            }
        }
    }

    Command::new("cargo")
        .args(["update"])
        .output()
        .map_err(|e| e.to_string())?;

    Ok(())
}
