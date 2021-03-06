use crate::sidebar::physics::joint::prismatic::PrismaticJointSection;
use crate::{
    gui::{BuildContext, Ui, UiMessage, UiNode},
    physics::{Joint, RigidBody},
    scene::{SceneCommand, SetJointConnectedBodyCommand},
    send_sync_message,
    sidebar::{
        make_text_mark,
        physics::joint::{
            ball::BallJointSection, fixed::FixedJointSection, revolute::RevoluteJointSection,
        },
        COLUMN_WIDTH, ROW_HEIGHT,
    },
    Message,
};
use rg3d::core::BiDirHashMap;
use rg3d::{
    core::{color::Color, pool::Handle},
    gui::{
        border::BorderBuilder,
        brush::Brush,
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
use std::sync::mpsc::Sender;

mod ball;
mod fixed;
mod prismatic;
mod revolute;

pub struct JointSection {
    pub section: Handle<UiNode>,
    connected_body: Handle<UiNode>,
    connected_body_text: Handle<UiNode>,
    sender: Sender<Message>,
    ball_section: BallJointSection,
    fixed_section: FixedJointSection,
    revolute_section: RevoluteJointSection,
    prismatic_section: PrismaticJointSection,
    available_bodies: Vec<Handle<RigidBody>>,
}

impl JointSection {
    pub fn new(ctx: &mut BuildContext, sender: Sender<Message>) -> Self {
        let connected_body;
        let connected_body_text;
        let ball_section = BallJointSection::new(ctx, sender.clone());
        let fixed_section = FixedJointSection::new(ctx, sender.clone());
        let revolute_section = RevoluteJointSection::new(ctx, sender.clone());
        let prismatic_section = PrismaticJointSection::new(ctx, sender.clone());
        let section = StackPanelBuilder::new(
            WidgetBuilder::new().with_children(&[
                GridBuilder::new(
                    WidgetBuilder::new()
                        .with_child({
                            connected_body_text = make_text_mark(ctx, "Connected Body", 0);
                            connected_body_text
                        })
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
                revolute_section.section,
                prismatic_section.section,
            ]),
        )
        .build(ctx);

        Self {
            section,
            sender,
            connected_body_text,
            connected_body,
            ball_section,
            fixed_section,
            revolute_section,
            prismatic_section,
            available_bodies: Default::default(),
        }
    }

    pub fn sync_to_model(
        &mut self,
        joint: &Joint,
        graph: &Graph,
        binder: &BiDirHashMap<Handle<Node>, Handle<RigidBody>>,
        ui: &mut Ui,
    ) {
        fn toggle_visibility(ui: &mut Ui, destination: Handle<UiNode>, value: bool) {
            send_sync_message(
                ui,
                WidgetMessage::visibility(destination, MessageDirection::ToWidget, value),
            );
        };

        toggle_visibility(ui, self.ball_section.section, false);
        toggle_visibility(ui, self.fixed_section.section, false);
        toggle_visibility(ui, self.revolute_section.section, false);
        toggle_visibility(ui, self.prismatic_section.section, false);

        match &joint.params {
            JointParamsDesc::BallJoint(ball) => {
                toggle_visibility(ui, self.ball_section.section, true);
                self.ball_section.sync_to_model(ball, ui);
            }
            JointParamsDesc::FixedJoint(fixed) => {
                toggle_visibility(ui, self.fixed_section.section, true);
                self.fixed_section.sync_to_model(fixed, ui);
            }
            JointParamsDesc::PrismaticJoint(prismatic) => {
                toggle_visibility(ui, self.prismatic_section.section, true);
                self.prismatic_section.sync_to_model(prismatic, ui);
            }
            JointParamsDesc::RevoluteJoint(revolute) => {
                toggle_visibility(ui, self.revolute_section.section, true);
                self.revolute_section.sync_to_model(revolute, ui);
            }
        };

        self.available_bodies.clear();
        let mut items = Vec::new();
        let ctx = &mut ui.build_ctx();
        let mut connected_index = None;
        for (handle, node) in graph.pair_iter() {
            if let Some(&body) = binder.value_of(&handle) {
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

                if body == joint.body2.into() {
                    connected_index = Some(items.len() - 1);
                }
            }
        }

        send_sync_message(
            ui,
            DropdownListMessage::items(self.connected_body, MessageDirection::ToWidget, items),
        );

        send_sync_message(
            ui,
            DropdownListMessage::selection(
                self.connected_body,
                MessageDirection::ToWidget,
                connected_index,
            ),
        );

        let brush = if ui
            .node(self.connected_body)
            .as_dropdown_list()
            .selection()
            .is_some()
        {
            Brush::Solid(Color::WHITE)
        } else {
            Brush::Solid(Color::RED)
        };

        send_sync_message(
            ui,
            WidgetMessage::foreground(self.connected_body_text, MessageDirection::ToWidget, brush),
        );
    }

    pub fn handle_message(&mut self, message: &UiMessage, joint: &Joint, handle: Handle<Joint>) {
        match &joint.params {
            JointParamsDesc::BallJoint(ball) => {
                self.ball_section.handle_message(message, ball, handle);
            }
            JointParamsDesc::FixedJoint(fixed) => {
                self.fixed_section.handle_message(message, fixed, handle);
            }
            JointParamsDesc::PrismaticJoint(prismatic) => {
                self.prismatic_section
                    .handle_message(message, prismatic, handle);
            }
            JointParamsDesc::RevoluteJoint(revolute) => {
                self.revolute_section
                    .handle_message(message, revolute, handle);
            }
        }

        if let UiMessageData::DropdownList(DropdownListMessage::SelectionChanged(Some(index))) =
            *message.data()
        {
            if message.direction() == MessageDirection::FromWidget
                && message.destination() == self.connected_body
            {
                let body = self.available_bodies[index];
                if joint.body2.ne(&body.into()) {
                    self.sender
                        .send(Message::DoSceneCommand(
                            SceneCommand::SetJointConnectedBody(SetJointConnectedBodyCommand::new(
                                handle,
                                body.into(),
                            )),
                        ))
                        .unwrap();
                }
            }
        }
    }
}
