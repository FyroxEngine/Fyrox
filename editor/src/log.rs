use crate::fyrox::graph::BaseSceneGraph;
use crate::fyrox::{
    core::{
        log::{LogMessage, MessageKind},
        pool::Handle,
        scope_profile,
    },
    gui::{
        border::BorderBuilder,
        button::ButtonMessage,
        copypasta::ClipboardProvider,
        dropdown_list::DropdownListMessage,
        formatted_text::WrapMode,
        grid::{Column, GridBuilder, Row},
        list_view::{ListView, ListViewBuilder, ListViewMessage},
        menu::{MenuItemBuilder, MenuItemContent, MenuItemMessage},
        message::{MessageDirection, UiMessage},
        popup::{Placement, PopupBuilder, PopupMessage},
        scroll_viewer::ScrollViewerBuilder,
        stack_panel::StackPanelBuilder,
        text::{Text, TextBuilder},
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowTitle},
        BuildContext, HorizontalAlignment, Orientation, RcUiNodeHandle, Thickness, UiNode,
    },
};
use crate::{
    gui::{make_dropdown_list_option, make_image_button_with_tooltip},
    load_image, Brush, Color, DropdownListBuilder, Engine,
};
use fyrox::gui::menu::ContextMenuBuilder;
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

    pub fn handle_ui_message(&mut self, message: &UiMessage, engine: &mut Engine) {
        if let Some(PopupMessage::Placement(Placement::Cursor(target))) = message.data() {
            if message.destination() == self.menu.handle() {
                self.placement_target = *target;
            }
        } else if let Some(MenuItemMessage::Click) = message.data() {
            if message.destination() == self.copy {
                if let Some(field) = engine
                    .user_interfaces
                    .first_mut()
                    .try_get(self.placement_target)
                    .and_then(|n| n.query_component::<Text>())
                {
                    let text = field.text();
                    if let Some(mut clipboard) = engine.user_interfaces.first_mut().clipboard_mut()
                    {
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
}

impl LogPanel {
    pub fn new(ctx: &mut BuildContext, message_receiver: Receiver<LogMessage>) -> Self {
        let messages;
        let clear;
        let severity_list;
        let window = WindowBuilder::new(WidgetBuilder::new().with_name("LogPanel"))
            .can_minimize(false)
            .with_title(WindowTitle::text("Message Log"))
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
                                            load_image(include_bytes!("../resources/clear.png")),
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
        }
    }

    pub fn handle_ui_message(&mut self, message: &UiMessage, engine: &mut Engine) {
        scope_profile!();

        if let Some(ButtonMessage::Click) = message.data::<ButtonMessage>() {
            if message.destination() == self.clear {
                engine
                    .user_interfaces
                    .first_mut()
                    .send_message(ListViewMessage::items(
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

        self.context_menu.handle_ui_message(message, engine);
    }

    pub fn update(&mut self, engine: &mut Engine) {
        let mut count = engine
            .user_interfaces
            .first_mut()
            .node(self.messages)
            .cast::<ListView>()
            .map(|v| v.items().len())
            .unwrap_or_default();

        let mut item_to_bring_into_view = Handle::NONE;

        while let Ok(msg) = self.receiver.try_recv() {
            if msg.kind < self.severity {
                continue;
            }

            let text = format!("[{:.2}s] {}", msg.time.as_secs_f32(), msg.content);

            let ctx = &mut engine.user_interfaces.first_mut().build_ctx();
            let item = BorderBuilder::new(
                WidgetBuilder::new()
                    .with_background(Brush::Solid(if count % 2 == 0 {
                        Color::opaque(70, 70, 70)
                    } else {
                        Color::opaque(40, 40, 40)
                    }))
                    .with_child(
                        TextBuilder::new(
                            WidgetBuilder::new()
                                .with_context_menu(self.context_menu.menu.clone())
                                .with_margin(Thickness::uniform(1.0))
                                .with_foreground(Brush::Solid(match msg.kind {
                                    MessageKind::Information => Color::ANTIQUE_WHITE,
                                    MessageKind::Warning => Color::GOLD,
                                    MessageKind::Error => Color::RED,
                                })),
                        )
                        .with_text(text)
                        .with_wrap(WrapMode::Word)
                        .build(ctx),
                    ),
            )
            .build(ctx);

            engine
                .user_interfaces
                .first_mut()
                .send_message(ListViewMessage::add_item(
                    self.messages,
                    MessageDirection::ToWidget,
                    item,
                ));

            item_to_bring_into_view = item;

            count += 1;
        }

        if item_to_bring_into_view.is_some() {
            engine
                .user_interfaces
                .first_mut()
                .send_message(ListViewMessage::bring_item_into_view(
                    self.messages,
                    MessageDirection::ToWidget,
                    item_to_bring_into_view,
                ));
        }
    }
}
