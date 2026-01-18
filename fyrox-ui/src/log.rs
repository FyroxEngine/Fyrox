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

use crate::button::Button;
use crate::scroll_viewer::ScrollViewer;
use crate::stack_panel::StackPanel;
use crate::window::Window;
use crate::{
    border::BorderBuilder,
    button::ButtonMessage,
    copypasta::ClipboardProvider,
    core::{
        log::{LogMessage, MessageKind},
        pool::Handle,
    },
    dropdown_list::{DropdownListBuilder, DropdownListMessage},
    grid::{Column, GridBuilder, Row},
    menu::{ContextMenuBuilder, MenuItemBuilder, MenuItemContent, MenuItemMessage},
    message::UiMessage,
    popup::{Placement, PopupBuilder, PopupMessage},
    scroll_viewer::{ScrollViewerBuilder, ScrollViewerMessage},
    stack_panel::StackPanelBuilder,
    style::{resource::StyleResourceExt, Style},
    text::{Text, TextBuilder},
    utils::{make_dropdown_list_option, make_image_button_with_tooltip},
    widget::{WidgetBuilder, WidgetMessage},
    window::{WindowAlignment, WindowBuilder, WindowMessage, WindowTitle},
    BuildContext, HorizontalAlignment, Orientation, RcUiNodeHandle, Thickness, UiNode,
    UserInterface, VerticalAlignment,
};
use fyrox_graph::SceneGraph;
use fyrox_texture::TextureResource;
use std::sync::mpsc::Receiver;

struct ContextMenu {
    menu: RcUiNodeHandle,
    copy: Handle<UiNode>,
    placement_target: Handle<UiNode>,
}

impl ContextMenu {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let copy;
        let menu = ContextMenuBuilder::new(
            PopupBuilder::new(WidgetBuilder::new())
                .with_content(
                    StackPanelBuilder::new(WidgetBuilder::new().with_child({
                        copy = MenuItemBuilder::new(WidgetBuilder::new())
                            .with_content(MenuItemContent::text("Copy"))
                            .build(ctx);
                        copy
                    }))
                    .build(ctx),
                )
                .with_restrict_picking(false),
        )
        .build(ctx);
        let menu = RcUiNodeHandle::new(menu, ctx.sender());

        Self {
            menu,
            copy,
            placement_target: Default::default(),
        }
    }

    fn on_copy_clicked(&self, ui: &mut UserInterface) -> Option<()> {
        let text = ui.find_component::<Text>(self.placement_target)?.1.text();
        ui.clipboard_mut()?.set_contents(text).ok()
    }

    pub fn handle_ui_message(&mut self, message: &UiMessage, ui: &mut UserInterface) {
        if let Some(PopupMessage::Placement(Placement::Cursor(target))) =
            message.data_from(self.menu.handle())
        {
            self.placement_target = *target;
        } else if let Some(MenuItemMessage::Click) = message.data_from(self.copy) {
            self.on_copy_clicked(ui);
        }
    }
}

pub struct LogPanel {
    pub window: Handle<Window>,
    messages: Handle<StackPanel>,
    clear: Handle<Button>,
    receiver: Receiver<LogMessage>,
    severity: MessageKind,
    severity_list: Handle<UiNode>,
    context_menu: ContextMenu,
    scroll_viewer: Handle<ScrollViewer>,
    pub message_count: usize,
}

impl LogPanel {
    pub fn new(
        ctx: &mut BuildContext,
        message_receiver: Receiver<LogMessage>,
        clear_icon: Option<TextureResource>,
        open: bool,
    ) -> Self {
        let messages;
        let clear;
        let severity_list;
        let scroll_viewer;
        let window = WindowBuilder::new(
            WidgetBuilder::new()
                .with_width(400.0)
                .with_height(200.0)
                .with_name("LogPanel"),
        )
        .can_minimize(false)
        .open(open)
        .with_title(WindowTitle::text("Message Log"))
        .with_tab_label("Log")
        .with_content(
            GridBuilder::new(
                WidgetBuilder::new()
                    .with_child(
                        StackPanelBuilder::new(
                            WidgetBuilder::new()
                                .with_horizontal_alignment(HorizontalAlignment::Left)
                                .on_row(0)
                                .on_column(0)
                                .with_child({
                                    clear = make_image_button_with_tooltip(
                                        ctx,
                                        18.0,
                                        18.0,
                                        clear_icon,
                                        "Clear the log.",
                                        Some(0),
                                    );
                                    clear
                                })
                                .with_child({
                                    severity_list = DropdownListBuilder::new(
                                        WidgetBuilder::new()
                                            .with_tab_index(Some(1))
                                            .with_width(120.0)
                                            .with_margin(Thickness::uniform(1.0)),
                                    )
                                    .with_items(vec![
                                        make_dropdown_list_option(ctx, "Info+"),
                                        make_dropdown_list_option(ctx, "Warnings+"),
                                        make_dropdown_list_option(ctx, "Errors"),
                                    ])
                                    // Warnings+
                                    .with_selected(1)
                                    .build(ctx);
                                    severity_list
                                }),
                        )
                        .with_orientation(Orientation::Horizontal)
                        .build(ctx),
                    )
                    .with_child({
                        scroll_viewer = ScrollViewerBuilder::new(
                            WidgetBuilder::new()
                                .on_row(1)
                                .on_column(0)
                                .with_margin(Thickness::uniform(3.0)),
                        )
                        .with_content({
                            messages = StackPanelBuilder::new(
                                WidgetBuilder::new().with_margin(Thickness::uniform(1.0)),
                            )
                            .build(ctx);
                            messages
                        })
                        .with_horizontal_scroll_allowed(true)
                        .with_vertical_scroll_allowed(true)
                        .build(ctx);
                        scroll_viewer
                    }),
            )
            .add_row(Row::auto())
            .add_row(Row::stretch())
            .add_column(Column::stretch())
            .build(ctx),
        )
        .build(ctx);

        let context_menu = ContextMenu::new(ctx);

        Self {
            window,
            messages,
            clear,
            receiver: message_receiver,
            severity: MessageKind::Warning,
            severity_list,
            context_menu,
            message_count: 0,
            scroll_viewer,
        }
    }

    pub fn destroy(self, ui: &UserInterface) {
        ui.send(self.context_menu.menu.handle(), WidgetMessage::Remove);
        ui.send(self.window, WidgetMessage::Remove);
    }

    pub fn open(&self, ui: &UserInterface) {
        ui.send(
            self.window,
            WindowMessage::Open {
                alignment: WindowAlignment::Center,
                modal: false,
                focus_content: true,
            },
        );
    }

    pub fn close(&self, ui: &UserInterface) {
        ui.send(self.window, WindowMessage::Close);
    }

    pub fn handle_ui_message(&mut self, message: &UiMessage, ui: &mut UserInterface) {
        if let Some(ButtonMessage::Click) = message.data_from(self.clear) {
            ui.send(self.messages, WidgetMessage::ReplaceChildren(vec![]));
        } else if let Some(DropdownListMessage::Selection(Some(idx))) =
            message.data_from(self.severity_list)
        {
            match idx {
                0 => self.severity = MessageKind::Information,
                1 => self.severity = MessageKind::Warning,
                2 => self.severity = MessageKind::Error,
                _ => (),
            };
        }

        self.context_menu.handle_ui_message(message, ui);
    }

    pub fn update(&mut self, max_log_entries: usize, ui: &mut UserInterface) -> bool {
        let existing_items = ui[self.messages].children();

        let mut count = existing_items.len();

        if count > max_log_entries {
            let delta = count - max_log_entries;
            // Remove every item in the head of the list of entries to keep the amount of entries
            // in the limits.
            //
            // TODO: This is suboptimal, because it creates a message per each excessive entry, which
            //  might be slow to process in case of large amount of messages.
            for item in existing_items.iter().take(delta) {
                ui.send(*item, WidgetMessage::Remove);
            }

            count -= delta;
        }

        let mut item_to_bring_into_view = Handle::NONE;

        let mut received_anything = false;

        while let Ok(msg) = self.receiver.try_recv() {
            if msg.kind < self.severity {
                continue;
            }

            self.message_count += 1;
            received_anything = true;

            let mut text = format!("[{:.2}s] {}", msg.time.as_secs_f32(), msg.content);
            if let Some(ch) = text.chars().last() {
                if ch == '\n' {
                    text.pop();
                }
            }

            let ctx = &mut ui.build_ctx();
            let item = BorderBuilder::new(
                WidgetBuilder::new()
                    .with_context_menu(self.context_menu.menu.clone())
                    .with_background(if count.is_multiple_of(2) {
                        ctx.style.property(Style::BRUSH_LIGHT)
                    } else {
                        ctx.style.property(Style::BRUSH_DARK)
                    })
                    .with_child(
                        TextBuilder::new(
                            WidgetBuilder::new()
                                .with_margin(Thickness::uniform(2.0))
                                .with_foreground(match msg.kind {
                                    MessageKind::Information => {
                                        ctx.style.property(Style::BRUSH_INFORMATION)
                                    }
                                    MessageKind::Warning => {
                                        ctx.style.property(Style::BRUSH_WARNING)
                                    }
                                    MessageKind::Error => ctx.style.property(Style::BRUSH_ERROR),
                                }),
                        )
                        .with_vertical_text_alignment(VerticalAlignment::Center)
                        .with_text(text)
                        .build(ctx),
                    ),
            )
            .build(ctx);

            ui.send(item, WidgetMessage::link_with(self.messages));

            item_to_bring_into_view = item;

            count += 1;
        }

        if item_to_bring_into_view.is_some() {
            ui.send(
                self.scroll_viewer,
                ScrollViewerMessage::BringIntoView(item_to_bring_into_view.to_base()),
            );
        }

        received_anything
    }
}
