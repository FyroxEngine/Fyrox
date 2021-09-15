use crate::sidebar::make_section;
use crate::{
    gui::{BuildContext, Ui, UiMessage, UiNode},
    physics::Collider,
    scene::commands::{
        physics::{SetCylinderHalfHeightCommand, SetCylinderRadiusCommand},
        SceneCommand,
    },
    send_sync_message,
    sidebar::{make_f32_input_field, make_text_mark, COLUMN_WIDTH, ROW_HEIGHT},
    Message,
};
use rg3d::{
    core::pool::Handle,
    gui::{
        grid::{Column, GridBuilder, Row},
        message::{MessageDirection, NumericUpDownMessage, UiMessageData},
        widget::WidgetBuilder,
    },
    physics3d::desc::CylinderDesc,
};
use std::sync::mpsc::Sender;

pub struct CylinderSection {
    pub section: Handle<UiNode>,
    half_height: Handle<UiNode>,
    radius: Handle<UiNode>,
    sender: Sender<Message>,
}

impl CylinderSection {
    pub fn new(ctx: &mut BuildContext, sender: Sender<Message>) -> Self {
        let half_height;
        let radius;
        let section = make_section(
            "Cylinder Properties",
            GridBuilder::new(
                WidgetBuilder::new()
                    .with_child(make_text_mark(ctx, "Half Height", 0))
                    .with_child({
                        half_height = make_f32_input_field(ctx, 0, 0.0, std::f32::MAX, 0.1);
                        half_height
                    })
                    .with_child(make_text_mark(ctx, "Radius", 1))
                    .with_child({
                        radius = make_f32_input_field(ctx, 1, 0.0, std::f32::MAX, 0.1);
                        radius
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
            half_height,
            radius,
        }
    }

    pub fn sync_to_model(&mut self, cylinder: &CylinderDesc, ui: &mut Ui) {
        send_sync_message(
            ui,
            NumericUpDownMessage::value(
                self.half_height,
                MessageDirection::ToWidget,
                cylinder.half_height,
            ),
        );

        send_sync_message(
            ui,
            NumericUpDownMessage::value(self.radius, MessageDirection::ToWidget, cylinder.radius),
        );
    }

    pub fn handle_message(
        &mut self,
        message: &UiMessage,
        cylinder: &CylinderDesc,
        handle: Handle<Collider>,
    ) {
        if let UiMessageData::NumericUpDown(NumericUpDownMessage::Value(value)) = *message.data() {
            if message.direction() == MessageDirection::FromWidget {
                if message.destination() == self.half_height && cylinder.half_height.ne(&value) {
                    self.sender
                        .send(Message::DoSceneCommand(
                            SceneCommand::SetCylinderHalfHeight(SetCylinderHalfHeightCommand::new(
                                handle, value,
                            )),
                        ))
                        .unwrap();
                } else if message.destination() == self.radius && cylinder.radius.ne(&value) {
                    self.sender
                        .send(Message::DoSceneCommand(SceneCommand::SetCylinderRadius(
                            SetCylinderRadiusCommand::new(handle, value),
                        )))
                        .unwrap();
                }
            }
        }
    }
}
