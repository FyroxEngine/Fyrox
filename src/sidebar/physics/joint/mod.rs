use crate::sidebar::physics::joint::fixed::FixedJointSection;
use crate::{
    gui::{BuildContext, Ui, UiMessage, UiNode},
    physics::{Joint, RigidBody},
    scene::{SceneCommand, SetJointConnectedBodyCommand},
    sidebar::{make_text_mark, physics::joint::ball::BallJointSection, COLUMN_WIDTH, ROW_HEIGHT},
    Message,
};
use rg3d::{
    core::pool::Handle,
    gui::{
        border::BorderBuilder,
        decorator::DecoratorBuilder,
        dropdown_list::DropdownListBuilder,
        grid::{Column, GridBuilder, Row},
        message::{DropdownListMessage, MessageDirection, UiMessageData, WidgetMessage},
        stack_panel::StackPanelBuilder,
        text::TextBuilder,
        widget::WidgetBuilder,
        HorizontalAlignment, VerticalAlignment,
    },
    scene::{graph::Graph, node::Node, physics::JointParamsDesc},
};
use std::{collections::HashMap, sync::mpsc::Sender};

mod ball;
mod fixed;

pub struct JointSection {
    pub section: Handle<UiNode>,
    connected_body: Handle<UiNode>,
    sender: Sender<Message>,
    ball_section: BallJointSection,
    fixed_section: FixedJointSection,
    available_bodies: Vec<Handle<RigidBody>>,
}

impl JointSection {
    pub fn new(ctx: &mut BuildContext, sender: Sender<Message>) -> Self {
        let connected_body;
        let ball_section = BallJointSection::new(ctx, sender.clone());
        let fixed_section = FixedJointSection::new(ctx, sender.clone());
        let section = StackPanelBuilder::new(
            WidgetBuilder::new().with_children(&[
                GridBuilder::new(
                    WidgetBuilder::new()
                        .with_child(make_text_mark(ctx, "Connected Body", 0))
                        .with_child({
                            connected_body =
                                DropdownListBuilder::new(WidgetBuilder::new().on_column(1))
                                    .build(ctx);
                            connected_body
                        }),
                )
                .add_column(Column::strict(COLUMN_WIDTH))
                .add_column(Column::stretch())
                .add_row(Row::strict(ROW_HEIGHT))
                .build(ctx),
                ball_section.section,
                fixed_section.section,
            ]),
        )
        .build(ctx);

        Self {
            section,
            sender,
            connected_body,
            ball_section,
            fixed_section,
            available_bodies: Default::default(),
        }
    }

    pub fn sync_to_model(
        &mut self,
        joint: &Joint,
        graph: &Graph,
        binder: &HashMap<Handle<Node>, Handle<RigidBody>>,
        ui: &mut Ui,
    ) {
        fn toggle_visibility(ui: &mut Ui, destination: Handle<UiNode>, value: bool) {
            ui.send_message(WidgetMessage::visibility(
                destination,
                MessageDirection::ToWidget,
                value,
            ));
        };

        toggle_visibility(ui, self.ball_section.section, false);
        toggle_visibility(ui, self.fixed_section.section, false);

        match &joint.params {
            JointParamsDesc::BallJoint(ball) => {
                toggle_visibility(ui, self.ball_section.section, true);
                self.ball_section.sync_to_model(ball, ui);
            }
            JointParamsDesc::FixedJoint(fixed) => {
                toggle_visibility(ui, self.fixed_section.section, true);
                self.fixed_section.sync_to_model(fixed, ui);
            }
            JointParamsDesc::PrismaticJoint(_) => {}
            JointParamsDesc::RevoluteJoint(_) => {}
        };

        self.available_bodies.clear();
        let mut items = Vec::new();
        let ctx = &mut ui.build_ctx();
        for (handle, node) in graph.pair_iter() {
            if let Some(&body) = binder.get(&handle) {
                if body != joint.body1.into() {
                    let item = DecoratorBuilder::new(BorderBuilder::new(
                        WidgetBuilder::new().with_height(26.0).with_child(
                            TextBuilder::new(WidgetBuilder::new())
                                .with_vertical_text_alignment(VerticalAlignment::Center)
                                .with_horizontal_text_alignment(HorizontalAlignment::Center)
                                .with_text(node.name())
                                .build(ctx),
                        ),
                    ))
                    .build(ctx);
                    self.available_bodies.push(body);
                    items.push(item);
                }
            }
        }

        ui.send_message(DropdownListMessage::items(
            self.connected_body,
            MessageDirection::ToWidget,
            items,
        ));
    }

    pub fn handle_message(&mut self, message: &UiMessage, joint: &Joint, handle: Handle<Joint>) {
        match &joint.params {
            JointParamsDesc::BallJoint(ball) => {
                self.ball_section.handle_message(message, ball, handle);
            }
            JointParamsDesc::FixedJoint(fixed) => {
                self.fixed_section.handle_message(message, fixed, handle);
            }
            JointParamsDesc::PrismaticJoint(_) => (),
            JointParamsDesc::RevoluteJoint(_) => (),
        }

        if let UiMessageData::DropdownList(msg) = message.data() {
            if let &DropdownListMessage::SelectionChanged(value) = msg {
                if let Some(index) = value {
                    if message.direction() == MessageDirection::FromWidget {
                        if message.destination() == self.connected_body {
                            let body = self.available_bodies[index];
                            if joint.body2.ne(&body.into()) {
                                self.sender
                                    .send(Message::DoSceneCommand(
                                        SceneCommand::SetJointConnectedBody(
                                            SetJointConnectedBodyCommand::new(handle, body.into()),
                                        ),
                                    ))
                                    .unwrap();
                            }
                        }
                    }
                }
            }
        }
    }
}
