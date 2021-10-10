use crate::sidebar::make_section;
use crate::{
    scene::commands::light::SetPointLightRadiusCommand,
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
        message::{MessageDirection, UiMessageData, WidgetMessage},
        widget::WidgetBuilder,
    },
    scene::{light::Light, node::Node},
};
use std::sync::mpsc::Sender;

pub struct PointLightSection {
    pub section: Handle<UiNode>,
    radius: Handle<UiNode>,
    sender: Sender<Message>,
}

impl PointLightSection {
    pub fn new(ctx: &mut BuildContext, sender: Sender<Message>) -> Self {
        let radius;
        let section = make_section(
            "Point Light Properties",
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
            radius,
            sender,
        }
    }

    pub fn sync_to_model(&mut self, node: &Node, ui: &mut UserInterface) {
        let visible = if let Node::Light(Light::Point(point)) = node {
            send_sync_message(
                ui,
                NumericUpDownMessage::value(
                    self.radius,
                    MessageDirection::ToWidget,
                    point.radius(),
                ),
            );

            true
        } else {
            false
        };
        send_sync_message(
            ui,
            WidgetMessage::visibility(self.section, MessageDirection::ToWidget, visible),
        );
    }

    pub fn handle_message(&mut self, message: &UiMessage, node: &Node, handle: Handle<Node>) {
        if let Node::Light(Light::Point(point)) = node {
            if let UiMessageData::User(msg) = message.data() {
                if let Some(&NumericUpDownMessage::Value(value)) =
                    msg.cast::<NumericUpDownMessage<f32>>()
                {
                    if message.destination() == self.radius && point.radius().ne(&value) {
                        self.sender
                            .send(Message::do_scene_command(SetPointLightRadiusCommand::new(
                                handle, value,
                            )))
                            .unwrap();
                    }
                }
            }
        }
    }
}
