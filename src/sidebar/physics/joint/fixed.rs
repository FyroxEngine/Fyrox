use crate::{
    gui::{BuildContext, Ui, UiMessage, UiNode},
    physics::Joint,
    scene::commands::{
        physics::{
            SetFixedJointAnchor1RotationCommand, SetFixedJointAnchor1TranslationCommand,
            SetFixedJointAnchor2RotationCommand, SetFixedJointAnchor2TranslationCommand,
        },
        SceneCommand,
    },
    send_sync_message,
    sidebar::{make_text_mark, make_vec3_input_field, COLUMN_WIDTH, ROW_HEIGHT},
    Message,
};
use rg3d::{
    core::{
        algebra::Vector3,
        math::{quat_from_euler, RotationOrder, UnitQuaternionExt},
        pool::Handle,
    },
    gui::{
        grid::{Column, GridBuilder, Row},
        message::{MessageDirection, UiMessageData, Vec3EditorMessage},
        widget::WidgetBuilder,
    },
    scene::physics::desc::FixedJointDesc,
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
        let section = GridBuilder::new(
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
        .build(ctx);

        Self {
            section,
            sender,
            joint_anchor_translation,
            joint_anchor_rotation,
            connected_anchor_translation,
            connected_anchor_rotation,
        }
    }

    pub fn sync_to_model(&mut self, fixed: &FixedJointDesc, ui: &mut Ui) {
        send_sync_message(
            ui,
            Vec3EditorMessage::value(
                self.joint_anchor_translation,
                MessageDirection::ToWidget,
                fixed.local_anchor1_translation,
            ),
        );

        let euler = fixed.local_anchor1_rotation.to_euler();
        send_sync_message(
            ui,
            Vec3EditorMessage::value(
                self.joint_anchor_rotation,
                MessageDirection::ToWidget,
                Vector3::new(
                    euler.x.to_degrees(),
                    euler.y.to_degrees(),
                    euler.z.to_degrees(),
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

        let euler = fixed.local_anchor2_rotation.to_euler();
        send_sync_message(
            ui,
            Vec3EditorMessage::value(
                self.connected_anchor_rotation,
                MessageDirection::ToWidget,
                Vector3::new(
                    euler.x.to_degrees(),
                    euler.y.to_degrees(),
                    euler.z.to_degrees(),
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
        if let UiMessageData::Vec3Editor(Vec3EditorMessage::Value(value)) = *message.data() {
            if message.direction() == MessageDirection::FromWidget {
                if message.destination() == self.joint_anchor_translation
                    && fixed.local_anchor1_translation.ne(&value)
                {
                    self.sender
                        .send(Message::DoSceneCommand(
                            SceneCommand::SetFixedJointAnchor1Translation(
                                SetFixedJointAnchor1TranslationCommand::new(handle, value),
                            ),
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
                    if !old_rotation.approx_eq(&new_rotation, 0.00001) {
                        self.sender
                            .send(Message::DoSceneCommand(
                                SceneCommand::SetFixedJointAnchor1Rotation(
                                    SetFixedJointAnchor1RotationCommand::new(handle, new_rotation),
                                ),
                            ))
                            .unwrap();
                    }
                } else if message.destination() == self.connected_anchor_translation
                    && fixed.local_anchor2_translation.ne(&value)
                {
                    self.sender
                        .send(Message::DoSceneCommand(
                            SceneCommand::SetFixedJointAnchor2Translation(
                                SetFixedJointAnchor2TranslationCommand::new(handle, value),
                            ),
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
                    if !old_rotation.approx_eq(&new_rotation, 0.00001) {
                        self.sender
                            .send(Message::DoSceneCommand(
                                SceneCommand::SetFixedJointAnchor2Rotation(
                                    SetFixedJointAnchor2RotationCommand::new(handle, new_rotation),
                                ),
                            ))
                            .unwrap();
                    }
                }
            }
        }
    }
}
