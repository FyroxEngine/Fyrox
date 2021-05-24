use crate::{
    gui::{BuildContext, Ui, UiMessage, UiNode},
    physics::Joint,
    scene::commands::{
        physics::{
            SetPrismaticJointAnchor1Command, SetPrismaticJointAnchor2Command,
            SetPrismaticJointAxis1Command, SetPrismaticJointAxis2Command,
        },
        SceneCommand,
    },
    send_sync_message,
    sidebar::{make_text_mark, make_vec3_input_field, COLUMN_WIDTH, ROW_HEIGHT},
    Message,
};
use rg3d::{
    core::pool::Handle,
    gui::{
        grid::{Column, GridBuilder, Row},
        message::{MessageDirection, UiMessageData, Vec3EditorMessage},
        widget::WidgetBuilder,
    },
    scene::physics::PrismaticJointDesc,
};
use std::sync::mpsc::Sender;

pub struct PrismaticJointSection {
    pub section: Handle<UiNode>,
    joint_anchor: Handle<UiNode>,
    joint_axis: Handle<UiNode>,
    connected_anchor: Handle<UiNode>,
    connected_axis: Handle<UiNode>,
    sender: Sender<Message>,
}

impl PrismaticJointSection {
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

    pub fn sync_to_model(&mut self, prismatic: &PrismaticJointDesc, ui: &mut Ui) {
        send_sync_message(
            ui,
            Vec3EditorMessage::value(
                self.joint_anchor,
                MessageDirection::ToWidget,
                prismatic.local_anchor1,
            ),
        );

        send_sync_message(
            ui,
            Vec3EditorMessage::value(
                self.joint_axis,
                MessageDirection::ToWidget,
                prismatic.local_axis1,
            ),
        );

        send_sync_message(
            ui,
            Vec3EditorMessage::value(
                self.connected_anchor,
                MessageDirection::ToWidget,
                prismatic.local_anchor2,
            ),
        );

        send_sync_message(
            ui,
            Vec3EditorMessage::value(
                self.connected_axis,
                MessageDirection::ToWidget,
                prismatic.local_axis2,
            ),
        );
    }

    pub fn handle_message(
        &mut self,
        message: &UiMessage,
        prismatic: &PrismaticJointDesc,
        handle: Handle<Joint>,
    ) {
        if let UiMessageData::Vec3Editor(Vec3EditorMessage::Value(value)) = *message.data() {
            if message.direction() == MessageDirection::FromWidget {
                if message.destination() == self.joint_anchor && prismatic.local_anchor1.ne(&value)
                {
                    self.sender
                        .send(Message::DoSceneCommand(
                            SceneCommand::SetPrismaticJointAnchor1(
                                SetPrismaticJointAnchor1Command::new(handle, value),
                            ),
                        ))
                        .unwrap();
                } else if message.destination() == self.joint_axis
                    && prismatic.local_axis1.ne(&value)
                {
                    self.sender
                        .send(Message::DoSceneCommand(
                            SceneCommand::SetPrismaticJointAxis1(
                                SetPrismaticJointAxis1Command::new(handle, value),
                            ),
                        ))
                        .unwrap();
                } else if message.destination() == self.connected_anchor
                    && prismatic.local_anchor2.ne(&value)
                {
                    self.sender
                        .send(Message::DoSceneCommand(
                            SceneCommand::SetPrismaticJointAnchor2(
                                SetPrismaticJointAnchor2Command::new(handle, value),
                            ),
                        ))
                        .unwrap();
                } else if message.destination() == self.connected_axis
                    && prismatic.local_axis2.ne(&value)
                {
                    self.sender
                        .send(Message::DoSceneCommand(
                            SceneCommand::SetPrismaticJointAxis2(
                                SetPrismaticJointAxis2Command::new(handle, value),
                            ),
                        ))
                        .unwrap();
                }
            }
        }
    }
}
