use crate::{
    physics::Joint,
    scene::commands::physics::{
        SetFixedJointAnchor1RotationCommand, SetFixedJointAnchor1TranslationCommand,
        SetFixedJointAnchor2RotationCommand, SetFixedJointAnchor2TranslationCommand,
    },
    send_sync_message,
    sidebar::{make_section, make_text_mark, make_vec3_input_field, COLUMN_WIDTH, ROW_HEIGHT},
    Message,
};
use rg3d::gui::message::UiMessage;
use rg3d::gui::vec::vec3::Vec3EditorMessage;
use rg3d::gui::{BuildContext, UiNode, UserInterface};
use rg3d::{
    core::{
        algebra::Vector3,
        math::{quat_from_euler, RotationOrder},
        pool::Handle,
    },
    gui::{
        grid::{Column, GridBuilder, Row},
        message::{MessageDirection, UiMessageData},
        widget::WidgetBuilder,
    },
    physics3d::desc::FixedJointDesc,
};
use std::sync::mpsc::Sender;

pub struct FixedJointSection {
    pub section: Handle<UiNode>,
    joint_anchor_translation: Handle<UiNode>,
    joint_anchor_rotation: Handle<UiNode>,
    connected_anchor_translation: Handle<UiNode>,
    connected_anchor_rotation: Handle<UiNode>,
    sender: Sender<Message>,
}

impl FixedJointSection {
    pub fn new(ctx: &mut BuildContext, sender: Sender<Message>) -> Self {
        let joint_anchor_translation;
        let joint_anchor_rotation;
        let connected_anchor_translation;
        let connected_anchor_rotation;
        let section = make_section(
            "Fixed Joint Properties",
            GridBuilder::new(
                WidgetBuilder::new()
                    .with_child(make_text_mark(ctx, "Joint Translation", 0))
                    .with_child({
                        joint_anchor_translation = make_vec3_input_field(ctx, 0);
                        joint_anchor_translation
                    })
                    .with_child(make_text_mark(ctx, "Joint Rotation", 1))
                    .with_child({
                        joint_anchor_rotation = make_vec3_input_field(ctx, 1);
                        joint_anchor_rotation
                    })
                    .with_child(make_text_mark(ctx, "Connected Translation", 2))
                    .with_child({
                        connected_anchor_translation = make_vec3_input_field(ctx, 2);
                        connected_anchor_translation
                    })
                    .with_child(make_text_mark(ctx, "Connected Rotation", 3))
                    .with_child({
                        connected_anchor_rotation = make_vec3_input_field(ctx, 3);
                        connected_anchor_rotation
                    }),
            )
            .add_column(Column::strict(COLUMN_WIDTH))
            .add_column(Column::stretch())
            .add_row(Row::strict(ROW_HEIGHT))
            .add_row(Row::strict(ROW_HEIGHT))
            .add_row(Row::strict(ROW_HEIGHT))
            .add_row(Row::strict(ROW_HEIGHT))
            .build(ctx),
            ctx,
        );

        Self {
            section,
            sender,
            joint_anchor_translation,
            joint_anchor_rotation,
            connected_anchor_translation,
            connected_anchor_rotation,
        }
    }

    pub fn sync_to_model(&mut self, fixed: &FixedJointDesc, ui: &mut UserInterface) {
        send_sync_message(
            ui,
            Vec3EditorMessage::value(
                self.joint_anchor_translation,
                MessageDirection::ToWidget,
                fixed.local_anchor1_translation,
            ),
        );

        let euler = fixed.local_anchor1_rotation.euler_angles();
        send_sync_message(
            ui,
            Vec3EditorMessage::value(
                self.joint_anchor_rotation,
                MessageDirection::ToWidget,
                Vector3::new(
                    euler.0.to_degrees(),
                    euler.1.to_degrees(),
                    euler.2.to_degrees(),
                ),
            ),
        );

        send_sync_message(
            ui,
            Vec3EditorMessage::value(
                self.connected_anchor_translation,
                MessageDirection::ToWidget,
                fixed.local_anchor2_translation,
            ),
        );

        let euler = fixed.local_anchor2_rotation.euler_angles();
        send_sync_message(
            ui,
            Vec3EditorMessage::value(
                self.connected_anchor_rotation,
                MessageDirection::ToWidget,
                Vector3::new(
                    euler.0.to_degrees(),
                    euler.1.to_degrees(),
                    euler.2.to_degrees(),
                ),
            ),
        );
    }

    pub fn handle_message(
        &mut self,
        message: &UiMessage,
        fixed: &FixedJointDesc,
        handle: Handle<Joint>,
    ) {
        if let UiMessageData::User(msg) = message.data() {
            if let Some(&Vec3EditorMessage::Value(value)) = msg.cast::<Vec3EditorMessage<f32>>() {
                if message.direction() == MessageDirection::FromWidget {
                    if message.destination() == self.joint_anchor_translation
                        && fixed.local_anchor1_translation.ne(&value)
                    {
                        self.sender
                            .send(Message::do_scene_command(
                                SetFixedJointAnchor1TranslationCommand::new(handle, value),
                            ))
                            .unwrap();
                    } else if message.destination() == self.joint_anchor_rotation {
                        let old_rotation = fixed.local_anchor1_rotation;
                        let euler = Vector3::new(
                            value.x.to_radians(),
                            value.y.to_radians(),
                            value.z.to_radians(),
                        );
                        let new_rotation = quat_from_euler(euler, RotationOrder::XYZ);
                        if old_rotation.ne(&new_rotation) {
                            self.sender
                                .send(Message::do_scene_command(
                                    SetFixedJointAnchor1RotationCommand::new(handle, new_rotation),
                                ))
                                .unwrap();
                        }
                    } else if message.destination() == self.connected_anchor_translation
                        && fixed.local_anchor2_translation.ne(&value)
                    {
                        self.sender
                            .send(Message::do_scene_command(
                                SetFixedJointAnchor2TranslationCommand::new(handle, value),
                            ))
                            .unwrap();
                    } else if message.destination() == self.connected_anchor_rotation {
                        let old_rotation = fixed.local_anchor2_rotation;
                        let euler = Vector3::new(
                            value.x.to_radians(),
                            value.y.to_radians(),
                            value.z.to_radians(),
                        );
                        let new_rotation = quat_from_euler(euler, RotationOrder::XYZ);
                        if old_rotation.ne(&new_rotation) {
                            self.sender
                                .send(Message::do_scene_command(
                                    SetFixedJointAnchor2RotationCommand::new(handle, new_rotation),
                                ))
                                .unwrap();
                        }
                    }
                }
            }
        }
    }
}
