use crate::{
    gui::{BuildContext, UiMessage, UiNode},
    GameEngine, Message, STARTUP_WORKING_DIR,
};
use rg3d::core::algebra::Vector2;
use rg3d::core::scope_profile;
use rg3d::core::visitor::{Visit, VisitResult, Visitor};
use rg3d::gui::border::BorderBuilder;
use rg3d::gui::decorator::DecoratorBuilder;
use rg3d::gui::message::ListViewMessage;
use rg3d::{
    core::pool::Handle,
    gui::{
        button::ButtonBuilder,
        file_browser::FileSelectorBuilder,
        grid::{Column, GridBuilder, Row},
        list_view::ListViewBuilder,
        message::{
            ButtonMessage, FileSelectorMessage, MessageDirection, TextBoxMessage, UiMessageData,
            WidgetMessage, WindowMessage,
        },
        stack_panel::StackPanelBuilder,
        text::TextBuilder,
        text_box::TextBoxBuilder,
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowTitle},
        HorizontalAlignment, Orientation, Thickness, VerticalAlignment,
    },
};
use std::{
    cell::RefCell,
    path::{Path, PathBuf},
    rc::Rc,
    sync::mpsc::Sender,
};

#[derive(Default, Eq, PartialEq)]
struct HistoryEntry {
    work_dir: PathBuf,
    textures_path: PathBuf,
}

impl Visit for HistoryEntry {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.work_dir.visit("WorkDir", visitor)?;
        self.textures_path.visit("TexturesPath", visitor)?;

        visitor.leave_region()
    }
}

pub const HISTORY_PATH: &str = "history.bin";

pub struct Configurator {
    pub window: Handle<UiNode>,
    textures_dir_browser: Handle<UiNode>,
    work_dir_browser: Handle<UiNode>,
    select_work_dir: Handle<UiNode>,
    select_textures_dir: Handle<UiNode>,
    ok: Handle<UiNode>,
    sender: Sender<Message>,
    work_dir: PathBuf,
    textures_path: PathBuf,
    tb_work_dir: Handle<UiNode>,
    tb_textures_path: Handle<UiNode>,
    lv_history: Handle<UiNode>,
    history: Vec<HistoryEntry>,
}

fn make_history_entry_widget(ctx: &mut BuildContext, entry: &HistoryEntry) -> Handle<UiNode> {
    DecoratorBuilder::new(BorderBuilder::new(
        WidgetBuilder::new()
            .with_height(32.0)
            .with_margin(Thickness {
                left: 1.0,
                top: 0.0,
                right: 1.0,
                bottom: 1.0,
            })
            .with_child(
                TextBuilder::new(WidgetBuilder::new())
                    .with_text(format!(
                        "WD: {}\nTP: {}",
                        entry.work_dir.display(),
                        entry.textures_path.display()
                    ))
                    .with_vertical_text_alignment(VerticalAlignment::Center)
                    .build(ctx),
            ),
    ))
    .build(ctx)
}

impl Configurator {
    pub fn new(sender: Sender<Message>, ctx: &mut BuildContext) -> Self {
        let select_work_dir;
        let select_textures_dir;
        let ok;
        let tb_work_dir;
        let tb_textures_path;

        let filter = Rc::new(RefCell::new(|p: &Path| p.is_dir()));

        let scene_browser = FileSelectorBuilder::new(
            WindowBuilder::new(WidgetBuilder::new().with_width(300.0).with_height(400.0))
                .open(false)
                .with_title(WindowTitle::Text("Select Textures Path".into())),
        )
        .with_filter(filter.clone())
        .build(ctx);

        let folder_browser = FileSelectorBuilder::new(
            WindowBuilder::new(WidgetBuilder::new().with_width(300.0).with_height(400.0))
                .open(false)
                .with_title(WindowTitle::Text("Select Working Directory".into())),
        )
        .with_filter(filter)
        .build(ctx);

        // Load history.
        let mut history: Vec<HistoryEntry> = Vec::new();
        if let Ok(mut visitor) =
            Visitor::load_binary(STARTUP_WORKING_DIR.lock().unwrap().join(HISTORY_PATH))
        {
            history.visit("History", &mut visitor).unwrap();
        }

        // Remove entries with invalid paths.
        history = history
            .into_iter()
            .filter(|e| e.textures_path.exists() && e.work_dir.exists())
            .collect::<Vec<_>>();

        let message = "Please select a working directory of a project your will\
         work on and a path to the textures. Textures directory must be under working\
          directory!";

        let lv_history;
        let window = WindowBuilder::new(
            WidgetBuilder::new()
                .with_width(370.0)
                .with_height(250.0)
                .with_min_size(Vector2::new(370.0, 250.0)),
        )
        .with_title(WindowTitle::Text("Configure Editor".into()))
        .open(false)
        .can_close(false)
        .with_content(
            GridBuilder::new(
                WidgetBuilder::new()
                    .with_margin(Thickness::uniform(1.0))
                    .with_child(
                        TextBuilder::new(WidgetBuilder::new().with_margin(Thickness::uniform(1.0)))
                            .with_text(message)
                            .with_wrap(true)
                            .build(ctx),
                    )
                    .with_child(
                        GridBuilder::new(
                            WidgetBuilder::new()
                                .on_row(1)
                                .with_child(
                                    TextBuilder::new(
                                        WidgetBuilder::new()
                                            .on_row(0)
                                            .on_column(0)
                                            .with_margin(Thickness::uniform(1.0))
                                            .with_vertical_alignment(VerticalAlignment::Center),
                                    )
                                    .with_text("Working Directory")
                                    .build(ctx),
                                )
                                .with_child({
                                    tb_work_dir = TextBoxBuilder::new(
                                        WidgetBuilder::new()
                                            .on_row(0)
                                            .on_column(1)
                                            .with_margin(Thickness::uniform(1.0)),
                                    )
                                    .with_vertical_text_alignment(VerticalAlignment::Center)
                                    .build(ctx);
                                    tb_work_dir
                                })
                                .with_child({
                                    select_work_dir = ButtonBuilder::new(
                                        WidgetBuilder::new()
                                            .on_row(0)
                                            .on_column(2)
                                            .with_margin(Thickness::uniform(1.0)),
                                    )
                                    .with_text("...")
                                    .build(ctx);
                                    select_work_dir
                                })
                                .with_child(
                                    TextBuilder::new(
                                        WidgetBuilder::new()
                                            .on_row(1)
                                            .on_column(0)
                                            .with_margin(Thickness::uniform(1.0))
                                            .with_vertical_alignment(VerticalAlignment::Center),
                                    )
                                    .with_text("Textures Directory")
                                    .build(ctx),
                                )
                                .with_child({
                                    tb_textures_path = TextBoxBuilder::new(
                                        WidgetBuilder::new()
                                            .on_row(1)
                                            .on_column(1)
                                            .with_margin(Thickness::uniform(1.0)),
                                    )
                                    .with_vertical_text_alignment(VerticalAlignment::Center)
                                    .build(ctx);
                                    tb_textures_path
                                })
                                .with_child({
                                    select_textures_dir = ButtonBuilder::new(
                                        WidgetBuilder::new()
                                            .on_row(1)
                                            .on_column(2)
                                            .with_margin(Thickness::uniform(1.0)),
                                    )
                                    .with_text("...")
                                    .build(ctx);
                                    select_textures_dir
                                }),
                        )
                        .add_row(Row::strict(25.0))
                        .add_row(Row::strict(25.0))
                        .add_column(Column::strict(120.0))
                        .add_column(Column::stretch())
                        .add_column(Column::strict(25.0))
                        .build(ctx),
                    )
                    .with_child(
                        TextBuilder::new(
                            WidgetBuilder::new()
                                .with_margin(Thickness::uniform(5.0))
                                .on_row(2),
                        )
                        .with_text("Previous Configurations")
                        .with_horizontal_text_alignment(HorizontalAlignment::Center)
                        .build(ctx),
                    )
                    .with_child({
                        lv_history = ListViewBuilder::new(WidgetBuilder::new().on_row(3))
                            .with_items(
                                history
                                    .iter()
                                    .map(|entry| make_history_entry_widget(ctx, entry))
                                    .collect(),
                            )
                            .build(ctx);
                        lv_history
                    })
                    .with_child(
                        StackPanelBuilder::new(
                            WidgetBuilder::new()
                                .on_row(4)
                                .with_horizontal_alignment(HorizontalAlignment::Right)
                                .with_vertical_alignment(VerticalAlignment::Bottom)
                                .with_child({
                                    ok = ButtonBuilder::new(
                                        WidgetBuilder::new()
                                            .with_enabled(false) // Disabled by default.
                                            .with_width(80.0)
                                            .with_height(25.0)
                                            .with_margin(Thickness::uniform(1.0)),
                                    )
                                    .with_text("OK")
                                    .build(ctx);
                                    ok
                                }),
                        )
                        .with_orientation(Orientation::Horizontal)
                        .build(ctx),
                    ),
            )
            .add_row(Row::auto())
            .add_row(Row::auto())
            .add_row(Row::auto())
            .add_row(Row::strict(80.0))
            .add_row(Row::stretch())
            .add_column(Column::stretch())
            .build(ctx),
        )
        .build(ctx);

        Self {
            window,
            textures_dir_browser: scene_browser,
            work_dir_browser: folder_browser,
            select_work_dir,
            select_textures_dir,
            ok,
            sender,
            tb_work_dir,
            tb_textures_path,
            work_dir: Default::default(),
            textures_path: Default::default(),
            lv_history,
            history,
        }
    }

    fn validate(&mut self, engine: &mut GameEngine) {
        let is_valid_scene_path = self.textures_path.exists()
            && self.work_dir.exists()
            && self.textures_path.starts_with(&self.work_dir);
        engine.user_interface.send_message(WidgetMessage::enabled(
            self.ok,
            MessageDirection::ToWidget,
            is_valid_scene_path,
        ));
    }

    pub fn handle_ui_message(&mut self, message: &UiMessage, engine: &mut GameEngine) {
        scope_profile!();

        match message.data() {
            UiMessageData::Window(msg) => {
                if message.destination() == self.window {
                    if let WindowMessage::Close = msg {
                        // Save history for next editor runs.
                        let mut visitor = Visitor::new();
                        self.history.visit("History", &mut visitor).unwrap();
                        visitor
                            .save_binary(STARTUP_WORKING_DIR.lock().unwrap().join(HISTORY_PATH))
                            .unwrap();
                    }
                }
            }
            UiMessageData::ListView(msg) => {
                if message.destination() == self.lv_history
                    && message.direction() == MessageDirection::FromWidget
                {
                    if let ListViewMessage::SelectionChanged(Some(index)) = *msg {
                        let entry = &self.history[index];
                        self.textures_path = entry.textures_path.clone();
                        self.work_dir = entry.work_dir.clone();

                        engine.user_interface.send_message(TextBoxMessage::text(
                            self.tb_textures_path,
                            MessageDirection::ToWidget,
                            self.textures_path.to_string_lossy().to_string(),
                        ));
                        engine.user_interface.send_message(TextBoxMessage::text(
                            self.tb_work_dir,
                            MessageDirection::ToWidget,
                            self.work_dir.to_string_lossy().to_string(),
                        ));

                        self.validate(engine);
                    }
                }
            }
            UiMessageData::FileSelector(FileSelectorMessage::Commit(path)) => {
                if message.destination() == self.textures_dir_browser {
                    if let Ok(textures_path) = path.clone().canonicalize() {
                        self.textures_path = textures_path;
                        engine.user_interface.send_message(TextBoxMessage::text(
                            self.tb_textures_path,
                            MessageDirection::ToWidget,
                            self.textures_path.to_string_lossy().to_string(),
                        ));

                        self.validate(engine);
                    }
                } else if message.destination() == self.work_dir_browser {
                    if let Ok(work_dir) = path.clone().canonicalize() {
                        self.work_dir = work_dir;
                        self.textures_path = self.work_dir.clone();
                        engine.user_interface.send_message(TextBoxMessage::text(
                            self.tb_work_dir,
                            MessageDirection::ToWidget,
                            self.work_dir.to_string_lossy().to_string(),
                        ));
                        engine
                            .user_interface
                            .send_message(FileSelectorMessage::root(
                                self.textures_dir_browser,
                                MessageDirection::ToWidget,
                                Some(self.textures_path.clone()),
                            ));
                        engine.user_interface.send_message(TextBoxMessage::text(
                            self.tb_textures_path,
                            MessageDirection::ToWidget,
                            self.textures_path.to_string_lossy().to_string(),
                        ));

                        self.validate(engine);
                    }
                }
            }
            UiMessageData::Button(ButtonMessage::Click) => {
                if message.destination() == self.ok {
                    self.sender
                        .send(Message::Configure {
                            working_directory: self.work_dir.clone(),
                            textures_path: self.textures_path.clone(),
                        })
                        .unwrap();

                    let new_entry = HistoryEntry {
                        work_dir: self.work_dir.clone(),
                        textures_path: self.textures_path.clone(),
                    };
                    if self.history.iter().position(|e| e == &new_entry).is_none() {
                        self.history.push(new_entry);

                        let widget = make_history_entry_widget(
                            &mut engine.user_interface.build_ctx(),
                            self.history.last().unwrap(),
                        );

                        engine
                            .user_interface
                            .send_message(ListViewMessage::add_item(
                                self.lv_history,
                                MessageDirection::ToWidget,
                                widget,
                            ));
                    }

                    engine.user_interface.send_message(WindowMessage::close(
                        self.window,
                        MessageDirection::ToWidget,
                    ));
                } else if message.destination() == self.select_textures_dir {
                    engine
                        .user_interface
                        .send_message(WindowMessage::open_modal(
                            self.textures_dir_browser,
                            MessageDirection::ToWidget,
                            true,
                        ));
                    if self.work_dir.exists() {
                        // Once working directory was selected we can reduce amount of clicks
                        // for user by setting initial path of scene selector to working dir.
                        engine
                            .user_interface
                            .send_message(FileSelectorMessage::path(
                                self.textures_dir_browser,
                                MessageDirection::ToWidget,
                                self.work_dir.clone(),
                            ));
                    }
                } else if message.destination() == self.select_work_dir {
                    engine
                        .user_interface
                        .send_message(WindowMessage::open_modal(
                            self.work_dir_browser,
                            MessageDirection::ToWidget,
                            true,
                        ));
                }
            }
            _ => {}
        }
    }
}
