use crate::scene::{
    SetBoxEmitterHalfDepthCommand, SetBoxEmitterHalfHeightCommand, SetBoxEmitterHalfWidthCommand,
};
use crate::{
    gui::{BuildContext, Ui, UiMessage, UiNode},
    scene::SceneCommand,
    send_sync_message,
    sidebar::{make_f32_input_field, make_text_mark, COLUMN_WIDTH, ROW_HEIGHT},
    Message,
};
use rg3d::scene::particle_system::BoxEmitter;
use rg3d::{
    core::pool::Handle,
    gui::{
        grid::{Column, GridBuilder, Row},
        message::{MessageDirection, NumericUpDownMessage, UiMessageData},
        widget::WidgetBuilder,
    },
    scene::node::Node,
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
        let section = GridBuilder::new(
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
        .build(ctx);

        Self {
            section,
            sender,
            half_width,
            half_height,
            half_depth,
        }
    }

    pub fn sync_to_model(&mut self, box_emitter: &BoxEmitter, ui: &mut Ui) {
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
        box_emitter: &BoxEmitter,
        handle: Handle<Node>,
        emitter_index: usize,
    ) {
        if let UiMessageData::NumericUpDown(NumericUpDownMessage::Value(value)) = *message.data() {
            if message.direction() == MessageDirection::FromWidget {
                if message.destination() == self.half_width && box_emitter.half_width().ne(&value) {
                    self.sender
                        .send(Message::DoSceneCommand(
                            SceneCommand::SetBoxEmitterHalfWidth(
                                SetBoxEmitterHalfWidthCommand::new(handle, emitter_index, value),
                            ),
                        ))
                        .unwrap();
                } else if message.destination() == self.half_height
                    && box_emitter.half_height().ne(&value)
                {
                    self.sender
                        .send(Message::DoSceneCommand(
                            SceneCommand::SetBoxEmitterHalfHeight(
                                SetBoxEmitterHalfHeightCommand::new(handle, emitter_index, value),
                            ),
                        ))
                        .unwrap();
                } else if message.destination() == self.half_depth
                    && box_emitter.half_depth().ne(&value)
                {
                    self.sender
                        .send(Message::DoSceneCommand(
                            SceneCommand::SetBoxEmitterHalfDepth(
                                SetBoxEmitterHalfDepthCommand::new(handle, emitter_index, value),
                            ),
                        ))
                        .unwrap();
                }
            }
        }
    }
}
