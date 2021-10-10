use crate::sidebar::make_section;
use crate::{
    physics::Collider,
    scene::commands::physics::SetBallRadiusCommand,
    sidebar::{make_f32_input_field, make_text_mark, COLUMN_WIDTH, ROW_HEIGHT},
    Message,
};
use rg3d::gui::message::UiMessage;
use rg3d::gui::numeric::NumericUpDownMessage;
use rg3d::gui::{BuildContext, UiNode, UserInterface};
use rg3d::{
    core::pool::Handle,
    gui::{
        grid::{Column, GridBuilder, Row},
        message::{MessageDirection, UiMessageData},
        widget::WidgetBuilder,
    },
    physics3d::desc::BallDesc,
};
use std::sync::mpsc::Sender;

pub struct BallSection {
    pub section: Handle<UiNode>,
    radius: Handle<UiNode>,
    sender: Sender<Message>,
}

impl BallSection {
    pub fn new(ctx: &mut BuildContext, sender: Sender<Message>) -> Self {
        let radius;
        let section = make_section(
            "Ball Properties",
            GridBuilder::new(
                WidgetBuilder::new()
                    .with_child(make_text_mark(ctx, "Radius", 0))
                    .with_child({
                        radius = make_f32_input_field(ctx, 0, 0.0, std::f32::MAX, 0.1);
                        radius
                    }),
            )
            .add_column(Column::strict(COLUMN_WIDTH))
            .add_column(Column::stretch())
            .add_row(Row::strict(ROW_HEIGHT))
            .build(ctx),
            ctx,
        );

        Self {
            section,
            sender,
            radius,
        }
    }

    pub fn sync_to_model(&mut self, ball: &BallDesc, ui: &mut UserInterface) {
        ui.send_message(NumericUpDownMessage::value(
            self.radius,
            MessageDirection::ToWidget,
            ball.radius,
        ));
    }

    pub fn handle_message(
        &mut self,
        message: &UiMessage,
        ball: &BallDesc,
        handle: Handle<Collider>,
    ) {
        if let UiMessageData::User(msg) = message.data() {
            if let Some(&NumericUpDownMessage::Value(value)) =
                msg.cast::<NumericUpDownMessage<f32>>()
            {
                if message.direction() == MessageDirection::FromWidget
                    && message.destination() == self.radius
                    && ball.radius.ne(&value)
                {
                    self.sender
                        .send(Message::do_scene_command(SetBallRadiusCommand::new(
                            handle, value,
                        )))
                        .unwrap();
                }
            }
        }
    }
}
