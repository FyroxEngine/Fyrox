use crate::{
    gui::{BuildContext, Ui, UiMessage, UiNode},
    scene::commands::{
        particle_system::{SetCylinderEmitterHeightCommand, SetCylinderEmitterRadiusCommand},
        SceneCommand,
    },
    send_sync_message,
    sidebar::{make_f32_input_field, make_text_mark, COLUMN_WIDTH, ROW_HEIGHT},
    Message,
};
use rg3d::{
    core::pool::Handle,
    gui::{
        grid::{Column, GridBuilder, Row},
        message::{MessageDirection, NumericUpDownMessage, UiMessageData},
        widget::WidgetBuilder,
    },
    scene::{node::Node, particle_system::emitter::cylinder::CylinderEmitter},
};
use std::sync::mpsc::Sender;

pub struct CylinderSection {
    pub section: Handle<UiNode>,
    radius: Handle<UiNode>,
    height: Handle<UiNode>,
    sender: Sender<Message>,
}

impl CylinderSection {
    pub fn new(ctx: &mut BuildContext, sender: Sender<Message>) -> Self {
        let radius;
        let height;
        let section = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(make_text_mark(ctx, "Radius", 0))
                .with_child({
                    radius = make_f32_input_field(ctx, 0, 0.0, std::f32::MAX, 0.1);
                    radius
                })
                .with_child(make_text_mark(ctx, "Height", 1))
                .with_child({
                    height = make_f32_input_field(ctx, 1, 0.0, std::f32::MAX, 0.1);
                    height
                }),
        )
        .add_column(Column::strict(COLUMN_WIDTH))
        .add_column(Column::stretch())
        .add_row(Row::strict(ROW_HEIGHT))
        .add_row(Row::strict(ROW_HEIGHT))
        .build(ctx);

        Self {
            section,
            sender,
            radius,
            height,
        }
    }

    pub fn sync_to_model(&mut self, cylinder: &CylinderEmitter, ui: &mut Ui) {
        send_sync_message(
            ui,
            NumericUpDownMessage::value(self.radius, MessageDirection::ToWidget, cylinder.radius()),
        );

        send_sync_message(
            ui,
            NumericUpDownMessage::value(self.height, MessageDirection::ToWidget, cylinder.height()),
        );
    }

    pub fn handle_message(
        &mut self,
        message: &UiMessage,
        cylinder: &CylinderEmitter,
        handle: Handle<Node>,
        emitter_index: usize,
    ) {
        if let UiMessageData::NumericUpDown(NumericUpDownMessage::Value(value)) = *message.data() {
            if message.direction() == MessageDirection::FromWidget {
                if message.destination() == self.radius && cylinder.radius().ne(&value) {
                    self.sender
                        .send(Message::DoSceneCommand(
                            SceneCommand::SetCylinderEmitterRadius(
                                SetCylinderEmitterRadiusCommand::new(handle, emitter_index, value),
                            ),
                        ))
                        .unwrap();
                } else if message.destination() == self.height && cylinder.height().ne(&value) {
                    self.sender
                        .send(Message::DoSceneCommand(
                            SceneCommand::SetCylinderEmitterHeight(
                                SetCylinderEmitterHeightCommand::new(handle, emitter_index, value),
                            ),
                        ))
                        .unwrap();
                }
            }
        }
    }
}
