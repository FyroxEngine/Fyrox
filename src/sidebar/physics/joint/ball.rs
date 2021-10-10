use crate::sidebar::make_section;
use crate::{
    physics::Joint,
    scene::commands::physics::{SetBallJointAnchor1Command, SetBallJointAnchor2Command},
    send_sync_message,
    sidebar::{make_text_mark, make_vec3_input_field, COLUMN_WIDTH, ROW_HEIGHT},
    Message,
};
use rg3d::gui::message::UiMessage;
use rg3d::gui::vec::vec3::Vec3EditorMessage;
use rg3d::gui::{BuildContext, UiNode, UserInterface};
use rg3d::{
    core::pool::Handle,
    gui::{
        grid::{Column, GridBuilder, Row},
        message::{MessageDirection, UiMessageData},
        widget::WidgetBuilder,
    },
    physics3d::desc::BallJointDesc,
};
use std::sync::mpsc::Sender;

pub struct BallJointSection {
    pub section: Handle<UiNode>,
    joint_anchor: Handle<UiNode>,
    connected_anchor: Handle<UiNode>,
    sender: Sender<Message>,
}

impl BallJointSection {
    pub fn new(ctx: &mut BuildContext, sender: Sender<Message>) -> Self {
        let anchor1;
        let anchor2;
        let section = make_section(
            "Ball Joint Properties",
            GridBuilder::new(
                WidgetBuilder::new()
                    .with_child(make_text_mark(ctx, "Joint Anchor", 0))
                    .with_child({
                        anchor1 = make_vec3_input_field(ctx, 0);
                        anchor1
                    })
                    .with_child(make_text_mark(ctx, "Connected Anchor", 1))
                    .with_child({
                        anchor2 = make_vec3_input_field(ctx, 1);
                        anchor2
                    }),
            )
            .add_column(Column::strict(COLUMN_WIDTH))
            .add_column(Column::stretch())
            .add_row(Row::strict(ROW_HEIGHT))
            .add_row(Row::strict(ROW_HEIGHT))
            .build(ctx),
            ctx,
        );

        Self {
            section,
            sender,
            joint_anchor: anchor1,
            connected_anchor: anchor2,
        }
    }

    pub fn sync_to_model(&mut self, ball: &BallJointDesc, ui: &mut UserInterface) {
        send_sync_message(
            ui,
            Vec3EditorMessage::value(
                self.joint_anchor,
                MessageDirection::ToWidget,
                ball.local_anchor1,
            ),
        );

        send_sync_message(
            ui,
            Vec3EditorMessage::value(
                self.connected_anchor,
                MessageDirection::ToWidget,
                ball.local_anchor2,
            ),
        );
    }

    pub fn handle_message(
        &mut self,
        message: &UiMessage,
        ball: &BallJointDesc,
        handle: Handle<Joint>,
    ) {
        if let UiMessageData::User(msg) = message.data() {
            if let Some(&Vec3EditorMessage::Value(value)) = msg.cast::<Vec3EditorMessage<f32>>() {
                if message.direction() == MessageDirection::FromWidget {
                    if message.destination() == self.joint_anchor && ball.local_anchor1.ne(&value) {
                        self.sender
                            .send(Message::do_scene_command(SetBallJointAnchor1Command::new(
                                handle, value,
                            )))
                            .unwrap();
                    } else if message.destination() == self.connected_anchor
                        && ball.local_anchor2.ne(&value)
                    {
                        self.sender
                            .send(Message::do_scene_command(SetBallJointAnchor2Command::new(
                                handle, value,
                            )))
                            .unwrap();
                    }
                }
            }
        }
    }
}
