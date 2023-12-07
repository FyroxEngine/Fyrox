use crate::message::MessageSender;
use crate::{
    command::GameSceneCommandStack, gui::make_image_button_with_tooltip, load_image,
    scene::commands::GameSceneContext, send_sync_message, utils::window_content, Message, Mode,
};
use fyrox::{
    core::{color::Color, pool::Handle, scope_profile},
    gui::{
        brush::Brush,
        button::ButtonMessage,
        grid::{Column, GridBuilder, Row},
        list_view::{ListViewBuilder, ListViewMessage},
        message::{MessageDirection, UiMessage},
        scroll_viewer::ScrollViewerBuilder,
        stack_panel::StackPanelBuilder,
        text::TextBuilder,
        widget::{WidgetBuilder, WidgetMessage},
        window::{WindowBuilder, WindowTitle},
        BuildContext, Orientation, Thickness, UiNode, UserInterface,
    },
};

pub struct CommandStackViewer {
    pub window: Handle<UiNode>,
    list: Handle<UiNode>,
    sender: MessageSender,
    undo: Handle<UiNode>,
    redo: Handle<UiNode>,
    clear: Handle<UiNode>,
}

impl CommandStackViewer {
    pub fn new(ctx: &mut BuildContext, sender: MessageSender) -> Self {
        let list;
        let undo;
        let redo;
        let clear;
        let window = WindowBuilder::new(WidgetBuilder::new().with_name("CommandStackPanel"))
            .with_title(WindowTitle::Text("Command Stack".to_owned()))
            .with_content(
                GridBuilder::new(
                    WidgetBuilder::new()
                        .with_child(
                            StackPanelBuilder::new(
                                WidgetBuilder::new()
                                    .with_child({
                                        undo = make_image_button_with_tooltip(
                                            ctx,
                                            20.0,
                                            20.0,
                                            load_image(include_bytes!("../../resources/undo.png")),
                                            "Undo The Command",
                                        );
                                        undo
                                    })
                                    .with_child({
                                        redo = make_image_button_with_tooltip(
                                            ctx,
                                            20.0,
                                            20.0,
                                            load_image(include_bytes!("../../resources/redo.png")),
                                            "Redo The Command",
                                        );
                                        redo
                                    })
                                    .with_child({
                                        clear = make_image_button_with_tooltip(
                                            ctx,
                                            20.0,
                                            20.0,
                                            load_image(include_bytes!("../../resources/clear.png")),
                                            "Clear Command Stack\nChanges history will be erased.",
                                        );
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
                .add_row(Row::strict(26.0))
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
                self.sender.send(Message::UndoCurrentSceneCommand);
            } else if message.destination() == self.redo {
                self.sender.send(Message::RedoCurrentSceneCommand);
            } else if message.destination() == self.clear {
                self.sender.send(Message::ClearCurrentSceneCommandStack);
            }
        }
    }

    pub fn sync_to_model(
        &mut self,
        command_stack: &mut GameSceneCommandStack,
        ctx: &GameSceneContext,
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

                TextBuilder::new(
                    WidgetBuilder::new()
                        .with_margin(Thickness {
                            left: 2.0,
                            top: 1.0,
                            right: 2.0,
                            bottom: 0.0,
                        })
                        .with_foreground(brush),
                )
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
