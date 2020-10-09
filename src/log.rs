use crate::{
    gui::{BuildContext, UiMessage, UiNode},
    GameEngine, Message,
};
use rg3d::gui::grid::{Column, Row};
use rg3d::gui::window::WindowTitle;
use rg3d::gui::Thickness;
use rg3d::{
    core::pool::Handle,
    gui::{
        button::ButtonBuilder,
        grid::GridBuilder,
        list_view::ListViewBuilder,
        message::{ButtonMessage, ListViewMessage, MessageDirection, UiMessageData},
        stack_panel::StackPanelBuilder,
        text::TextBuilder,
        widget::WidgetBuilder,
        window::WindowBuilder,
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
                                .with_text("X")
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
                .add_column(Column::strict(20.0))
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
        if let UiMessageData::Button(msg) = message.data() {
            if let ButtonMessage::Click = msg {
                if message.destination() == self.clear {
                    engine.user_interface.send_message(ListViewMessage::items(
                        self.messages,
                        MessageDirection::ToWidget,
                        vec![],
                    ));
                }
            }
        }
    }

    pub fn handle_message(&mut self, message: &Message, engine: &mut GameEngine) {
        if let Message::Log(string) = message {
            let item = TextBuilder::new(WidgetBuilder::new())
                .with_text(string.clone())
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
