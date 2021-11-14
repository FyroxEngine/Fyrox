use crate::{GameEngine, Message};
use rg3d::{
    core::{algebra::Vector2, pool::Handle, scope_profile},
    gui::{
        button::{ButtonBuilder, ButtonMessage},
        formatted_text::WrapMode,
        grid::{Column, GridBuilder, Row},
        list_view::{ListViewBuilder, ListViewMessage},
        message::{MessageDirection, UiMessage},
        stack_panel::StackPanelBuilder,
        text::TextBuilder,
        vector_image::{Primitive, VectorImageBuilder},
        widget::WidgetBuilder,
        window::{WindowBuilder, WindowTitle},
        BuildContext, HorizontalAlignment, Thickness, UiNode, VerticalAlignment, BRUSH_BRIGHT,
    },
};

pub struct Log {
    pub window: Handle<UiNode>,
    messages: Handle<UiNode>,
    clear: Handle<UiNode>,
}

impl Log {
    pub fn new(ctx: &mut BuildContext) -> Self {
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

    pub fn handle_message(&mut self, message: &Message, engine: &mut GameEngine) {
        if let Message::Log(string) = message {
            let item = TextBuilder::new(WidgetBuilder::new())
                .with_text(string.clone())
                .with_wrap(WrapMode::Word)
                .build(&mut engine.user_interface.build_ctx());
            engine
                .user_interface
                .send_message(ListViewMessage::add_item(
                    self.messages,
                    MessageDirection::ToWidget,
                    item,
                ));
        }
    }
}
