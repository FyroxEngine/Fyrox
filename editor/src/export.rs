use fyrox::{
    core::{
        color::Color,
        log::{Log, LogMessage, MessageKind},
        pool::Handle,
    },
    gui::{
        brush::Brush,
        button::{ButtonBuilder, ButtonMessage},
        formatted_text::WrapMode,
        grid::{Column, GridBuilder, Row},
        list_view::ListViewBuilder,
        message::{MessageDirection, UiMessage},
        path::PathEditorBuilder,
        scroll_viewer::ScrollViewerBuilder,
        stack_panel::StackPanelBuilder,
        text::TextBuilder,
        widget::{WidgetBuilder, WidgetMessage},
        window::{WindowBuilder, WindowMessage, WindowTitle},
        BuildContext, HorizontalAlignment, Orientation, Thickness, UiNode, UserInterface,
    },
};
use std::{
    fs, io,
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
    destination_folder: PathBuf,
    cancel_flag: Arc<AtomicBool>,
    receiver: Option<Receiver<LogMessage>>,
}

fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}

fn export(destination_folder: PathBuf, assets_folders: Vec<PathBuf>, cancel_flag: Arc<AtomicBool>) {
    Log::info("Building the game...");

    // Build the game first.
    //  cargo +nightly build --package executor --out-dir=destination_folder -Z unstable-options
    let mut process = std::process::Command::new("cargo");
    let mut handle = match process
        .stderr(Stdio::piped())
        .arg("+nightly")
        .arg("build")
        .arg("--package")
        .arg("executor")
        .arg("--release")
        .arg("--out-dir")
        .arg(&destination_folder)
        .arg("-Z")
        .arg("unstable-options")
        .spawn()
    {
        Ok(handle) => handle,
        Err(err) => {
            Log::err(format!("Failed to build the game. Reason: {:?}", err));
            return;
        }
    };

    // Spin until the build is finished.
    loop {
        if cancel_flag.load(Ordering::Relaxed) {
            Log::verify(handle.kill());
            Log::warn("Build was cancelled.");
            return;
        }

        match handle.try_wait() {
            Ok(status) => {
                if let Some(status) = status {
                    let err_code = 101;
                    let code = status.code().unwrap_or(err_code);
                    if code == err_code {
                        Log::err("Failed to build the game.");
                        return;
                    } else {
                        Log::info("The game was built successfully.");
                        break;
                    }
                }
            }
            Err(err) => {
                Log::err(format!("Failed to build the game. Reason: {:?}", err));
                return;
            }
        }

        std::thread::sleep(Duration::from_millis(500));
    }

    // Copy assets.
    for folder in assets_folders {
        Log::verify(copy_dir_all(&folder, destination_folder.join(&folder)));
    }
}

impl ExportWindow {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let instructions =
            "Select the target directory in which you want to export the current project. You can \
            also specify the assets, that will be include in the final build.";

        let export;
        let cancel;
        let log;
        let destination_path;
        let assets_folders;
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
                        .with_child({
                            destination_path = PathEditorBuilder::new(
                                WidgetBuilder::new()
                                    .on_row(1)
                                    .with_margin(Thickness::uniform(2.0)),
                            )
                            .with_path("./build/")
                            .build(ctx);
                            destination_path
                        })
                        .with_child({
                            assets_folders = ListViewBuilder::new(WidgetBuilder::new().on_row(2))
                                .with_items(vec![TextBuilder::new(WidgetBuilder::new())
                                    .with_text("./data")
                                    .build(ctx)])
                                .build(ctx);
                            assets_folders
                        })
                        .with_child(
                            ScrollViewerBuilder::new(
                                WidgetBuilder::new()
                                    .on_row(3)
                                    .with_margin(Thickness::uniform(2.0)),
                            )
                            .with_content({
                                log = StackPanelBuilder::new(WidgetBuilder::new()).build(ctx);
                                log
                            })
                            .build(ctx),
                        )
                        .with_child(
                            StackPanelBuilder::new(
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
                            .build(ctx),
                        ),
                )
                .add_row(Row::auto())
                .add_row(Row::strict(22.0))
                .add_row(Row::strict(100.0))
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
            assets_folders_list: vec!["./data".into()],
            destination_path,
            destination_folder: Default::default(),
            cancel_flag: Arc::new(AtomicBool::new(false)),
            receiver: None,
        }
    }

    pub fn open(&self, ui: &UserInterface) {
        ui.send_message(WindowMessage::open(
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
        self.receiver = None;
    }

    pub fn handle_ui_message(&mut self, message: &UiMessage, ui: &UserInterface) {
        if let Some(ButtonMessage::Click) = message.data() {
            if message.destination() == self.export {
                let (tx, rx) = mpsc::channel();
                Log::add_listener(tx);
                self.receiver = Some(rx);

                let destination_folder = self.destination_folder.clone();
                let assets_folders_list = self.assets_folders_list.clone();
                let cancel_flag = self.cancel_flag.clone();

                Log::verify(
                    std::thread::Builder::new()
                        .name("ExportWorkerThread".to_string())
                        .spawn(|| export(destination_folder, assets_folders_list, cancel_flag)),
                );

                ui.send_message(WidgetMessage::enabled(
                    self.export,
                    MessageDirection::ToWidget,
                    false,
                ));
            } else if message.destination() == self.cancel {
                self.close_and_destroy(ui);
            }
        }
    }

    pub fn update(&mut self, ui: &mut UserInterface) {
        if let Some(receiver) = self.receiver.as_mut() {
            while let Ok(message) = receiver.try_recv() {
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
                .with_wrap(WrapMode::Word)
                .with_text(message.content)
                .build(ctx);

                ui.send_message(WidgetMessage::link(
                    entry,
                    MessageDirection::ToWidget,
                    self.log,
                ));
            }
        }
    }
}
