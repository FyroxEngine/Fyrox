use crate::fyrox::gui::text::TextMessage;
use crate::fyrox::{
    core::{
        algebra::Vector2,
        pool::Handle,
        scope_profile,
        visitor::{Visit, VisitResult, Visitor},
    },
    gui::{
        border::BorderBuilder,
        button::{ButtonBuilder, ButtonMessage},
        decorator::DecoratorBuilder,
        file_browser::{FileSelectorBuilder, FileSelectorMessage, Filter},
        formatted_text::WrapMode,
        grid::{Column, GridBuilder, Row},
        list_view::{ListViewBuilder, ListViewMessage},
        message::{MessageDirection, UiMessage},
        stack_panel::StackPanelBuilder,
        text::TextBuilder,
        text_box::TextBoxBuilder,
        widget::{WidgetBuilder, WidgetMessage},
        window::{WindowBuilder, WindowMessage, WindowTitle},
        BuildContext, HorizontalAlignment, Orientation, Thickness, UiNode, VerticalAlignment,
    },
};
use crate::message::MessageSender;
use crate::{Engine, Message};
use std::{
    env,
    path::{Path, PathBuf},
};

#[derive(Default, Eq, PartialEq, Visit)]
struct HistoryEntry {
    work_dir: PathBuf,
}

pub const HISTORY_PATH: &str = "history.bin";

pub struct Configurator {
    pub window: Handle<UiNode>,
    work_dir_browser: Handle<UiNode>,
    select_work_dir: Handle<UiNode>,
    ok: Handle<UiNode>,
    sender: MessageSender,
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
                TextBuilder::new(WidgetBuilder::new().with_margin(Thickness::left(5.0)))
                    .with_text(format!("{}", entry.work_dir.display(),))
                    .with_vertical_text_alignment(VerticalAlignment::Center)
                    .build(ctx),
            ),
    ))
    .build(ctx)
}

impl Configurator {
    pub fn new(sender: MessageSender, ctx: &mut BuildContext) -> Self {
        let select_work_dir;
        let ok;
        let tb_work_dir;

        let current_path = env::current_dir().unwrap();

        let filter = Filter::new(|p: &Path| p.is_dir());

        let folder_browser = FileSelectorBuilder::new(
            WindowBuilder::new(WidgetBuilder::new().with_width(300.0).with_height(400.0))
                .open(false)
                .with_title(WindowTitle::text("Select Working Directory")),
        )
        .with_filter(filter)
        .build(ctx);

        // Load history.
        let mut history: Vec<HistoryEntry> = Vec::new();
        if let Ok(mut visitor) =
            fyrox::core::futures::executor::block_on(Visitor::load_binary(HISTORY_PATH))
        {
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
        .with_title(WindowTitle::text("Configure Editor"))
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
                                            .with_margin(Thickness::uniform(1.0))
                                            .with_enabled(false),
                                    )
                                    .with_text(
                                        (current_path.clone())
                                            .into_os_string()
                                            .into_string()
                                            .unwrap(),
                                    )
                                    .with_vertical_text_alignment(VerticalAlignment::Center)
                                    .build(ctx);
                                    tb_work_dir
                                })
                                .with_child({
                                    select_work_dir = ButtonBuilder::new(
                                        WidgetBuilder::new()
                                            .with_tab_index(Some(0))
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
                        lv_history = ListViewBuilder::new(
                            WidgetBuilder::new().with_tab_index(Some(1)).on_row(3),
                        )
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
                                            .with_tab_index(Some(2))
                                            .with_enabled(true) // Enabled by default.
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
            work_dir: current_path,
            lv_history,
            history,
        }
    }

    fn validate(&mut self, engine: &mut Engine) {
        let is_valid_scene_path = self.work_dir.exists();
        engine
            .user_interfaces
            .first_mut()
            .send_message(WidgetMessage::enabled(
                self.ok,
                MessageDirection::ToWidget,
                is_valid_scene_path,
            ));
    }

    pub fn handle_ui_message(&mut self, message: &UiMessage, engine: &mut Engine) {
        scope_profile!();

        if let Some(WindowMessage::Close) = message.data::<WindowMessage>() {
            if message.destination() == self.window {
                // Save history for next editor runs.
                let mut visitor = Visitor::new();
                self.history.visit("History", &mut visitor).unwrap();
                visitor.save_binary(HISTORY_PATH).unwrap();
            }
        } else if let Some(ListViewMessage::SelectionChanged(selected_indices)) =
            message.data::<ListViewMessage>()
        {
            if let Some(index) = selected_indices.first().cloned() {
                if message.destination() == self.lv_history
                    && message.direction() == MessageDirection::FromWidget
                {
                    let entry = &self.history[index];
                    self.work_dir.clone_from(&entry.work_dir);

                    engine
                        .user_interfaces
                        .first_mut()
                        .send_message(TextMessage::text(
                            self.tb_work_dir,
                            MessageDirection::ToWidget,
                            self.work_dir.to_string_lossy().to_string(),
                        ));

                    self.validate(engine);
                }
            }
        } else if let Some(FileSelectorMessage::Commit(path)) =
            message.data::<FileSelectorMessage>()
        {
            if message.destination() == self.work_dir_browser {
                if let Ok(work_dir) = path.clone().canonicalize() {
                    self.work_dir = work_dir;
                    engine
                        .user_interfaces
                        .first_mut()
                        .send_message(TextMessage::text(
                            self.tb_work_dir,
                            MessageDirection::ToWidget,
                            self.work_dir.to_string_lossy().to_string(),
                        ));

                    self.validate(engine);
                }
            }
        } else if let Some(ButtonMessage::Click) = message.data::<ButtonMessage>() {
            if message.destination() == self.ok {
                self.sender.send(Message::Configure {
                    working_directory: self.work_dir.clone(),
                });

                let new_entry = HistoryEntry {
                    work_dir: self.work_dir.clone(),
                };
                if !self.history.iter().any(|e| e == &new_entry) {
                    self.history.push(new_entry);

                    let widget = make_history_entry_widget(
                        &mut engine.user_interfaces.first_mut().build_ctx(),
                        self.history.last().unwrap(),
                    );

                    engine
                        .user_interfaces
                        .first_mut()
                        .send_message(ListViewMessage::add_item(
                            self.lv_history,
                            MessageDirection::ToWidget,
                            widget,
                        ));
                }

                engine
                    .user_interfaces
                    .first_mut()
                    .send_message(WindowMessage::close(
                        self.window,
                        MessageDirection::ToWidget,
                    ));
            } else if message.destination() == self.select_work_dir {
                engine
                    .user_interfaces
                    .first_mut()
                    .send_message(WindowMessage::open_modal(
                        self.work_dir_browser,
                        MessageDirection::ToWidget,
                        true,
                        true,
                    ));
            }
        }
    }
}
