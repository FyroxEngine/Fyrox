use crate::{
    gui::{BuildContext, Ui, UiMessage, UiNode},
    scene::{SceneCommand, SetParticleSystemAccelerationCommand},
    sidebar::{make_text_mark, make_vec3_input_field, COLUMN_WIDTH, ROW_HEIGHT},
    Message,
};
use rg3d::{
    core::pool::Handle,
    gui::{
        grid::{Column, GridBuilder, Row},
        message::{MessageDirection, UiMessageData, Vec3EditorMessage, WidgetMessage},
        widget::WidgetBuilder,
    },
    scene::node::Node,
};
use std::sync::mpsc::Sender;

pub struct ParticleSystemSection {
    pub section: Handle<UiNode>,
    acceleration: Handle<UiNode>,
    sender: Sender<Message>,
}

impl ParticleSystemSection {
    pub fn new(ctx: &mut BuildContext, sender: Sender<Message>) -> Self {
        let acceleration;
        let section = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(make_text_mark(ctx, "Acceleration", 0))
                .with_child({
                    acceleration = make_vec3_input_field(ctx, 0);
                    acceleration
                }),
        )
        .add_column(Column::strict(COLUMN_WIDTH))
        .add_column(Column::stretch())
        .add_row(Row::strict(ROW_HEIGHT))
        .build(ctx);

        Self {
            section,
            acceleration,
            sender,
        }
    }

    pub fn sync_to_model(&mut self, node: &Node, ui: &mut Ui) {
        ui.send_message(WidgetMessage::visibility(
            self.section,
            MessageDirection::ToWidget,
            node.is_particle_system(),
        ));

        if let Node::ParticleSystem(particle_system) = node {
            ui.send_message(Vec3EditorMessage::value(
                self.acceleration,
                MessageDirection::ToWidget,
                particle_system.acceleration(),
            ));
        }
    }

    pub fn handle_message(&mut self, message: &UiMessage, node: &Node, handle: Handle<Node>) {
        if let Node::ParticleSystem(particle_system) = node {
            if let UiMessageData::Vec3Editor(msg) = &message.data() {
                if let Vec3EditorMessage::Value(value) = *msg {
                    if particle_system.acceleration() != value
                        && message.destination() == self.acceleration
                    {
                        self.sender
                            .send(Message::DoSceneCommand(
                                SceneCommand::SetParticleSystemAcceleration(
                                    SetParticleSystemAccelerationCommand::new(handle, value),
                                ),
                            ))
                            .unwrap();
                    }
                }
            }
        }
    }
}
