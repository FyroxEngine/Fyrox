use crate::scene::{
    SetRevoluteJointAnchor1Command, SetRevoluteJointAnchor2Command, SetRevoluteJointAxis1Command,
    SetRevoluteJointAxis2Command,
};
use crate::{
    gui::{BuildContext, Ui, UiMessage, UiNode},
    physics::Joint,
    scene::SceneCommand,
    sidebar::{make_text_mark, make_vec3_input_field, COLUMN_WIDTH, ROW_HEIGHT},
    Message,
};
use rg3d::scene::physics::RevoluteJointDesc;
use rg3d::{
    core::pool::Handle,
    gui::{
        grid::{Column, GridBuilder, Row},
        message::{MessageDirection, UiMessageData, Vec3EditorMessage},
        widget::WidgetBuilder,
    },
};
use std::sync::mpsc::Sender;

pub struct RevoluteJointSection {
    pub section: Handle<UiNode>,
    joint_anchor: Handle<UiNode>,
    joint_axis: Handle<UiNode>,
    connected_anchor: Handle<UiNode>,
    connected_axis: Handle<UiNode>,
    sender: Sender<Message>,
}

impl RevoluteJointSection {
    pub fn new(ctx: &mut BuildContext, sender: Sender<Message>) -> Self {
        let joint_anchor;
        let joint_axis;
        let connected_anchor;
        let connected_axis;
        let section = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(make_text_mark(ctx, "Joint Anchor", 0))
                .with_child({
                    joint_anchor = make_vec3_input_field(ctx, 0);
                    joint_anchor
                })
                .with_child(make_text_mark(ctx, "Joint Axis", 1))
                .with_child({
                    joint_axis = make_vec3_input_field(ctx, 1);
                    joint_axis
                })
                .with_child(make_text_mark(ctx, "Connected Anchor", 2))
                .with_child({
                    connected_anchor = make_vec3_input_field(ctx, 2);
                    connected_anchor
                })
                .with_child(make_text_mark(ctx, "Connected Axis", 3))
                .with_child({
                    connected_axis = make_vec3_input_field(ctx, 3);
                    connected_axis
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
            joint_anchor,
            joint_axis,
            connected_anchor,
            connected_axis,
        }
    }

    pub fn sync_to_model(&mut self, revolute: &RevoluteJointDesc, ui: &mut Ui) {
        ui.send_message(Vec3EditorMessage::value(
            self.joint_anchor,
            MessageDirection::ToWidget,
            revolute.local_anchor1,
        ));

        ui.send_message(Vec3EditorMessage::value(
            self.joint_axis,
            MessageDirection::ToWidget,
            revolute.local_axis1,
        ));

        ui.send_message(Vec3EditorMessage::value(
            self.connected_anchor,
            MessageDirection::ToWidget,
            revolute.local_anchor2,
        ));

        ui.send_message(Vec3EditorMessage::value(
            self.connected_axis,
            MessageDirection::ToWidget,
            revolute.local_axis2,
        ));
    }

    pub fn handle_message(
        &mut self,
        message: &UiMessage,
        revolute: &RevoluteJointDesc,
        handle: Handle<Joint>,
    ) {
        if let UiMessageData::Vec3Editor(Vec3EditorMessage::Value(value)) = *message.data() {
            if message.direction() == MessageDirection::FromWidget {
                if message.destination() == self.joint_anchor && revolute.local_anchor1.ne(&value) {
                    self.sender
                        .send(Message::DoSceneCommand(
                            SceneCommand::SetRevoluteJointAnchor1(
                                SetRevoluteJointAnchor1Command::new(handle, value),
                            ),
                        ))
                        .unwrap();
                } else if message.destination() == self.joint_axis
                    && revolute.local_axis1.ne(&value)
                {
                    self.sender
                        .send(Message::DoSceneCommand(
                            SceneCommand::SetRevoluteJointAxis1(SetRevoluteJointAxis1Command::new(
                                handle, value,
                            )),
                        ))
                        .unwrap();
                } else if message.destination() == self.connected_anchor
                    && revolute.local_anchor2.ne(&value)
                {
                    self.sender
                        .send(Message::DoSceneCommand(
                            SceneCommand::SetRevoluteJointAnchor2(
                                SetRevoluteJointAnchor2Command::new(handle, value),
                            ),
                        ))
                        .unwrap();
                } else if message.destination() == self.connected_axis
                    && revolute.local_axis2.ne(&value)
                {
                    self.sender
                        .send(Message::DoSceneCommand(
                            SceneCommand::SetRevoluteJointAxis2(SetRevoluteJointAxis2Command::new(
                                handle, value,
                            )),
                        ))
                        .unwrap();
                }
            }
        }
    }
}
