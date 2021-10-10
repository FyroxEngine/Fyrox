use crate::{
    scene::commands::light::{
        SetSpotLightDistanceCommand, SetSpotLightFalloffAngleDeltaCommand,
        SetSpotLightHotspotCommand,
    },
    send_sync_message,
    sidebar::{make_f32_input_field, make_section, make_text_mark, COLUMN_WIDTH, ROW_HEIGHT},
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

pub struct SpotLightSection {
    pub section: Handle<UiNode>,
    hotspot: Handle<UiNode>,
    falloff_delta: Handle<UiNode>,
    distance: Handle<UiNode>,
    sender: Sender<Message>,
}

impl SpotLightSection {
    pub fn new(ctx: &mut BuildContext, sender: Sender<Message>) -> Self {
        let hotspot;
        let falloff_delta;
        let distance;

        let section = make_section(
            "Light Properties",
            GridBuilder::new(
                WidgetBuilder::new()
                    .with_child(make_text_mark(ctx, "Hotspot", 0))
                    .with_child({
                        hotspot = make_f32_input_field(ctx, 0, 0.0, std::f32::consts::PI, 0.1);
                        hotspot
                    })
                    .with_child(make_text_mark(ctx, "Falloff Delta", 1))
                    .with_child({
                        falloff_delta =
                            make_f32_input_field(ctx, 1, 0.0, std::f32::consts::PI, 0.01);
                        falloff_delta
                    })
                    .with_child(make_text_mark(ctx, "Radius", 2))
                    .with_child({
                        distance = make_f32_input_field(ctx, 2, 0.0, std::f32::MAX, 0.1);
                        distance
                    }),
            )
            .add_column(Column::strict(COLUMN_WIDTH))
            .add_column(Column::stretch())
            .add_row(Row::strict(ROW_HEIGHT))
            .add_row(Row::strict(ROW_HEIGHT))
            .add_row(Row::strict(ROW_HEIGHT))
            .build(ctx),
            ctx,
        );

        Self {
            section,
            hotspot,
            falloff_delta,
            distance,
            sender,
        }
    }

    pub fn sync_to_model(&mut self, node: &Node, ui: &mut UserInterface) {
        let visible = if let Node::Light(Light::Spot(spot)) = node {
            send_sync_message(
                ui,
                NumericUpDownMessage::value(
                    self.hotspot,
                    MessageDirection::ToWidget,
                    spot.hotspot_cone_angle(),
                ),
            );

            send_sync_message(
                ui,
                NumericUpDownMessage::value(
                    self.falloff_delta,
                    MessageDirection::ToWidget,
                    spot.falloff_angle_delta(),
                ),
            );

            send_sync_message(
                ui,
                NumericUpDownMessage::value(
                    self.distance,
                    MessageDirection::ToWidget,
                    spot.distance(),
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
        if let Node::Light(Light::Spot(spot)) = node {
            if let UiMessageData::User(msg) = message.data() {
                if let Some(&NumericUpDownMessage::Value(value)) =
                    msg.cast::<NumericUpDownMessage<f32>>()
                {
                    if message.destination() == self.hotspot && spot.hotspot_cone_angle().ne(&value)
                    {
                        self.sender
                            .send(Message::do_scene_command(SetSpotLightHotspotCommand::new(
                                handle, value,
                            )))
                            .unwrap();
                    } else if message.destination() == self.falloff_delta
                        && spot.falloff_angle_delta().ne(&value)
                    {
                        self.sender
                            .send(Message::do_scene_command(
                                SetSpotLightFalloffAngleDeltaCommand::new(handle, value),
                            ))
                            .unwrap();
                    } else if message.destination() == self.distance && spot.distance().ne(&value) {
                        self.sender
                            .send(Message::do_scene_command(SetSpotLightDistanceCommand::new(
                                handle, value,
                            )))
                            .unwrap();
                    }
                }
            }
        }
    }
}
