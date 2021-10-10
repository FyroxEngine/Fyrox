use crate::sidebar::make_section;
use crate::{
    scene::commands::particle_system::{
        SetBoxEmitterHalfDepthCommand, SetBoxEmitterHalfHeightCommand,
        SetBoxEmitterHalfWidthCommand,
    },
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
    scene::{node::Node, particle_system::emitter::cuboid::CuboidEmitter},
};
use std::sync::mpsc::Sender;

pub struct BoxSection {
    pub section: Handle<UiNode>,
    half_width: Handle<UiNode>,
    half_height: Handle<UiNode>,
    half_depth: Handle<UiNode>,
    sender: Sender<Message>,
}

impl BoxSection {
    pub fn new(ctx: &mut BuildContext, sender: Sender<Message>) -> Self {
        let half_width;
        let half_height;
        let half_depth;
        let section = make_section(
            "Cuboid Emitter Properties",
            GridBuilder::new(
                WidgetBuilder::new()
                    .with_child(make_text_mark(ctx, "Half Width", 0))
                    .with_child({
                        half_width = make_f32_input_field(ctx, 0, 0.0, std::f32::MAX, 0.1);
                        half_width
                    })
                    .with_child(make_text_mark(ctx, "Half Height", 1))
                    .with_child({
                        half_height = make_f32_input_field(ctx, 1, 0.0, std::f32::MAX, 0.1);
                        half_height
                    })
                    .with_child(make_text_mark(ctx, "Half Depth", 2))
                    .with_child({
                        half_depth = make_f32_input_field(ctx, 2, 0.0, std::f32::MAX, 0.1);
                        half_depth
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
            sender,
            half_width,
            half_height,
            half_depth,
        }
    }

    pub fn sync_to_model(&mut self, box_emitter: &CuboidEmitter, ui: &mut UserInterface) {
        send_sync_message(
            ui,
            NumericUpDownMessage::value(
                self.half_width,
                MessageDirection::ToWidget,
                box_emitter.half_width(),
            ),
        );

        send_sync_message(
            ui,
            NumericUpDownMessage::value(
                self.half_height,
                MessageDirection::ToWidget,
                box_emitter.half_height(),
            ),
        );

        send_sync_message(
            ui,
            NumericUpDownMessage::value(
                self.half_depth,
                MessageDirection::ToWidget,
                box_emitter.half_depth(),
            ),
        );
    }

    pub fn handle_message(
        &mut self,
        message: &UiMessage,
        box_emitter: &CuboidEmitter,
        handle: Handle<Node>,
        emitter_index: usize,
    ) {
        if let UiMessageData::User(msg) = message.data() {
            if let Some(&NumericUpDownMessage::Value(value)) =
                msg.cast::<NumericUpDownMessage<f32>>()
            {
                if message.direction() == MessageDirection::FromWidget {
                    if message.destination() == self.half_width
                        && box_emitter.half_width().ne(&value)
                    {
                        self.sender
                            .send(Message::do_scene_command(
                                SetBoxEmitterHalfWidthCommand::new(handle, emitter_index, value),
                            ))
                            .unwrap();
                    } else if message.destination() == self.half_height
                        && box_emitter.half_height().ne(&value)
                    {
                        self.sender
                            .send(Message::do_scene_command(
                                SetBoxEmitterHalfHeightCommand::new(handle, emitter_index, value),
                            ))
                            .unwrap();
                    } else if message.destination() == self.half_depth
                        && box_emitter.half_depth().ne(&value)
                    {
                        self.sender
                            .send(Message::do_scene_command(
                                SetBoxEmitterHalfDepthCommand::new(handle, emitter_index, value),
                            ))
                            .unwrap();
                    }
                }
            }
        }
    }
}
