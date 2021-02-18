use crate::scene::{
    SetColliderIsSensorCommand, SetColliderPositionCommand, SetColliderRotationCommand,
};
use crate::sidebar::{make_bool_input_field, make_vec3_input_field};
use crate::{
    gui::{BuildContext, Ui, UiMessage, UiNode},
    physics::Collider,
    scene::{SceneCommand, SetColliderFrictionCommand, SetColliderRestitutionCommand},
    sidebar::{make_f32_input_field, make_text_mark, COLUMN_WIDTH, ROW_HEIGHT},
    Message,
};
use rg3d::core::math::{quat_from_euler, RotationOrder, UnitQuaternionExt};
use rg3d::gui::message::{CheckBoxMessage, Vec3EditorMessage};
use rg3d::{
    core::algebra::Vector3,
    core::pool::Handle,
    gui::{
        grid::{Column, GridBuilder, Row},
        message::{MessageDirection, NumericUpDownMessage, UiMessageData},
        widget::WidgetBuilder,
    },
};
use std::sync::mpsc::Sender;

pub struct ColliderSection {
    pub section: Handle<UiNode>,
    friction: Handle<UiNode>,
    restitution: Handle<UiNode>,
    position: Handle<UiNode>,
    rotation: Handle<UiNode>,
    is_sensor: Handle<UiNode>,
    sender: Sender<Message>,
}

impl ColliderSection {
    pub fn new(ctx: &mut BuildContext, sender: Sender<Message>) -> Self {
        let friction;
        let restitution;
        let position;
        let rotation;
        let is_sensor;
        let section = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(make_text_mark(ctx, "Friction", 0))
                .with_child({
                    friction = make_f32_input_field(ctx, 0, 0.0, std::f32::MAX, 0.1);
                    friction
                })
                .with_child(make_text_mark(ctx, "Restitution", 1))
                .with_child({
                    restitution = make_f32_input_field(ctx, 1, 0.0, std::f32::MAX, 0.1);
                    restitution
                })
                .with_child(make_text_mark(ctx, "Collider Position", 2))
                .with_child({
                    position = make_vec3_input_field(ctx, 2);
                    position
                })
                .with_child(make_text_mark(ctx, "Collider Rotation", 3))
                .with_child({
                    rotation = make_vec3_input_field(ctx, 3);
                    rotation
                })
                .with_child(make_text_mark(ctx, "Is Sensor", 4))
                .with_child({
                    is_sensor = make_bool_input_field(ctx, 4);
                    is_sensor
                }),
        )
        .add_column(Column::strict(COLUMN_WIDTH))
        .add_column(Column::stretch())
        .add_row(Row::strict(ROW_HEIGHT))
        .add_row(Row::strict(ROW_HEIGHT))
        .add_row(Row::strict(ROW_HEIGHT))
        .add_row(Row::strict(ROW_HEIGHT))
        .add_row(Row::strict(ROW_HEIGHT))
        .build(ctx);

        Self {
            section,
            sender,
            friction,
            restitution,
            position,
            rotation,
            is_sensor,
        }
    }

    pub fn sync_to_model(&mut self, collider: &Collider, ui: &mut Ui) {
        ui.send_message(NumericUpDownMessage::value(
            self.friction,
            MessageDirection::ToWidget,
            collider.friction,
        ));

        ui.send_message(NumericUpDownMessage::value(
            self.restitution,
            MessageDirection::ToWidget,
            collider.restitution,
        ));

        ui.send_message(Vec3EditorMessage::value(
            self.position,
            MessageDirection::ToWidget,
            collider.translation,
        ));

        let euler = collider.rotation.to_euler();
        let euler_degrees = Vector3::new(
            euler.x.to_degrees(),
            euler.y.to_degrees(),
            euler.z.to_degrees(),
        );
        ui.send_message(Vec3EditorMessage::value(
            self.rotation,
            MessageDirection::ToWidget,
            euler_degrees,
        ));

        ui.send_message(CheckBoxMessage::checked(
            self.is_sensor,
            MessageDirection::ToWidget,
            Some(collider.is_sensor),
        ));
    }

    pub fn handle_message(
        &mut self,
        message: &UiMessage,
        collider: &Collider,
        handle: Handle<Collider>,
    ) {
        if message.direction() == MessageDirection::FromWidget {
            match message.data() {
                &UiMessageData::NumericUpDown(NumericUpDownMessage::Value(value)) => {
                    if message.destination() == self.friction && collider.friction.ne(&value) {
                        self.sender
                            .send(Message::DoSceneCommand(SceneCommand::SetColliderFriction(
                                SetColliderFrictionCommand::new(handle, value),
                            )))
                            .unwrap();
                    } else if message.destination() == self.restitution
                        && collider.restitution.ne(&value)
                    {
                        self.sender
                            .send(Message::DoSceneCommand(
                                SceneCommand::SetColliderRestitution(
                                    SetColliderRestitutionCommand::new(handle, value),
                                ),
                            ))
                            .unwrap();
                    }
                }
                UiMessageData::Vec3Editor(Vec3EditorMessage::Value(value)) => {
                    if message.destination() == self.position && collider.translation.ne(value) {
                        self.sender
                            .send(Message::DoSceneCommand(SceneCommand::SetColliderPosition(
                                SetColliderPositionCommand::new(handle, *value),
                            )))
                            .unwrap();
                    } else if message.destination() == self.rotation {
                        let old_rotation = collider.rotation;
                        let euler = Vector3::new(
                            value.x.to_radians(),
                            value.y.to_radians(),
                            value.z.to_radians(),
                        );
                        let new_rotation = quat_from_euler(euler, RotationOrder::XYZ);
                        if !old_rotation.approx_eq(&new_rotation, 0.00001) {
                            self.sender
                                .send(Message::DoSceneCommand(SceneCommand::SetColliderRotation(
                                    SetColliderRotationCommand::new(handle, new_rotation),
                                )))
                                .unwrap();
                        }
                    }
                }
                UiMessageData::CheckBox(CheckBoxMessage::Check(checked)) => {
                    if message.destination() == self.is_sensor {
                        let value = checked.unwrap_or_default();
                        if value != collider.is_sensor {
                            self.sender
                                .send(Message::DoSceneCommand(SceneCommand::SetColliderIsSensor(
                                    SetColliderIsSensorCommand::new(handle, value),
                                )))
                                .unwrap();
                        }
                    }
                }
                _ => {}
            }
        }
    }
}
