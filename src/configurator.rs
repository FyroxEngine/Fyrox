use crate::{GameEngine, Message, CONFIG_DIR};
use rg3d::gui::message::UiMessage;
use rg3d::gui::{BuildContext, UiNode};
use rg3d::{
    core::{
        algebra::Vector2,
        pool::Handle,
        scope_profile,
        visitor::{Visit, VisitResult, Visitor},
    },
    gui::{
        border::BorderBuilder,
        button::ButtonBuilder,
        decorator::DecoratorBuilder,
        file_browser::{FileSelectorBuilder, Filter},
        formatted_text::WrapMode,
        grid::{Column, GridBuilder, Row},
        list_view::ListViewBuilder,
        message::{
            ButtonMessage, FileSelectorMessage, ListViewMessage, MessageDirection, TextBoxMessage,
            UiMessageData, WidgetMessage, WindowMessage,
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
    path::{Path, PathBuf},
    sync::mpsc::Sender,
};

#[derive(Default, Eq, PartialEq)]
struct HistoryEntry {
    work_dir: PathBuf,
}

impl Visit for HistoryEntry {
    fn visit(&mut self, name: &str, visitor: &mut Visitor) -> VisitResult {
        visitor.enter_region(name)?;

        self.work_dir.visit("WorkDir", visitor)?;

        visitor.leave_region()
    }
}

pub const HISTORY_PATH: &str = "history.bin";

pub struct Configurator {
    pub window: Handle<UiNode>,
    work_dir_browser: Handle<UiNode>,
    select_work_dir: Handle<UiNode>,
    ok: Handle<UiNode>,
    sender: Sender<Message>,
    work_dir: PathBuf,
    tb_work_dir: Handle<UiNode>,
    lv_history: Handle<UiNode>,
    history: Vec<HistoryEntry>,
}

fn make_history_entry_widget(ctx: &mut BuildContext, entry: &HistoryEntry) -> Handle<UiNode> {
    DecoratorBuilder::new(BorderBuilder::new(
        WidgetBuilder::new()
            .with_height(18.0)
            .with_margin(Thickness {
                left: 1.0,
                top: 0.0,
                right: 1.0,
                bottom: 1.0,
            })
            .with_child(
                TextBuilder::new(WidgetBuilder::new())
                    .with_text(format!("{}", entry.work_dir.display(),))
                    .with_vertical_text_alignment(VerticalAlignment::Center)
                    .build(ctx),
            ),
    ))
    .build(ctx)
}

impl Configurator {
    pub fn new(sender: Sender<Message>, ctx: &mut BuildContext) -> Self {
        let select_work_dir;
        let ok;
        let tb_work_dir;

        let filter = Filter::new(|p: &Path| p.is_dir());

        let folder_browser = FileSelectorBuilder::new(
            WindowBuilder::new(WidgetBuilder::new().with_width(300.0).with_height(400.0))
                .open(false)
                .with_title(WindowTitle::Text("Select Working Directory".into())),
        )
        .with_filter(filter)
        .build(ctx);

        // Load history.
        let mut history: Vec<HistoryEntry> = Vec::new();
        if let Ok(mut visitor) = rg3d::core::futures::executor::block_on(Visitor::load_binary(
            CONFIG_DIR.lock().unwrap().join(HISTORY_PATH),
        )) {
            history.visit("History", &mut visitor).unwrap();
        }

        // Remove entries with invalid paths.
        history = history
            .into_iter()
            .filter(|e| e.work_dir.exists())
            .collect::<Vec<_>>();

        let message = "Please select the working directory of \
        your current project. In most cases it will be the root folder \
        of your project";

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
                            .with_wrap(WrapMode::Word)
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
                                }),
                        )
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
            work_dir_browser: folder_browser,
            select_work_dir,
            ok,
            sender,
            tb_work_dir,
            work_dir: Default::default(),
            lv_history,
            history,
        }
    }

    fn validate(&mut self, engine: &mut GameEngine) {
        let is_valid_scene_path = self.work_dir.exists();
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
                            .save_binary(CONFIG_DIR.lock().unwrap().join(HISTORY_PATH))
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
                        self.work_dir = entry.work_dir.clone();

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
                if message.destination() == self.work_dir_browser {
                    if let Ok(work_dir) = path.clone().canonicalize() {
                        self.work_dir = work_dir;
                        engine.user_interface.send_message(TextBoxMessage::text(
                            self.tb_work_dir,
                            MessageDirection::ToWidget,
                            self.work_dir.to_string_lossy().to_string(),
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
                        })
                        .unwrap();

                    let new_entry = HistoryEntry {
                        work_dir: self.work_dir.clone(),
                    };
                    if !self.history.iter().any(|e| e == &new_entry) {
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
