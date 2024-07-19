use crate::{
    fyrox::{
        core::{
            color::Color,
            log::{Log, LogMessage, MessageKind},
            pool::Handle,
            reflect::prelude::*,
        },
        graph::BaseSceneGraph,
        gui::{
            border::BorderBuilder,
            brush::Brush,
            button::{ButtonBuilder, ButtonMessage},
            decorator::DecoratorBuilder,
            dropdown_list::{DropdownListBuilder, DropdownListMessage},
            formatted_text::WrapMode,
            grid::{Column, GridBuilder, Row},
            inspector::{
                editors::PropertyEditorDefinitionContainer, Inspector, InspectorBuilder,
                InspectorContext, InspectorMessage, PropertyAction,
            },
            list_view::{ListViewBuilder, ListViewMessage},
            message::{MessageDirection, UiMessage},
            scroll_viewer::{ScrollViewerBuilder, ScrollViewerMessage},
            stack_panel::StackPanelBuilder,
            text::TextBuilder,
            widget::{WidgetBuilder, WidgetMessage},
            window::{WindowBuilder, WindowMessage, WindowTitle},
            wrap_panel::WrapPanelBuilder,
            BuildContext, HorizontalAlignment, Orientation, Thickness, UiNode, UserInterface,
            VerticalAlignment, BRUSH_DARKER, BRUSH_LIGHT,
        },
    },
    gui::make_dropdown_list_option,
    message::MessageSender,
    Message,
};
use cargo_metadata::{camino::Utf8Path, Metadata};
use fyrox::graph::SceneGraph;
use std::{
    ffi::OsStr,
    fmt::{Display, Formatter},
    fs,
    io::{self, BufRead, BufReader},
    path::{Path, PathBuf},
    process::Stdio,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver},
        Arc,
    },
    time::Duration,
};
use strum::VariantNames;
use strum_macros::VariantNames;

#[derive(Reflect, Debug, Clone)]
struct ExportOptions {
    #[reflect(hidden)]
    target_platform: TargetPlatform,
    destination_folder: PathBuf,
    include_used_assets: bool,
    assets_folders: Vec<PathBuf>,
    ignored_extensions: Vec<String>,
    #[reflect(hidden)]
    build_targets: Vec<String>,
    #[reflect(hidden)]
    selected_build_target: usize,
    run_after_build: bool,
    open_destination_folder: bool,
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            target_platform: Default::default(),
            destination_folder: "./build/".into(),
            assets_folders: vec!["./data/".into()],
            include_used_assets: false,
            ignored_extensions: vec!["log".to_string()],
            build_targets: vec!["default".to_string()],
            selected_build_target: 0,
            run_after_build: false,
            open_destination_folder: true,
        }
    }
}

#[derive(Copy, Clone, VariantNames, Default, Debug, Eq, PartialEq)]
enum TargetPlatform {
    #[default]
    PC,
    WebAssembly,
    Android,
}

impl Display for TargetPlatform {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                TargetPlatform::PC => "PC",
                TargetPlatform::WebAssembly => "WASM",
                TargetPlatform::Android => "Android",
            }
        )
    }
}

pub struct ExportWindow {
    pub window: Handle<UiNode>,
    log: Handle<UiNode>,
    export: Handle<UiNode>,
    cancel: Handle<UiNode>,
    log_scroll_viewer: Handle<UiNode>,
    cancel_flag: Arc<AtomicBool>,
    log_message_receiver: Option<Receiver<LogMessage>>,
    build_result_receiver: Option<Receiver<Result<(), String>>>,
    target_platform_list: Handle<UiNode>,
    export_options: ExportOptions,
    inspector: Handle<UiNode>,
    build_targets_selector: Handle<UiNode>,
}

fn copy_dir<F>(src: impl AsRef<Path>, dst: impl AsRef<Path>, filter: &F) -> io::Result<()>
where
    F: Fn(&Path) -> bool,
{
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let path = entry.path();
        if !filter(&path) {
            continue;
        }
        if ty.is_dir() {
            copy_dir(path, dst.as_ref().join(entry.file_name()), filter)?;
        } else {
            let from = path;
            let to = dst.as_ref().join(entry.file_name());
            fs::copy(&from, &to)?;
            Log::info(format!(
                "{} successfully cloned to {}",
                from.display(),
                to.display()
            ))
        }
    }
    Ok(())
}

fn read_metadata() -> Result<Metadata, String> {
    return match std::process::Command::new("cargo")
        .arg("metadata")
        .stdout(Stdio::piped())
        .spawn()
    {
        Ok(handle) => match handle.wait_with_output() {
            Ok(output) => match serde_json::from_slice::<Metadata>(&output.stdout) {
                Ok(metadata) => Ok(metadata),
                Err(err) => Err(format!(
                    "Unable to parse workspace metadata. Reason {:?}",
                    err
                )),
            },
            Err(err) => Err(format!(
                "Unable to fetch project metadata. Reason {:?}",
                err
            )),
        },
        Err(err) => Err(format!(
            "Unable to fetch project metadata. Reason {:?}",
            err
        )),
    };
}

fn prepare_build_dir(path: &Path) -> Result<(), String> {
    if path.exists() {
        Log::info("Trying to delete previous build...");

        if let Err(err) = fs::remove_dir_all(path) {
            return Err(format!(
                "Unable to remove previous build at destination path! Reason: {:?}",
                err
            ));
        }
    }

    // Create the new clean folder.
    if let Err(err) = fs::create_dir_all(path) {
        return Err(format!(
            "Unable to create build directory at destination path! Reason: {:?}",
            err
        ));
    }

    Ok(())
}

fn is_wasm_pack_installed() -> bool {
    if let Ok(mut handle) = std::process::Command::new("wasm-pack --version").spawn() {
        if let Ok(code) = handle.wait() {
            if code.code().unwrap_or(1) == 0 {
                return true;
            }
        }
    }

    false
}

fn cargo_install(crate_name: &str) -> Result<(), String> {
    Log::info(format!("Trying to install {crate_name}..."));

    let mut process = std::process::Command::new("cargo");
    match process
        .stderr(Stdio::piped())
        .arg("install")
        .arg(crate_name)
        .spawn()
    {
        Ok(handle) => match handle.wait_with_output() {
            Ok(output) => {
                if output.status.code().unwrap_or(1) == 0 {
                    Log::info(format!("{crate_name} installed successfully!"));

                    Ok(())
                } else {
                    Err(String::from_utf8_lossy(&output.stderr).to_string())
                }
            }
            Err(err) => Err(format!("Unable to install {crate_name}. Reason: {:?}", err)),
        },
        Err(err) => Err(format!("Unable to install {crate_name}. Reason: {:?}", err)),
    }
}

fn install_build_target(target: &str) -> Result<(), String> {
    Log::info(format!("Trying to install {} build target...", target));

    let mut process = std::process::Command::new("rustup");
    match process
        .stderr(Stdio::piped())
        .arg("target")
        .arg("add")
        .arg(target)
        .spawn()
    {
        Ok(handle) => match handle.wait_with_output() {
            Ok(output) => {
                if output.status.code().unwrap_or(1) == 0 {
                    Log::info(format!("{} target installed successfully!", target));

                    Ok(())
                } else {
                    Err(String::from_utf8_lossy(&output.stderr).to_string())
                }
            }
            Err(err) => Err(format!(
                "Unable to install {} target. Reason: {:?}",
                target, err
            )),
        },
        Err(err) => Err(format!(
            "Unable to install {} target. Reason: {:?}",
            target, err
        )),
    }
}

fn configure_build_environment(
    target_platform: TargetPlatform,
    build_target: &str,
) -> Result<(), String> {
    match target_platform {
        TargetPlatform::PC => {
            // Assume that rustup have installed the correct toolchain.
            Ok(())
        }
        TargetPlatform::WebAssembly => {
            // Check if the user have `wasm-pack` installed.
            if !is_wasm_pack_installed() {
                cargo_install("wasm-pack")?;
            }
            install_build_target(build_target)
        }
        TargetPlatform::Android => {
            cargo_install("cargo-apk")?;
            install_build_target(build_target)
        }
    }
}

fn build_package(
    package_name: &str,
    build_target: &str,
    package_dir_path: &Utf8Path,
    target_platform: TargetPlatform,
    cancel_flag: Arc<AtomicBool>,
) -> Result<(), String> {
    configure_build_environment(target_platform, build_target)?;

    let mut process = match target_platform {
        TargetPlatform::PC => {
            let mut process = std::process::Command::new("cargo");
            process
                .stderr(Stdio::piped())
                .arg("build")
                .arg("--package")
                .arg(package_name)
                .arg("--release");
            process
        }
        TargetPlatform::WebAssembly => {
            let mut process = std::process::Command::new("wasm-pack");
            process
                .stderr(Stdio::piped())
                .arg("build")
                .arg(package_dir_path)
                .arg("--target")
                .arg("web");
            process
        }
        TargetPlatform::Android => {
            let mut process = std::process::Command::new("cargo-apk");
            process
                .stderr(Stdio::piped())
                .arg("apk")
                .arg("build")
                .arg("--package")
                .arg(package_name)
                .arg("--target")
                .arg(build_target)
                .arg("--release");
            process
        }
    };

    let mut handle = match process.spawn() {
        Ok(handle) => handle,
        Err(err) => {
            return Err(format!("Failed to build the game. Reason: {:?}", err));
        }
    };

    let mut stderr = handle.stderr.take().unwrap();

    // Spin until the build is finished.
    loop {
        if cancel_flag.load(Ordering::Relaxed) {
            Log::verify(handle.kill());
            Log::warn("Build was cancelled.");
            return Ok(());
        }

        for line in BufReader::new(&mut stderr).lines().take(10).flatten() {
            Log::writeln(MessageKind::Information, line);
        }

        match handle.try_wait() {
            Ok(status) => {
                if let Some(status) = status {
                    let code = status.code().unwrap_or(1);
                    if code != 0 {
                        return Err("Failed to build the game.".to_string());
                    } else {
                        Log::info("The game was built successfully.");
                        break;
                    }
                }
            }
            Err(err) => {
                return Err(format!("Failed to build the game. Reason: {:?}", err));
            }
        }

        std::thread::sleep(Duration::from_millis(500));
    }

    Ok(())
}

fn copy_binaries_pc(
    metadata: &Metadata,
    package_name: &str,
    destination_folder: &Path,
) -> Result<(), String> {
    let mut binary_paths = vec![];
    for entry in fs::read_dir(metadata.target_directory.join("release"))
        .unwrap()
        .flatten()
    {
        if let Ok(file_metadata) = entry.metadata() {
            if !file_metadata.file_type().is_file() {
                continue;
            }
        }

        if let Some(stem) = entry.path().file_stem() {
            if stem == OsStr::new(package_name) {
                binary_paths.push(entry.path());
            }
        }
    }
    for path in binary_paths {
        if let Some(file_name) = path.file_name() {
            match fs::copy(&path, &destination_folder.join(file_name)) {
                Ok(_) => {
                    Log::info(format!(
                        "{} was successfully copied to the {} folder.",
                        path.display(),
                        destination_folder.display()
                    ));
                }
                Err(err) => {
                    Log::warn(format!(
                        "Failed to copy {} file to the {} folder. Reason: {:?}",
                        path.display(),
                        destination_folder.display(),
                        err
                    ));
                }
            }
        }
    }

    Ok(())
}

fn copy_binaries_android(
    metadata: &Metadata,
    package_name: &str,
    destination_folder: &Path,
) -> Result<(), String> {
    let mut binary_paths = vec![];
    for entry in fs::read_dir(metadata.target_directory.join("release/apk"))
        .unwrap()
        .flatten()
    {
        if let Ok(file_metadata) = entry.metadata() {
            if !file_metadata.file_type().is_file() {
                continue;
            }
        }

        if let Some(stem) = entry.path().file_stem() {
            if stem == OsStr::new(package_name) {
                binary_paths.push(entry.path());
            }
        }
    }
    for path in binary_paths {
        if let Some(file_name) = path.file_name() {
            match fs::copy(&path, &destination_folder.join(file_name)) {
                Ok(_) => {
                    Log::info(format!(
                        "{} was successfully copied to the {} folder.",
                        path.display(),
                        destination_folder.display()
                    ));
                }
                Err(err) => {
                    Log::warn(format!(
                        "Failed to copy {} file to the {} folder. Reason: {:?}",
                        path.display(),
                        destination_folder.display(),
                        err
                    ));
                }
            }
        }
    }

    Ok(())
}

fn copy_binaries_wasm(package_dir_path: &Path, destination_folder: &Path) -> Result<(), String> {
    copy_dir(package_dir_path, destination_folder, &|path: &Path| {
        if path.is_file() {
            if path.file_name() == Some(OsStr::new("Cargo.toml"))
                || path.file_name() == Some(OsStr::new("README.md"))
                || path.file_name() == Some(OsStr::new(".gitignore"))
            {
                return false;
            }
        } else if path.is_dir() && path.file_name() == Some(OsStr::new("target")) {
            return false;
        }

        true
    })
    .map_err(|e| e.to_string())
}

fn export(export_options: ExportOptions, cancel_flag: Arc<AtomicBool>) -> Result<(), String> {
    Log::info("Building the game...");

    prepare_build_dir(&export_options.destination_folder)?;
    let metadata = read_metadata()?;

    let package_name = match export_options.target_platform {
        TargetPlatform::PC => "executor",
        TargetPlatform::WebAssembly => "executor-wasm",
        TargetPlatform::Android => "executor-android",
    };

    let Some(package) = metadata.packages.iter().find(|p| p.name == package_name) else {
        return Err(format!(
            "The project does not have `{}` package.",
            package_name
        ));
    };

    let package_dir_path = package.manifest_path.as_path().parent().unwrap();

    let mut temp_folders = Vec::new();

    // Copy assets
    match export_options.target_platform {
        TargetPlatform::PC | TargetPlatform::WebAssembly => {
            Log::info("Trying to copy the assets...");

            for folder in export_options.assets_folders {
                Log::info(format!(
                    "Trying to copy assets from {} to {}...",
                    folder.display(),
                    export_options.destination_folder.display()
                ));

                Log::verify(copy_dir(
                    &folder,
                    export_options.destination_folder.join(&folder),
                    &|_| true,
                ));
            }
        }
        TargetPlatform::Android => {
            // Asset management on Android is quite annoying, because all other target platforms
            // uses the workspace manifest path as a root directory and all paths in code/assets
            // stored relatively to it. On Android, however, all your assets must be in unified
            // assets storage. This means that, if we simply specify assets folder to be `../data`
            // (relative to `executor-android`), it will put all the assets in the storage, but
            // their path will become relative to the storage. For example, in your code you can
            // reference an asset like this: `data/my/textures/foo.jpg` and when build script for
            // Android will clone the assets from `data` folder, your asset will have this
            // actual path `my/textures/foo.jpg`. In other words, `data` is stripped from the path.
            //
            // To solve this, we just copy the entire assets folder to a temporary folder set in
            // the manifest of `executor-android` and then cargo-apk will pack these assets and the
            // paths to assets will become valid.
            //
            // It could very well possible, that I'm missing something and this could be fixed in
            // a much easier way.
            if let Some(assets) = package
                .metadata
                .get("android")
                .and_then(|v| v.get("assets"))
                .and_then(|v| v.as_str())
            {
                let temp_assets_storage = package_dir_path.join(assets).as_std_path().to_path_buf();

                Log::info(format!(
                    "Trying to copy the assets to a temporary storage {}...",
                    temp_assets_storage.display()
                ));

                if !temp_assets_storage.exists() {
                    Log::verify(std::fs::create_dir_all(&temp_assets_storage));
                }

                temp_folders.push(temp_assets_storage.clone());

                for folder in export_options.assets_folders {
                    Log::info(format!(
                        "Trying to copy assets from {} to {}...",
                        folder.display(),
                        temp_assets_storage.display()
                    ));

                    Log::verify(copy_dir(
                        &folder,
                        temp_assets_storage.join(&folder),
                        &|_| true,
                    ));
                }
            } else {
                return Err("Android executor must specify assets folder in \
                    [package.metadata.android] section"
                    .to_string());
            }
        }
    }

    build_package(
        package_name,
        &export_options.build_targets[export_options.selected_build_target],
        package_dir_path,
        export_options.target_platform,
        cancel_flag,
    )?;

    match export_options.target_platform {
        TargetPlatform::PC => {
            // TODO: This should be replaced with `--out-dir` flag to cargo when it is stabilized.
            Log::info("Trying to copy the executable...");
            copy_binaries_pc(&metadata, package_name, &export_options.destination_folder)?;
        }
        TargetPlatform::WebAssembly => {
            Log::info("Trying to copy the executable...");
            copy_binaries_wasm(
                package_dir_path.as_std_path(),
                &export_options.destination_folder,
            )?;
        }
        TargetPlatform::Android => {
            Log::info("Trying to copy the apk...");
            copy_binaries_android(&metadata, package_name, &export_options.destination_folder)?;
        }
    }

    // Remove all temp folders.
    for temp_folder in temp_folders {
        Log::verify(std::fs::remove_dir_all(temp_folder));
    }

    if let Ok(destination_folder) = export_options.destination_folder.canonicalize() {
        if export_options.run_after_build {
            match export_options.target_platform {
                TargetPlatform::PC => {
                    #[allow(unused_mut)]
                    let mut path = destination_folder.join(package_name);
                    #[cfg(windows)]
                    {
                        path.set_extension("exe");
                    }
                    Log::verify(open::that_detached(path))
                }
                TargetPlatform::WebAssembly => {
                    Log::verify(cargo_install("basic-http-server"));

                    Log::verify(
                        std::process::Command::new("basic-http-server")
                            .arg("--addr")
                            .arg("127.0.0.1:4000")
                            .current_dir(&destination_folder)
                            .spawn(),
                    );

                    Log::verify(open::that_detached("http://127.0.0.1:4000"));
                }
                TargetPlatform::Android => {
                    if let Ok(adb) = std::process::Command::new("adb")
                        .current_dir(&destination_folder)
                        .arg("install")
                        .arg(format!("{package_name}.apk"))
                        .spawn()
                    {
                        match adb.wait_with_output() {
                            Ok(_) => {
                                let compatible_package_name = package_name.replace('-', "_");
                                Log::verify(
                                    std::process::Command::new("adb")
                                        .arg("shell")
                                        .arg("am")
                                        .arg("start")
                                        .arg("-n")
                                        .arg(format!(
                                            "rust.{compatible_package_name}/android.app.NativeActivity"
                                        ))
                                        .spawn(),
                                );
                            }
                            Err(err) => Log::err(format!("ADB error: {:?}", err)),
                        }
                    }
                }
            }
        }

        if export_options.open_destination_folder {
            Log::verify(open::that_detached(destination_folder));
        }
    }

    Ok(())
}

fn make_title_text(text: &str, row: usize, ctx: &mut BuildContext) -> Handle<UiNode> {
    TextBuilder::new(
        WidgetBuilder::new()
            .on_row(row)
            .with_foreground(Brush::Solid(Color::CORN_SILK))
            .with_margin(Thickness::uniform(2.0)),
    )
    .with_font_size(14.0)
    .with_text(text)
    .build(ctx)
}

impl ExportWindow {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let instructions =
            "Select the target directory in which you want to export the current project. You can \
            also specify the assets, that will be included in the final build. Previous content of \
            the build folder will be completely erased when you press Export.";

        let export;
        let cancel;
        let log;
        let log_scroll_viewer;
        let target_platform_list;
        let export_options = ExportOptions::default();

        let platform_section = StackPanelBuilder::new(
            WidgetBuilder::new()
                .on_row(1)
                .with_child(make_title_text("Target Platform", 0, ctx))
                .with_child({
                    target_platform_list = ListViewBuilder::new(
                        WidgetBuilder::new()
                            .with_margin(Thickness::uniform(2.0))
                            .with_height(60.0),
                    )
                    .with_items_panel(
                        WrapPanelBuilder::new(WidgetBuilder::new())
                            .with_orientation(Orientation::Horizontal)
                            .build(ctx),
                    )
                    .with_items(
                        TargetPlatform::VARIANTS
                            .iter()
                            .enumerate()
                            .map(|(i, p)| {
                                DecoratorBuilder::new(BorderBuilder::new(
                                    WidgetBuilder::new()
                                        .with_width(100.0)
                                        .with_height(50.0)
                                        .with_margin(Thickness::uniform(1.0))
                                        .with_child(
                                            TextBuilder::new(WidgetBuilder::new())
                                                .with_vertical_text_alignment(
                                                    VerticalAlignment::Center,
                                                )
                                                .with_horizontal_text_alignment(
                                                    HorizontalAlignment::Center,
                                                )
                                                .with_text(p)
                                                .with_font_size(14.0)
                                                .build(ctx),
                                        ),
                                ))
                                .with_selected(i == 0)
                                .build(ctx)
                            })
                            .collect::<Vec<_>>(),
                    )
                    .build(ctx);
                    target_platform_list
                }),
        )
        .build(ctx);

        let build_targets_selector;
        let grid = GridBuilder::new(
            WidgetBuilder::new()
                .on_row(2)
                .with_child(
                    TextBuilder::new(WidgetBuilder::new().with_margin(Thickness::uniform(2.0)))
                        .with_vertical_text_alignment(VerticalAlignment::Center)
                        .with_text("Build Target")
                        .build(ctx),
                )
                .with_child({
                    build_targets_selector =
                        DropdownListBuilder::new(WidgetBuilder::new().on_column(1))
                            .with_items(
                                export_options
                                    .build_targets
                                    .iter()
                                    .map(|opt| make_dropdown_list_option(ctx, opt))
                                    .collect::<Vec<_>>(),
                            )
                            .with_selected(0)
                            .build(ctx);
                    build_targets_selector
                }),
        )
        .add_column(Column::auto())
        .add_column(Column::stretch())
        .add_row(Row::strict(22.0))
        .build(ctx);

        let inspector;
        let export_options_section = BorderBuilder::new(
            WidgetBuilder::new()
                .on_row(3)
                .with_margin(Thickness::uniform(2.0))
                .with_background(BRUSH_LIGHT)
                .with_child(
                    ScrollViewerBuilder::new(
                        WidgetBuilder::new().with_margin(Thickness::uniform(2.0)),
                    )
                    .with_content({
                        let context = InspectorContext::from_object(
                            &export_options,
                            ctx,
                            Arc::new(PropertyEditorDefinitionContainer::with_default_editors()),
                            None,
                            1,
                            0,
                            true,
                            Default::default(),
                            150.0,
                        );

                        inspector = InspectorBuilder::new(WidgetBuilder::new())
                            .with_context(context)
                            .build(ctx);
                        inspector
                    })
                    .build(ctx),
                ),
        )
        .build(ctx);

        let log_section = GridBuilder::new(
            WidgetBuilder::new()
                .on_row(4)
                .with_child(make_title_text("Export Log", 0, ctx))
                .with_child(
                    BorderBuilder::new(
                        WidgetBuilder::new()
                            .on_row(1)
                            .with_background(BRUSH_DARKER)
                            .with_margin(Thickness::uniform(2.0))
                            .with_child({
                                log_scroll_viewer = ScrollViewerBuilder::new(
                                    WidgetBuilder::new().with_margin(Thickness::uniform(2.0)),
                                )
                                .with_content({
                                    log = StackPanelBuilder::new(WidgetBuilder::new()).build(ctx);
                                    log
                                })
                                .build(ctx);
                                log_scroll_viewer
                            }),
                    )
                    .build(ctx),
                ),
        )
        .add_row(Row::auto())
        .add_row(Row::stretch())
        .add_column(Column::auto())
        .build(ctx);

        let buttons_section = StackPanelBuilder::new(
            WidgetBuilder::new()
                .on_row(5)
                .with_horizontal_alignment(HorizontalAlignment::Right)
                .with_child({
                    export = ButtonBuilder::new(
                        WidgetBuilder::new()
                            .with_width(100.0)
                            .with_margin(Thickness::uniform(2.0)),
                    )
                    .with_text("Export")
                    .build(ctx);
                    export
                })
                .with_child({
                    cancel = ButtonBuilder::new(
                        WidgetBuilder::new()
                            .with_width(100.0)
                            .with_margin(Thickness::uniform(2.0)),
                    )
                    .with_text("Cancel")
                    .build(ctx);
                    cancel
                }),
        )
        .with_orientation(Orientation::Horizontal)
        .build(ctx);

        let window = WindowBuilder::new(WidgetBuilder::new().with_width(500.0).with_height(650.0))
            .open(false)
            .with_content(
                GridBuilder::new(
                    WidgetBuilder::new()
                        .with_child(
                            TextBuilder::new(
                                WidgetBuilder::new()
                                    .on_row(0)
                                    .with_margin(Thickness::uniform(2.0)),
                            )
                            .with_wrap(WrapMode::Word)
                            .with_text(instructions)
                            .build(ctx),
                        )
                        .with_child(platform_section)
                        .with_child(grid)
                        .with_child(export_options_section)
                        .with_child(log_section)
                        .with_child(buttons_section),
                )
                .add_row(Row::auto())
                .add_row(Row::auto())
                .add_row(Row::auto())
                .add_row(Row::strict(200.0))
                .add_row(Row::stretch())
                .add_row(Row::strict(32.0))
                .add_column(Column::auto())
                .build(ctx),
            )
            .with_title(WindowTitle::text("Export Project"))
            .build(ctx);

        Self {
            window,
            log,
            export,
            cancel,
            log_scroll_viewer,
            cancel_flag: Arc::new(AtomicBool::new(false)),
            log_message_receiver: None,
            build_result_receiver: None,
            target_platform_list,
            export_options,
            inspector,
            build_targets_selector,
        }
    }

    pub fn open(&self, ui: &UserInterface) {
        ui.send_message(WindowMessage::open_modal(
            self.window,
            MessageDirection::ToWidget,
            true,
            true,
        ));
    }

    pub fn close_and_destroy(&mut self, ui: &UserInterface) {
        ui.send_message(WindowMessage::close(
            self.window,
            MessageDirection::ToWidget,
        ));
        ui.send_message(WidgetMessage::remove(
            self.window,
            MessageDirection::ToWidget,
        ));
        self.log_message_receiver = None;
        self.build_result_receiver = None;
    }

    fn clear_log(&self, ui: &UserInterface) {
        for child in ui.node(self.log).children() {
            ui.send_message(WidgetMessage::remove(*child, MessageDirection::ToWidget));
        }
    }

    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        ui: &mut UserInterface,
        sender: &MessageSender,
    ) {
        if let Some(ButtonMessage::Click) = message.data() {
            if message.destination() == self.export {
                let (tx, rx) = mpsc::channel();
                Log::add_listener(tx);
                self.log_message_receiver = Some(rx);

                let (tx, rx) = mpsc::channel();
                self.build_result_receiver = Some(rx);

                ui.send_message(WidgetMessage::enabled(
                    self.export,
                    MessageDirection::ToWidget,
                    false,
                ));

                self.clear_log(ui);

                let cancel_flag = self.cancel_flag.clone();
                let export_options = self.export_options.clone();

                Log::verify(
                    std::thread::Builder::new()
                        .name("ExportWorkerThread".to_string())
                        .spawn(move || {
                            if std::panic::catch_unwind(|| {
                                tx.send(export(export_options, cancel_flag))
                                    .expect("Channel must exist!")
                            })
                            .is_err()
                            {
                                Log::err("Unexpected error has occurred in the exporter thread.")
                            }
                        }),
                );
            } else if message.destination() == self.cancel {
                self.close_and_destroy(ui);
            }
        } else if let Some(ListViewMessage::SelectionChanged(selection)) = message.data() {
            if message.destination() == self.target_platform_list
                && message.direction() == MessageDirection::FromWidget
            {
                if let Some(index) = selection.first().cloned() {
                    match index {
                        0 => self.export_options.target_platform = TargetPlatform::PC,
                        1 => self.export_options.target_platform = TargetPlatform::WebAssembly,
                        2 => self.export_options.target_platform = TargetPlatform::Android,
                        _ => Log::err("Unhandled platform index!"),
                    }

                    // TODO: move this to settings.
                    let build_targets = match self.export_options.target_platform {
                        TargetPlatform::PC => vec!["default".to_string()],
                        TargetPlatform::WebAssembly => vec!["wasm32-unknown-unknown".to_string()],
                        TargetPlatform::Android => {
                            vec![
                                "armv7-linux-androideabi".to_string(),
                                "aarch64-linux-android".to_string(),
                            ]
                        }
                    };

                    self.export_options.build_targets = build_targets;

                    let ui_items = self
                        .export_options
                        .build_targets
                        .iter()
                        .map(|name| make_dropdown_list_option(&mut ui.build_ctx(), name))
                        .collect::<Vec<_>>();

                    ui.send_message(DropdownListMessage::items(
                        self.build_targets_selector,
                        MessageDirection::ToWidget,
                        ui_items,
                    ));
                }
            }
        } else if let Some(InspectorMessage::PropertyChanged(args)) = message.data() {
            if message.destination() == self.inspector
                && message.direction() == MessageDirection::FromWidget
            {
                PropertyAction::from_field_kind(&args.value).apply(
                    &args.path(),
                    &mut self.export_options,
                    &mut |result| {
                        Log::verify(result);
                    },
                );
                sender.send(Message::ForceSync);
            }
        } else if let Some(DropdownListMessage::SelectionChanged(Some(index))) = message.data() {
            if message.destination() == self.build_targets_selector
                && message.direction() == MessageDirection::FromWidget
            {
                self.export_options.selected_build_target = *index;
            }
        }
    }

    pub fn sync_to_model(&self, ui: &mut UserInterface) {
        if let Some(inspector) = ui.try_get_of_type::<Inspector>(self.inspector) {
            let ctx = inspector.context().clone();
            if let Err(sync_errors) =
                ctx.sync(&self.export_options, ui, 0, true, Default::default())
            {
                for error in sync_errors {
                    Log::err(format!("Failed to sync property. Reason: {:?}", error))
                }
            }
        }
    }

    pub fn update(&mut self, ui: &mut UserInterface) {
        if let Some(log_message_receiver) = self.log_message_receiver.as_mut() {
            while let Ok(message) = log_message_receiver.try_recv() {
                let ctx = &mut ui.build_ctx();
                let color = match message.kind {
                    MessageKind::Information => Color::ANTIQUE_WHITE,
                    MessageKind::Warning => Color::ORANGE,
                    MessageKind::Error => Color::RED,
                };
                let entry = TextBuilder::new(
                    WidgetBuilder::new()
                        .with_margin(Thickness::uniform(1.0))
                        .with_foreground(Brush::Solid(color)),
                )
                .with_wrap(WrapMode::Letter)
                .with_text(format!("> {}", message.content))
                .build(ctx);

                ui.send_message(WidgetMessage::link(
                    entry,
                    MessageDirection::ToWidget,
                    self.log,
                ));

                ui.send_message(ScrollViewerMessage::scroll_to_end(
                    self.log_scroll_viewer,
                    MessageDirection::ToWidget,
                ));
            }
        }

        if let Some(receiver) = self.build_result_receiver.as_ref() {
            if let Ok(result) = receiver.try_recv() {
                match result {
                    Ok(_) => {
                        Log::info("Build finished!");
                    }
                    Err(err) => Log::err(format!("Build failed! Reason: {}", err)),
                }

                ui.send_message(WidgetMessage::enabled(
                    self.export,
                    MessageDirection::ToWidget,
                    true,
                ));
            }
        }
    }
}
