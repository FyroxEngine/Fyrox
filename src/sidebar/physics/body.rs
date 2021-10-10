use crate::sidebar::make_section;
use crate::{
    physics::RigidBody,
    scene::commands::physics::SetBodyMassCommand,
    send_sync_message,
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
};
use std::sync::mpsc::Sender;

pub struct BodySection {
    pub section: Handle<UiNode>,
    mass: Handle<UiNode>,
    sender: Sender<Message>,
}

impl BodySection {
    pub fn new(ctx: &mut BuildContext, sender: Sender<Message>) -> Self {
        let mass;
        let section = make_section(
            "Body Properties",
            GridBuilder::new(
                WidgetBuilder::new()
                    .with_child(make_text_mark(ctx, "Mass", 0))
                    .with_child({
                        mass = make_f32_input_field(ctx, 0, 0.0, std::f32::MAX, 0.1);
                        mass
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
            mass,
        }
    }

    pub fn sync_to_model(&mut self, body: &RigidBody, ui: &mut UserInterface) {
        send_sync_message(
            ui,
            NumericUpDownMessage::value(self.mass, MessageDirection::ToWidget, body.mass),
        );
    }

    pub fn handle_message(
        &mut self,
        message: &UiMessage,
        body: &RigidBody,
        handle: Handle<RigidBody>,
    ) {
        if let UiMessageData::User(msg) = message.data() {
            if let Some(&NumericUpDownMessage::Value(value)) =
                msg.cast::<NumericUpDownMessage<f32>>()
            {
                if message.direction() == MessageDirection::FromWidget
                    && message.destination() == self.mass
                    && body.mass.ne(&value)
                {
                    self.sender
                        .send(Message::do_scene_command(SetBodyMassCommand::new(
                            handle, value,
                        )))
                        .unwrap();
                }
            }
        }
    }
}
