use crate::{
    gui::{BuildContext, Ui, UiMessage, UiNode},
    scene::commands::{
        camera::{SetCameraPreviewCommand, SetFovCommand, SetZFarCommand, SetZNearCommand},
        SceneCommand,
    },
    send_sync_message,
    sidebar::{make_f32_input_field, make_text_mark, COLUMN_WIDTH, ROW_HEIGHT},
    Message,
};
use rg3d::{
    core::{pool::Handle, scope_profile},
    gui::{
        grid::{Column, GridBuilder, Row},
        message::{
            CheckBoxMessage, MessageDirection, NumericUpDownMessage, UiMessageData, WidgetMessage,
        },
        widget::WidgetBuilder,
    },
    scene::node::Node,
};
use std::sync::mpsc::Sender;

use super::make_bool_input_field;

pub struct CameraSection {
    pub section: Handle<UiNode>,
    fov: Handle<UiNode>,
    z_near: Handle<UiNode>,
    z_far: Handle<UiNode>,
    sender: Sender<Message>,
    preview: Handle<UiNode>,
}

impl CameraSection {
    pub fn new(ctx: &mut BuildContext, sender: Sender<Message>) -> Self {
        let fov;
        let z_near;
        let z_far;
        let preview;
        let section = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(make_text_mark(ctx, "FOV", 0))
                .with_child({
                    fov = make_f32_input_field(ctx, 0, 0.0, std::f32::consts::PI, 0.01);
                    fov
                })
                .with_child(make_text_mark(ctx, "Z Near", 1))
                .with_child({
                    z_near = make_f32_input_field(ctx, 1, 0.0, std::f32::MAX, 0.01);
                    z_near
                })
                .with_child(make_text_mark(ctx, "Z Far", 2))
                .with_child({
                    z_far = make_f32_input_field(ctx, 2, 0.0, std::f32::MAX, 1.0);
                    z_far
                })
                .with_child(make_text_mark(ctx, "Preview", 3))
                .with_child({
                    preview = make_bool_input_field(ctx, 3);
                    preview
                }),
        )
        .add_column(Column::strict(COLUMN_WIDTH))
        .add_column(Column::stretch())
        .add_row(Row::strict(ROW_HEIGHT))
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
            preview,
        }
    }

    pub fn sync_to_model(&mut self, node: &Node, ui: &mut Ui) {
        send_sync_message(
            ui,
            WidgetMessage::visibility(self.section, MessageDirection::ToWidget, node.is_camera()),
        );

        if let Node::Camera(camera) = node {
            send_sync_message(
                ui,
                NumericUpDownMessage::value(self.fov, MessageDirection::ToWidget, camera.fov()),
            );

            send_sync_message(
                ui,
                NumericUpDownMessage::value(
                    self.z_near,
                    MessageDirection::ToWidget,
                    camera.z_near(),
                ),
            );

            send_sync_message(
                ui,
                NumericUpDownMessage::value(self.z_far, MessageDirection::ToWidget, camera.z_far()),
            );

            send_sync_message(
                ui,
                CheckBoxMessage::checked(
                    self.preview,
                    MessageDirection::ToWidget,
                    Some(camera.is_enabled()),
                ),
            );
        }
    }

    pub fn handle_message(&mut self, message: &UiMessage, node: &Node, handle: Handle<Node>) {
        scope_profile!();

        if let Node::Camera(camera) = node {
            if let UiMessageData::NumericUpDown(NumericUpDownMessage::Value(value)) =
                *message.data()
            {
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
            } else if let UiMessageData::CheckBox(CheckBoxMessage::Check(value)) = *message.data() {
                if message.destination() == self.preview && camera.is_enabled().ne(&value.unwrap())
                {
                    self.sender
                        .send(Message::DoSceneCommand(SceneCommand::SetCameraActive(
                            SetCameraPreviewCommand::new(handle, value.unwrap_or(false)),
                        )))
                        .unwrap();
                }
            }
        }
    }
}
