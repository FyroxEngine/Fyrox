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

use fyrox::gui::style::resource::StyleResourceExt;
use fyrox::gui::style::Style;
use fyrox::{
    core::{parking_lot::Mutex, pool::Handle},
    gui::{
        border::BorderBuilder,
        button::{ButtonBuilder, ButtonMessage},
        grid::{Column, GridBuilder, Row},
        message::{MessageDirection, UiMessage},
        scroll_viewer::{ScrollViewerBuilder, ScrollViewerMessage},
        stack_panel::StackPanelBuilder,
        text::{TextBuilder, TextMessage},
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowMessage, WindowTitle},
        BuildContext, HorizontalAlignment, Orientation, Thickness, UiNode, UserInterface,
    },
};
use std::{
    io::{BufRead, BufReader},
    process::ChildStderr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

pub struct BuildWindow {
    window: Handle<UiNode>,
    active: Arc<AtomicBool>,
    changed: Arc<AtomicBool>,
    log: Arc<Mutex<String>>,
    log_text: Handle<UiNode>,
    stop: Handle<UiNode>,
    scroll_viewer: Handle<UiNode>,
}

impl BuildWindow {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let log_text;
        let stop;
        let scroll_viewer;
        let window = WindowBuilder::new(WidgetBuilder::new().with_width(420.0).with_height(200.0))
            .can_minimize(false)
            .can_close(false)
            .open(false)
            .with_content(
                GridBuilder::new(
                    WidgetBuilder::new()
                        .with_child(
                            TextBuilder::new(
                                WidgetBuilder::new().with_margin(Thickness::uniform(1.0)),
                            )
                            .with_text("Please wait while your game is building...\nLog:")
                            .build(ctx),
                        )
                        .with_child(
                            BorderBuilder::new(
                                WidgetBuilder::new()
                                    .on_row(1)
                                    .with_margin(Thickness::uniform(2.0))
                                    .with_background(ctx.style.get_or_default(Style::BRUSH_DARKEST))
                                    .with_child({
                                        scroll_viewer =
                                            ScrollViewerBuilder::new(WidgetBuilder::new())
                                                .with_content({
                                                    log_text =
                                                        TextBuilder::new(WidgetBuilder::new())
                                                            .build(ctx);
                                                    log_text
                                                })
                                                .build(ctx);
                                        scroll_viewer
                                    }),
                            )
                            .build(ctx),
                        )
                        .with_child(
                            StackPanelBuilder::new(
                                WidgetBuilder::new()
                                    .with_horizontal_alignment(HorizontalAlignment::Right)
                                    .on_row(2)
                                    .with_child({
                                        stop = ButtonBuilder::new(
                                            WidgetBuilder::new()
                                                .with_width(100.0)
                                                .with_margin(Thickness::uniform(1.0)),
                                        )
                                        .with_text("Stop")
                                        .build(ctx);
                                        stop
                                    }),
                            )
                            .with_orientation(Orientation::Horizontal)
                            .build(ctx),
                        ),
                )
                .add_row(Row::auto())
                .add_row(Row::stretch())
                .add_row(Row::strict(28.0))
                .add_column(Column::stretch())
                .build(ctx),
            )
            .with_title(WindowTitle::text("Building the Game..."))
            .build(ctx);

        Self {
            window,
            log_text,
            log: Arc::new(Default::default()),
            active: Arc::new(AtomicBool::new(false)),
            changed: Arc::new(AtomicBool::new(false)),
            stop,
            scroll_viewer,
        }
    }

    pub fn listen(&mut self, mut stdout: ChildStderr, ui: &UserInterface) {
        let log = self.log.clone();
        self.active.store(true, Ordering::SeqCst);
        let reader_active = self.active.clone();
        let log_changed = self.changed.clone();
        std::thread::spawn(move || {
            while reader_active.load(Ordering::SeqCst) {
                for line in BufReader::new(&mut stdout).lines().take(10).flatten() {
                    let mut log_guard = log.lock();
                    log_guard.push_str(&line);
                    log_guard.push('\n');
                    log_changed.store(true, Ordering::SeqCst);
                }
            }
        });

        ui.send_message(WindowMessage::open_modal(
            self.window,
            MessageDirection::ToWidget,
            true,
            true,
        ));
    }

    pub fn reset(&mut self, ui: &UserInterface) {
        self.active.store(false, Ordering::SeqCst);
        self.changed.store(false, Ordering::SeqCst);
        self.log.lock().clear();
        ui.send_message(TextMessage::text(
            self.log_text,
            MessageDirection::ToWidget,
            Default::default(),
        ));
    }

    pub fn close(&mut self, ui: &UserInterface) {
        ui.send_message(WindowMessage::close(
            self.window,
            MessageDirection::ToWidget,
        ));
    }

    pub fn close_and_reset(&mut self, ui: &UserInterface) {
        self.reset(ui);
        self.close(ui);
    }

    pub fn update(&mut self, ui: &UserInterface) {
        if self.changed.load(Ordering::SeqCst) {
            ui.send_message(TextMessage::text(
                self.log_text,
                MessageDirection::ToWidget,
                self.log.lock().clone(),
            ));
            ui.send_message(ScrollViewerMessage::scroll_to_end(
                self.scroll_viewer,
                MessageDirection::ToWidget,
            ));

            self.changed.store(false, Ordering::SeqCst);
        }
    }

    pub fn handle_ui_message(&mut self, message: &UiMessage, ui: &UserInterface) {
        if let Some(ButtonMessage::Click) = message.data() {
            if message.destination() == self.stop {
                self.close_and_reset(ui);
            }
        }
    }
}
