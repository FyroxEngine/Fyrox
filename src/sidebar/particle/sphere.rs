use crate::{
    gui::{BuildContext, Ui, UiMessage, UiNode},
    scene::{SceneCommand, SetSphereEmitterRadiusCommand},
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
    scene::{node::Node, particle_system::SphereEmitter},
};
use std::sync::mpsc::Sender;

pub struct SphereSection {
    pub section: Handle<UiNode>,
    radius: Handle<UiNode>,
    sender: Sender<Message>,
}

impl SphereSection {
    pub fn new(ctx: &mut BuildContext, sender: Sender<Message>) -> Self {
        let radius;
        let section = GridBuilder::new(
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
        .build(ctx);

        Self {
            section,
            sender,
            radius,
        }
    }

    pub fn sync_to_model(&mut self, sphere: &SphereEmitter, ui: &mut Ui) {
        send_sync_message(
            ui,
            NumericUpDownMessage::value(self.radius, MessageDirection::ToWidget, sphere.radius()),
        );
    }

    pub fn handle_message(
        &mut self,
        message: &UiMessage,
        sphere: &SphereEmitter,
        handle: Handle<Node>,
        emitter_index: usize,
    ) {
        if let UiMessageData::NumericUpDown(NumericUpDownMessage::Value(value)) = *message.data() {
            if message.direction() == MessageDirection::FromWidget
                && message.destination() == self.radius
                && sphere.radius().ne(&value)
            {
                self.sender
                    .send(Message::DoSceneCommand(
                        SceneCommand::SetSphereEmitterRadius(SetSphereEmitterRadiusCommand::new(
                            handle,
                            emitter_index,
                            value,
                        )),
                    ))
                    .unwrap();
            }
        }
    }
}
