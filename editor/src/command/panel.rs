use crate::utils::window_content;
use crate::{
    command::CommandStack, load_image, scene::commands::SceneContext, send_sync_message, Message,
    Mode,
};
use fyrox::gui::widget::WidgetMessage;
use fyrox::{
    core::{color::Color, pool::Handle, scope_profile},
    gui::{
        brush::Brush,
        button::{ButtonBuilder, ButtonMessage},
        grid::{Column, GridBuilder, Row},
        image::ImageBuilder,
        list_view::{ListViewBuilder, ListViewMessage},
        message::{MessageDirection, UiMessage},
        scroll_viewer::ScrollViewerBuilder,
        stack_panel::StackPanelBuilder,
        text::TextBuilder,
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowTitle},
        BuildContext, Orientation, Thickness, UiNode, UserInterface,
    },
};
use std::sync::mpsc::Sender;

pub struct CommandStackViewer {
    pub window: Handle<UiNode>,
    list: Handle<UiNode>,
    sender: Sender<Message>,
    undo: Handle<UiNode>,
    redo: Handle<UiNode>,
    clear: Handle<UiNode>,
}

impl CommandStackViewer {
    pub fn new(ctx: &mut BuildContext, sender: Sender<Message>) -> Self {
        let list;
        let undo;
        let redo;
        let clear;
        let window = WindowBuilder::new(WidgetBuilder::new())
            .with_title(WindowTitle::Text("Command Stack".to_owned()))
            .with_content(
                GridBuilder::new(
                    WidgetBuilder::new()
                        .with_child(
                            StackPanelBuilder::new(
                                WidgetBuilder::new()
                                    .with_child({
                                        undo = ButtonBuilder::new(
                                            WidgetBuilder::new()
                                                .with_margin(Thickness::uniform(1.0)),
                                        )
                                        .with_content(
                                            ImageBuilder::new(
                                                WidgetBuilder::new()
                                                    .with_margin(Thickness::uniform(1.0))
                                                    .with_height(28.0)
                                                    .with_width(28.0),
                                            )
                                            .with_opt_texture(load_image(include_bytes!(
                                                "../../resources/embed/undo.png"
                                            )))
                                            .build(ctx),
                                        )
                                        .build(ctx);
                                        undo
                                    })
                                    .with_child({
                                        redo = ButtonBuilder::new(
                                            WidgetBuilder::new()
                                                .with_margin(Thickness::uniform(1.0)),
                                        )
                                        .with_content(
                                            ImageBuilder::new(
                                                WidgetBuilder::new()
                                                    .with_margin(Thickness::uniform(1.0))
                                                    .with_height(28.0)
                                                    .with_width(28.0),
                                            )
                                            .with_opt_texture(load_image(include_bytes!(
                                                "../../resources/embed/redo.png"
                                            )))
                                            .build(ctx),
                                        )
                                        .build(ctx);
                                        redo
                                    })
                                    .with_child({
                                        clear = ButtonBuilder::new(WidgetBuilder::new())
                                            .with_content(
                                                ImageBuilder::new(
                                                    WidgetBuilder::new()
                                                        .with_margin(Thickness::uniform(1.0))
                                                        .with_height(28.0)
                                                        .with_width(28.0),
                                                )
                                                .with_opt_texture(load_image(include_bytes!(
                                                    "../../resources/embed/clear.png"
                                                )))
                                                .build(ctx),
                                            )
                                            .build(ctx);
                                        clear
                                    }),
                            )
                            .with_orientation(Orientation::Horizontal)
                            .build(ctx),
                        )
                        .with_child(
                            ScrollViewerBuilder::new(
                                WidgetBuilder::new()
                                    .with_margin(Thickness::uniform(1.0))
                                    .on_row(1),
                            )
                            .with_content({
                                list = ListViewBuilder::new(WidgetBuilder::new()).build(ctx);
                                list
                            })
                            .build(ctx),
                        ),
                )
                .add_column(Column::stretch())
                .add_row(Row::strict(30.0))
                .add_row(Row::stretch())
                .build(ctx),
            )
            .build(ctx);

        Self {
            window,
            list,
            sender,
            undo,
            redo,
            clear,
        }
    }

    pub fn handle_ui_message(&self, message: &UiMessage) {
        scope_profile!();

        if let Some(ButtonMessage::Click) = message.data::<ButtonMessage>() {
            if message.destination() == self.undo {
                self.sender.send(Message::UndoSceneCommand).unwrap();
            } else if message.destination() == self.redo {
                self.sender.send(Message::RedoSceneCommand).unwrap();
            } else if message.destination() == self.clear {
                self.sender.send(Message::ClearSceneCommandStack).unwrap();
            }
        }
    }

    pub fn sync_to_model(
        &mut self,
        command_stack: &mut CommandStack,
        ctx: &SceneContext,
        ui: &mut UserInterface,
    ) {
        scope_profile!();

        let top = command_stack.top;
        let items = command_stack
            .commands
            .iter_mut()
            .enumerate()
            .rev() // First command in list is last on stack.
            .map(|(i, cmd)| {
                let brush = if let Some(top) = top {
                    if (0..=top).contains(&i) {
                        Brush::Solid(Color::opaque(255, 255, 255))
                    } else {
                        Brush::Solid(Color::opaque(100, 100, 100))
                    }
                } else {
                    Brush::Solid(Color::opaque(100, 100, 100))
                };

                TextBuilder::new(WidgetBuilder::new().with_foreground(brush))
                    .with_text(cmd.name(ctx))
                    .build(&mut ui.build_ctx())
            })
            .collect();

        send_sync_message(
            ui,
            ListViewMessage::items(self.list, MessageDirection::ToWidget, items),
        );
    }

    pub fn on_mode_changed(&mut self, ui: &UserInterface, mode: &Mode) {
        ui.send_message(WidgetMessage::enabled(
            window_content(self.window, ui),
            MessageDirection::ToWidget,
            mode.is_edit(),
        ));
    }
}
