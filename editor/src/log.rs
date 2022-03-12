use crate::{Brush, Color, GameEngine};
use fyrox::{
    core::{algebra::Vector2, pool::Handle, scope_profile},
    gui::{
        border::BorderBuilder,
        button::{ButtonBuilder, ButtonMessage},
        formatted_text::WrapMode,
        grid::{Column, GridBuilder, Row},
        list_view::{ListView, ListViewBuilder, ListViewMessage},
        message::{MessageDirection, UiMessage},
        scroll_viewer::ScrollViewerBuilder,
        stack_panel::StackPanelBuilder,
        text::TextBuilder,
        vector_image::{Primitive, VectorImageBuilder},
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowTitle},
        BuildContext, HorizontalAlignment, Thickness, UiNode, VerticalAlignment, BRUSH_BRIGHT,
    },
    utils::log::{LogMessage, MessageKind},
};
use std::sync::mpsc::Receiver;

pub struct LogPanel {
    pub window: Handle<UiNode>,
    messages: Handle<UiNode>,
    clear: Handle<UiNode>,
    receiver: Receiver<LogMessage>,
}

impl LogPanel {
    pub fn new(ctx: &mut BuildContext, message_receiver: Receiver<LogMessage>) -> Self {
        let messages;
        let clear;
        let window = WindowBuilder::new(WidgetBuilder::new())
            .can_minimize(false)
            .with_title(WindowTitle::Text("Message Log".to_owned()))
            .with_content(
                GridBuilder::new(
                    WidgetBuilder::new()
                        .with_child(
                            StackPanelBuilder::new(WidgetBuilder::new().with_child({
                                clear = ButtonBuilder::new(
                                    WidgetBuilder::new().with_margin(Thickness::uniform(1.0)),
                                )
                                .with_content(
                                    VectorImageBuilder::new(
                                        WidgetBuilder::new()
                                            .with_vertical_alignment(VerticalAlignment::Center)
                                            .with_horizontal_alignment(HorizontalAlignment::Center)
                                            .with_foreground(BRUSH_BRIGHT)
                                            .with_margin(Thickness::uniform(3.0)),
                                    )
                                    .with_primitives(vec![
                                        Primitive::Line {
                                            begin: Vector2::new(0.0, 0.0),
                                            end: Vector2::new(12.0, 12.0),
                                            thickness: 3.0,
                                        },
                                        Primitive::Line {
                                            begin: Vector2::new(12.0, 0.0),
                                            end: Vector2::new(0.0, 12.0),
                                            thickness: 3.0,
                                        },
                                    ])
                                    .build(ctx),
                                )
                                .build(ctx);
                                clear
                            }))
                            .build(ctx),
                        )
                        .with_child({
                            messages = ListViewBuilder::new(
                                WidgetBuilder::new()
                                    .with_margin(Thickness::uniform(1.0))
                                    .on_column(1),
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
                .add_row(Row::stretch())
                .add_column(Column::strict(23.0))
                .add_column(Column::stretch())
                .build(ctx),
            )
            .build(ctx);

        Self {
            window,
            messages,
            clear,
            receiver: message_receiver,
        }
    }

    pub fn handle_ui_message(&mut self, message: &UiMessage, engine: &mut GameEngine) {
        scope_profile!();

        if let Some(ButtonMessage::Click) = message.data::<ButtonMessage>() {
            if message.destination() == self.clear {
                engine.user_interface.send_message(ListViewMessage::items(
                    self.messages,
                    MessageDirection::ToWidget,
                    vec![],
                ));
            }
        }
    }

    pub fn update(&mut self, engine: &mut GameEngine) {
        let mut count = engine
            .user_interface
            .node(self.messages)
            .cast::<ListView>()
            .map(|v| v.items().len())
            .unwrap_or_default();

        while let Ok(msg) = self.receiver.try_recv() {
            let text = format!("[{:.2}s] {}", msg.time.as_secs_f32(), msg.content);

            let ctx = &mut engine.user_interface.build_ctx();
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
                                .with_margin(Thickness::uniform(1.0))
                                .with_foreground(Brush::Solid(match msg.kind {
                                    MessageKind::Information => Color::opaque(210, 210, 210),
                                    MessageKind::Warning => Color::ORANGE,
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
                .user_interface
                .send_message(ListViewMessage::add_item(
                    self.messages,
                    MessageDirection::ToWidget,
                    item,
                ));
            engine
                .user_interface
                .send_message(ListViewMessage::bring_item_into_view(
                    self.messages,
                    MessageDirection::ToWidget,
                    item,
                ));

            count += 1;
        }
    }
}
