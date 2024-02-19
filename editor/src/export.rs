use cargo_metadata::Metadata;
use fyrox::{
    core::{
        color::Color,
        log::{Log, LogMessage, MessageKind},
        pool::Handle,
    },
    graph::BaseSceneGraph,
    gui::{
        border::BorderBuilder,
        brush::Brush,
        button::{ButtonBuilder, ButtonMessage},
        formatted_text::WrapMode,
        grid::{Column, GridBuilder, Row},
        list_view::ListViewBuilder,
        message::{MessageDirection, UiMessage},
        path::{PathEditorBuilder, PathEditorMessage},
        scroll_viewer::{ScrollViewerBuilder, ScrollViewerMessage},
        stack_panel::StackPanelBuilder,
        text::TextBuilder,
        widget::{WidgetBuilder, WidgetMessage},
        window::{WindowBuilder, WindowMessage, WindowTitle},
        BuildContext, HorizontalAlignment, Orientation, Thickness, UiNode, UserInterface,
        BRUSH_DARKER,
    },
};
use std::{
    ffi::OsStr,
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

#[allow(dead_code)]
pub struct ExportWindow {
    pub window: Handle<UiNode>,
    log: Handle<UiNode>,
    export: Handle<UiNode>,
    cancel: Handle<UiNode>,
    assets_folders: Handle<UiNode>,
    assets_folders_list: Vec<PathBuf>,
    destination_path: Handle<UiNode>,
    log_scroll_viewer: Handle<UiNode>,
    destination_folder: PathBuf,
    cancel_flag: Arc<AtomicBool>,
    log_message_receiver: Option<Receiver<LogMessage>>,
    build_result_receiver: Option<Receiver<Result<(), String>>>,
}

fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            let from = entry.path();
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

fn has_binary_package(name: &str, metadata: &Metadata) -> bool {
    for package in metadata.packages.iter() {
        if package.name == name {
            for target in package.targets.iter() {
                if target.is_bin() {
                    return true;
                }
            }
        }
    }

    false
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

fn build_package(package_name: &str, cancel_flag: Arc<AtomicBool>) -> Result<(), String> {
    // Build the game first.
    let mut process = std::process::Command::new("cargo");
    let mut handle = match process
        .stderr(Stdio::piped())
        .arg("build")
        .arg("--package")
        .arg(package_name)
        .arg("--release")
        .spawn()
    {
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
                    let err_code = 101;
                    let code = status.code().unwrap_or(err_code);
                    if code == err_code {
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

// TODO: This should be replaced with `--out-dir` flag to cargo when it is stabilized.
fn copy_binaries(
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

fn export(
    destination_folder: PathBuf,
    assets_folders: Vec<PathBuf>,
    cancel_flag: Arc<AtomicBool>,
    package_name: String,
) -> Result<(), String> {
    Log::info("Building the game...");

    prepare_build_dir(&destination_folder)?;
    let metadata = read_metadata()?;

    if !has_binary_package(&package_name, &metadata) {
        return Err(format!(
            "The project does not have `{}` package.",
            package_name
        ));
    }

    build_package(&package_name, cancel_flag)?;

    Log::info("Trying to copy the executable...");

    copy_binaries(&metadata, &package_name, &destination_folder)?;

    Log::info("Trying to copy the assets...");

    // Copy assets.
    for folder in assets_folders {
        Log::info(format!(
            "Trying to copy assets from {} to {}...",
            folder.display(),
            destination_folder.display()
        ));

        Log::verify(copy_dir_all(&folder, destination_folder.join(&folder)));
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

        let destination_folder = PathBuf::from("./build/");
        let assets_folders_list = vec![PathBuf::from("./data/")];

        let export;
        let cancel;
        let log;
        let destination_path;
        let assets_folders;
        let log_scroll_viewer;

        let dest_path_section = StackPanelBuilder::new(
            WidgetBuilder::new()
                .on_row(1)
                .with_child(make_title_text("Destination Folder", 0, ctx))
                .with_child({
                    destination_path = PathEditorBuilder::new(
                        WidgetBuilder::new().with_margin(Thickness::uniform(2.0)),
                    )
                    .with_path(&destination_folder)
                    .build(ctx);
                    destination_path
                }),
        )
        .build(ctx);

        let assets_section = StackPanelBuilder::new(
            WidgetBuilder::new()
                .on_row(2)
                .with_child(make_title_text("Assets Folders", 0, ctx))
                .with_child(
                    BorderBuilder::new(
                        WidgetBuilder::new()
                            .with_background(BRUSH_DARKER)
                            .with_child({
                                let items = assets_folders_list
                                    .iter()
                                    .map(|e| {
                                        TextBuilder::new(WidgetBuilder::new())
                                            .with_text(e.to_string_lossy())
                                            .build(ctx)
                                    })
                                    .collect::<Vec<_>>();
                                assets_folders =
                                    ListViewBuilder::new(WidgetBuilder::new().with_height(100.0))
                                        .with_items(items)
                                        .build(ctx);
                                assets_folders
                            }),
                    )
                    .build(ctx),
                ),
        )
        .build(ctx);

        let log_section = GridBuilder::new(
            WidgetBuilder::new()
                .on_row(3)
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
                .on_row(4)
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

        let window = WindowBuilder::new(WidgetBuilder::new().with_width(400.0).with_height(500.0))
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
                        .with_child(dest_path_section)
                        .with_child(assets_section)
                        .with_child(log_section)
                        .with_child(buttons_section),
                )
                .add_row(Row::auto())
                .add_row(Row::strict(42.0))
                .add_row(Row::auto())
                .add_row(Row::stretch())
                .add_row(Row::strict(26.0))
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
            assets_folders,
            log_scroll_viewer,
            assets_folders_list,
            destination_path,
            destination_folder,
            cancel_flag: Arc::new(AtomicBool::new(false)),
            log_message_receiver: None,
            build_result_receiver: None,
        }
    }

    pub fn open(&self, ui: &UserInterface) {
        ui.send_message(WindowMessage::open_modal(
            self.window,
            MessageDirection::ToWidget,
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

    pub fn handle_ui_message(&mut self, message: &UiMessage, ui: &UserInterface) {
        if let Some(ButtonMessage::Click) = message.data() {
            if message.destination() == self.export {
                let (tx, rx) = mpsc::channel();
                Log::add_listener(tx);
                self.log_message_receiver = Some(rx);

                let destination_folder = self.destination_folder.clone();
                let assets_folders_list = self.assets_folders_list.clone();
                let cancel_flag = self.cancel_flag.clone();

                let (tx, rx) = mpsc::channel();
                self.build_result_receiver = Some(rx);

                ui.send_message(WidgetMessage::enabled(
                    self.export,
                    MessageDirection::ToWidget,
                    false,
                ));

                self.clear_log(ui);

                Log::verify(
                    std::thread::Builder::new()
                        .name("ExportWorkerThread".to_string())
                        .spawn(move || {
                            if std::panic::catch_unwind(|| {
                                tx.send(export(
                                    destination_folder,
                                    assets_folders_list,
                                    cancel_flag,
                                    "executor".to_string(),
                                ))
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
        } else if let Some(PathEditorMessage::Path(path)) = message.data() {
            if message.destination() == self.destination_path {
                self.destination_folder = path.clone();
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
