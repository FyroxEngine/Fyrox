use crate::{
    gui::{BuildContext, Ui, UiMessage, UiNode},
    scene::{SceneCommand, SetFovCommand, SetZFarCommand, SetZNearCommand},
    sidebar::{
        make_f32_input_field, make_text_mark, make_vec3_input_field, COLUMN_WIDTH, ROW_HEIGHT,
    },
    Message,
};
use rg3d::{
    core::pool::Handle,
    gui::{
        grid::{Column, GridBuilder, Row},
        message::{MessageDirection, NumericUpDownMessage, UiMessageData, WidgetMessage},
        widget::WidgetBuilder,
    },
    scene::node::Node,
};
use std::sync::mpsc::Sender;

pub struct CameraSection {
    pub section: Handle<UiNode>,
    fov: Handle<UiNode>,
    z_near: Handle<UiNode>,
    z_far: Handle<UiNode>,
    sender: Sender<Message>,
}

impl CameraSection {
    pub fn new(ctx: &mut BuildContext, sender: Sender<Message>) -> Self {
        let fov;
        let z_near;
        let z_far;
        let section = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(make_text_mark(ctx, "FOV", 0))
                .with_child({
                    fov = make_f32_input_field(ctx, 0);
                    fov
                })
                .with_child(make_text_mark(ctx, "Z Near", 1))
                .with_child({
                    z_near = make_f32_input_field(ctx, 1);
                    z_near
                })
                .with_child(make_text_mark(ctx, "Z Far", 2))
                .with_child({
                    z_far = make_vec3_input_field(ctx, 2);
                    z_far
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
            fov,
            z_near,
            z_far,
            sender,
        }
    }

    pub fn sync_to_model(&mut self, node: &Node, ui: &mut Ui) {
        ui.send_message(WidgetMessage::visibility(
            self.section,
            MessageDirection::ToWidget,
            node.is_camera(),
        ));

        if let Node::Camera(camera) = node {
            ui.send_message(NumericUpDownMessage::value(
                self.fov,
                MessageDirection::ToWidget,
                camera.fov(),
            ));

            ui.send_message(NumericUpDownMessage::value(
                self.z_near,
                MessageDirection::ToWidget,
                camera.z_near(),
            ));

            ui.send_message(NumericUpDownMessage::value(
                self.z_far,
                MessageDirection::ToWidget,
                camera.z_far(),
            ));
        }
    }

    pub fn handle_message(&mut self, message: &UiMessage, node: &Node, handle: Handle<Node>) {
        if let Node::Camera(camera) = node {
            if let UiMessageData::NumericUpDown(msg) = &message.data() {
                if let NumericUpDownMessage::Value(value) = *msg {
                    if message.destination() == self.fov && camera.fov().ne(&value) {
                        self.sender
                            .send(Message::DoSceneCommand(SceneCommand::SetFov(
                                SetFovCommand::new(handle, value),
                            )))
                            .unwrap();
                    } else if message.destination() == self.z_far && camera.z_far().ne(&value) {
                        self.sender
                            .send(Message::DoSceneCommand(SceneCommand::SetZFar(
                                SetZFarCommand::new(handle, value),
                            )))
                            .unwrap();
                    } else if message.destination() == self.z_near && camera.z_near().ne(&value) {
                        self.sender
                            .send(Message::DoSceneCommand(SceneCommand::SetZNear(
                                SetZNearCommand::new(handle, value),
                            )))
                            .unwrap();
                    }
                }
            }
        }
    }
}
