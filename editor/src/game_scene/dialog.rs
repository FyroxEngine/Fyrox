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

use crate::command::Command;
use crate::{
    Message,
    fyrox::{
        core::pool::Handle,
        engine::Engine,
        gui::{
            BuildContext, HorizontalAlignment, Orientation, Thickness,
            button::{Button, ButtonBuilder, ButtonMessage},
            formatted_text::WrapMode,
            grid::{Column, GridBuilder, Row},
            message::UiMessage,
            scroll_viewer::ScrollViewerBuilder,
            stack_panel::StackPanelBuilder,
            text::{TextBuilder, TextMessage},
            text_box::{TextBox, TextBoxBuilder},
            widget::WidgetBuilder,
            window::{Window, WindowAlignment, WindowBuilder, WindowMessage, WindowTitle},
        },
    },
    message::MessageSender,
};

pub struct NodeRemovalDialog {
    pub window: Handle<Window>,
    info_text: Handle<TextBox>,
    ok: Handle<Button>,
    cancel: Handle<Button>,
    command: Option<Command>,
}

impl NodeRemovalDialog {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let info_text;
        let ok;
        let cancel;
        let text = "You're trying to delete scene node(s), that are referenced in some \
            other scene nodes, which may cause various issues in the engine or your \
            game. Are you sure you want to continue? You can always undo your changes. This \
            validation step could be disabled in editor settings.\n\n
            The full list of reference pairs is listed below:";
        let window = WindowBuilder::new(WidgetBuilder::new().with_width(400.0).with_height(500.0))
            .open(false)
            .with_title(WindowTitle::text("Node References"))
            .with_content(
                GridBuilder::new(
                    WidgetBuilder::new()
                        .with_child(
                            TextBuilder::new(
                                WidgetBuilder::new()
                                    .on_row(0)
                                    .with_margin(Thickness::uniform(4.0)),
                            )
                            .with_wrap(WrapMode::Word)
                            .with_text(text)
                            .build(ctx),
                        )
                        .with_child(
                            ScrollViewerBuilder::new(
                                WidgetBuilder::new()
                                    .on_row(1)
                                    .with_margin(Thickness::uniform(1.0)),
                            )
                            .with_content({
                                info_text = TextBoxBuilder::new(
                                    WidgetBuilder::new().with_margin(Thickness::uniform(1.0)),
                                )
                                .with_multiline(true)
                                .with_wrap(WrapMode::Word)
                                .build(ctx);
                                info_text
                            })
                            .build(ctx),
                        )
                        .with_child(
                            StackPanelBuilder::new(
                                WidgetBuilder::new()
                                    .with_horizontal_alignment(HorizontalAlignment::Right)
                                    .on_row(2)
                                    .with_margin(Thickness::uniform(1.0))
                                    .with_child({
                                        ok = ButtonBuilder::new(
                                            WidgetBuilder::new()
                                                .with_width(120.0)
                                                .with_height(24.0)
                                                .with_margin(Thickness::uniform(1.0)),
                                        )
                                        .with_text("Delete Anyway")
                                        .build(ctx);
                                        ok
                                    })
                                    .with_child({
                                        cancel = ButtonBuilder::new(
                                            WidgetBuilder::new()
                                                .with_width(120.0)
                                                .with_height(24.0)
                                                .with_margin(Thickness::uniform(1.0)),
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
                .add_row(Row::stretch())
                .add_row(Row::auto())
                .add_column(Column::stretch())
                .build(ctx),
            )
            .build(ctx);

        Self {
            info_text,
            window,
            ok,
            cancel,
            command: None,
        }
    }

    pub fn open(&mut self, delete_command: Command, refs: String, engine: &Engine) {
        let ui = &engine.user_interfaces.first();

        ui.send(
            self.window,
            WindowMessage::Open {
                alignment: WindowAlignment::Center,
                modal: true,
                focus_content: true,
            },
        );

        ui.send(self.info_text, TextMessage::Text(refs));

        self.command = Some(delete_command);
    }

    pub fn handle_ui_message(
        &mut self,
        message: &UiMessage,
        engine: &Engine,
        sender: &MessageSender,
    ) {
        let ui = &engine.user_interfaces.first();
        if let Some(ButtonMessage::Click) = message.data() {
            if message.destination() == self.ok {
                ui.send(self.window, WindowMessage::Close);

                if let Some(command) = self.command.take() {
                    sender.send(Message::DoCommand(command));
                }
            } else if message.destination() == self.cancel {
                ui.send(self.window, WindowMessage::Close);
            }
        }
    }
}
