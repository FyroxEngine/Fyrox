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

use crate::fyrox::{
    core::pool::Handle,
    gui::{
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
use crate::{
    load_image, message::MessageSender, send_sync_message, utils::window_content, Message, Mode,
};
use fyrox::gui::style::resource::StyleResourceExt;
use fyrox::gui::style::Style;
use fyrox::gui::utils::make_image_button_with_tooltip;

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
            .with_title(WindowTitle::text("Command Stack"))
            .with_tab_label("Commands")
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
                                            load_image!("../../resources/undo.png"),
                                            "Undo The Command",
                                            Some(0),
                                        );
                                        undo
                                    })
                                    .with_child({
                                        redo = make_image_button_with_tooltip(
                                            ctx,
                                            20.0,
                                            20.0,
                                            load_image!("../../resources/redo.png"),
                                            "Redo The Command",
                                            Some(1),
                                        );
                                        redo
                                    })
                                    .with_child({
                                        clear = make_image_button_with_tooltip(
                                            ctx,
                                            20.0,
                                            20.0,
                                            load_image!("../../resources/clear.png"),
                                            "Clear Command Stack\nChanges history will be erased.",
                                            Some(2),
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
        top: Option<usize>,
        command_names: Vec<String>,
        ui: &mut UserInterface,
    ) {
        let items = command_names
            .into_iter()
            .enumerate()
            .rev() // First command in list is last on stack.
            .map(|(i, name)| {
                let brush = if let Some(top) = top {
                    if (0..=top).contains(&i) {
                        ui.style.property(Style::BRUSH_TEXT)
                    } else {
                        ui.style.property(Style::BRUSH_LIGHTEST)
                    }
                } else {
                    ui.style.property(Style::BRUSH_LIGHTEST)
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
                .with_text(name)
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
