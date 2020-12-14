use crate::{
    gui::{BuildContext, Ui, UiMessage, UiNode},
    physics::Joint,
    scene::{SceneCommand, SetBallJointAnchor1Command, SetBallJointAnchor2Command},
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
    scene::physics::BallJointDesc,
};
use std::sync::mpsc::Sender;

pub struct BallJointSection {
    pub section: Handle<UiNode>,
    anchor1: Handle<UiNode>,
    anchor2: Handle<UiNode>,
    sender: Sender<Message>,
}

impl BallJointSection {
    pub fn new(ctx: &mut BuildContext, sender: Sender<Message>) -> Self {
        let anchor1;
        let anchor2;
        let section = GridBuilder::new(
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
        .build(ctx);

        Self {
            section,
            sender,
            anchor1,
            anchor2,
        }
    }

    pub fn sync_to_model(&mut self, ball: &BallJointDesc, ui: &mut Ui) {
        ui.send_message(Vec3EditorMessage::value(
            self.anchor1,
            MessageDirection::ToWidget,
            ball.local_anchor1,
        ));

        ui.send_message(Vec3EditorMessage::value(
            self.anchor2,
            MessageDirection::ToWidget,
            ball.local_anchor2,
        ));
    }

    pub fn handle_message(
        &mut self,
        message: &UiMessage,
        ball: &BallJointDesc,
        handle: Handle<Joint>,
    ) {
        if let UiMessageData::Vec3Editor(msg) = message.data() {
            if let &Vec3EditorMessage::Value(value) = msg {
                if message.direction() == MessageDirection::FromWidget {
                    if message.destination() == self.anchor1 && ball.local_anchor1.ne(&value) {
                        self.sender
                            .send(Message::DoSceneCommand(SceneCommand::SetBallJointAnchor1(
                                SetBallJointAnchor1Command::new(handle, value),
                            )))
                            .unwrap();
                    } else if message.destination() == self.anchor2 && ball.local_anchor2.ne(&value)
                    {
                        self.sender
                            .send(Message::DoSceneCommand(SceneCommand::SetBallJointAnchor2(
                                SetBallJointAnchor2Command::new(handle, value),
                            )))
                            .unwrap();
                    }
                }
            }
        }
    }
}
