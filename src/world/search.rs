use crate::Message;
use rg3d::gui::text_box::TextCommitMode;
use rg3d::{
    core::pool::Handle,
    gui::{
        button::ButtonBuilder,
        grid::{Column, GridBuilder, Row},
        message::{ButtonMessage, MessageDirection, TextBoxMessage, UiMessage, UiMessageData},
        text_box::TextBoxBuilder,
        widget::WidgetBuilder,
        BuildContext, Thickness, UiNode, UserInterface, VerticalAlignment,
    },
};
use std::sync::mpsc::Sender;

pub struct SearchBar {
    pub container: Handle<UiNode>,
    text: Handle<UiNode>,
    reset: Handle<UiNode>,
}

impl SearchBar {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let text;
        let reset;
        let container = GridBuilder::new(
            WidgetBuilder::new()
                .on_row(1)
                .with_child({
                    text = TextBoxBuilder::new(WidgetBuilder::new().on_column(0))
                        .with_text_commit_mode(TextCommitMode::Immediate)
                        .with_vertical_text_alignment(VerticalAlignment::Center)
                        .build(ctx);
                    text
                })
                .with_child({
                    reset = ButtonBuilder::new(
                        WidgetBuilder::new()
                            .with_margin(Thickness::left(1.0))
                            .on_column(1),
                    )
                    .with_text("X")
                    .build(ctx);
                    reset
                }),
        )
        .add_row(Row::stretch())
        .add_column(Column::stretch())
        .add_column(Column::strict(20.0))
        .build(ctx);

        Self {
            container,
            text,
            reset,
        }
    }

    pub fn handle_ui_message(
        &self,
        message: &UiMessage,
        ui: &UserInterface,
        sender: &Sender<Message>,
    ) {
        match message.data() {
            UiMessageData::TextBox(TextBoxMessage::Text(text)) => {
                if message.destination() == self.text
                    && message.direction() == MessageDirection::FromWidget
                {
                    sender
                        .send(Message::SetWorldViewerFilter(text.clone()))
                        .unwrap();
                }
            }
            UiMessageData::Button(ButtonMessage::Click) => {
                if message.destination() == self.reset {
                    ui.send_message(TextBoxMessage::text(
                        self.text,
                        MessageDirection::ToWidget,
                        Default::default(),
                    ));
                }
            }
            _ => (),
        }
    }
}
