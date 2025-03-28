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

use crate::{
    border::BorderBuilder,
    button::ButtonMessage,
    copypasta::ClipboardProvider,
    dropdown_list::{DropdownListBuilder, DropdownListMessage},
    formatted_text::WrapMode,
    grid::{Column, GridBuilder, Row},
    list_view::{ListView, ListViewBuilder, ListViewMessage},
    menu::{ContextMenuBuilder, MenuItemBuilder, MenuItemContent, MenuItemMessage},
    message::{MessageDirection, UiMessage},
    popup::{Placement, PopupBuilder, PopupMessage},
    scroll_viewer::ScrollViewerBuilder,
    stack_panel::StackPanelBuilder,
    style::{resource::StyleResourceExt, Style},
    text::{Text, TextBuilder},
    utils::{make_dropdown_list_option, make_image_button_with_tooltip},
    widget::{WidgetBuilder, WidgetMessage},
    window::{WindowBuilder, WindowMessage, WindowTitle},
    BuildContext, HorizontalAlignment, Orientation, RcUiNodeHandle, Thickness, UiNode,
    UserInterface,
};
use fyrox_core::{
    log::{LogMessage, MessageKind},
    pool::Handle,
};
use fyrox_graph::BaseSceneGraph;
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
            PopupBuilder::new(WidgetBuilder::new()).with_content(
                StackPanelBuilder::new(WidgetBuilder::new().with_child({
                    copy = MenuItemBuilder::new(WidgetBuilder::new())
                        .with_content(MenuItemContent::text("Copy"))
                        .build(ctx);
                    copy
                }))
                .build(ctx),
            ),
        )
        .build(ctx);
        let menu = RcUiNodeHandle::new(menu, ctx.sender());

        Self {
            menu,
            copy,
            placement_target: Default::default(),
        }
    }

    pub fn handle_ui_message(&mut self, message: &UiMessage, ui: &mut UserInterface) {
        if let Some(PopupMessage::Placement(Placement::Cursor(target))) = message.data() {
            if message.destination() == self.menu.handle() {
                self.placement_target = *target;
            }
        } else if let Some(MenuItemMessage::Click) = message.data() {
            if message.destination() == self.copy {
                if let Some(field) = ui
                    .try_get(self.placement_target)
                    .and_then(|n| n.query_component::<Text>())
                {
                    let text = field.text();
                    if let Some(mut clipboard) = ui.clipboard_mut() {
                        let _ = clipboard.set_contents(text);
                    }
                }
            }
        }
    }
}

pub struct LogPanel {
    pub window: Handle<UiNode>,
    messages: Handle<UiNode>,
    clear: Handle<UiNode>,
    receiver: Receiver<LogMessage>,
    severity: MessageKind,
    severity_list: Handle<UiNode>,
    context_menu: ContextMenu,
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
                                        24.0,
                                        24.0,
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
                        messages = ListViewBuilder::new(
                            WidgetBuilder::new()
                                .with_margin(Thickness::uniform(1.0))
                                .on_row(1)
                                .on_column(0),
                        )
                        .with_scroll_viewer(
                            ScrollViewerBuilder::new(
                                WidgetBuilder::new().with_margin(Thickness::uniform(3.0)),
                            )
                            .with_horizontal_scroll_allowed(true)
                            .with_vertical_scroll_allowed(true)
                            .build(ctx),
                        )
                        .build(ctx);
                        messages
                    }),
            )
            .add_row(Row::strict(26.0))
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
        }
    }

    pub fn destroy(self, ui: &UserInterface) {
        ui.send_message(WidgetMessage::remove(
            self.context_menu.menu.handle(),
            MessageDirection::ToWidget,
        ));
        ui.send_message(WidgetMessage::remove(
            self.window,
            MessageDirection::ToWidget,
        ));
    }

    pub fn open(&self, ui: &UserInterface) {
        ui.send_message(WindowMessage::open(
            self.window,
            MessageDirection::ToWidget,
            true,
            true,
        ));
    }

    pub fn close(&self, ui: &UserInterface) {
        ui.send_message(WindowMessage::close(
            self.window,
            MessageDirection::ToWidget,
        ));
    }

    pub fn handle_ui_message(&mut self, message: &UiMessage, ui: &mut UserInterface) {
        if let Some(ButtonMessage::Click) = message.data::<ButtonMessage>() {
            if message.destination() == self.clear {
                ui.send_message(ListViewMessage::items(
                    self.messages,
                    MessageDirection::ToWidget,
                    vec![],
                ));
            }
        } else if let Some(DropdownListMessage::SelectionChanged(Some(idx))) =
            message.data::<DropdownListMessage>()
        {
            if message.destination() == self.severity_list
                && message.direction() == MessageDirection::FromWidget
            {
                match idx {
                    0 => self.severity = MessageKind::Information,
                    1 => self.severity = MessageKind::Warning,
                    2 => self.severity = MessageKind::Error,
                    _ => (),
                };
            }
        }

        self.context_menu.handle_ui_message(message, ui);
    }

    pub fn update(&mut self, max_log_entries: usize, ui: &mut UserInterface) -> bool {
        let existing_items = ui
            .node(self.messages)
            .cast::<ListView>()
            .map(|v| v.items())
            .unwrap();

        let mut count = existing_items.len();

        if count > max_log_entries {
            let delta = count - max_log_entries;
            // Remove every item in the head of the list of entries to keep the amount of entries
            // in the limits.
            //
            // TODO: This is suboptimal, because it creates a message per each excessive entry, which
            //  might be slow to process in case of large amount of messages.
            for item in existing_items.iter().take(delta) {
                ui.send_message(ListViewMessage::remove_item(
                    self.messages,
                    MessageDirection::ToWidget,
                    *item,
                ));
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

            let text = format!("[{:.2}s] {}", msg.time.as_secs_f32(), msg.content);

            let ctx = &mut ui.build_ctx();
            let item = BorderBuilder::new(
                WidgetBuilder::new()
                    .with_background(if count % 2 == 0 {
                        ctx.style.property(Style::BRUSH_LIGHT)
                    } else {
                        ctx.style.property(Style::BRUSH_DARK)
                    })
                    .with_child(
                        TextBuilder::new(
                            WidgetBuilder::new()
                                .with_context_menu(self.context_menu.menu.clone())
                                .with_margin(Thickness::uniform(1.0))
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
                        .with_text(text)
                        .with_wrap(WrapMode::Word)
                        .build(ctx),
                    ),
            )
            .build(ctx);

            ui.send_message(ListViewMessage::add_item(
                self.messages,
                MessageDirection::ToWidget,
                item,
            ));

            item_to_bring_into_view = item;

            count += 1;
        }

        if item_to_bring_into_view.is_some() {
            ui.send_message(ListViewMessage::bring_item_into_view(
                self.messages,
                MessageDirection::ToWidget,
                item_to_bring_into_view,
            ));
        }

        received_anything
    }
}
