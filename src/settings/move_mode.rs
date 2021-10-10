use crate::settings::{make_bool_input_field, make_f32_input_field, make_text_mark};
use rg3d::gui::message::UiMessage;
use rg3d::gui::numeric::NumericUpDownMessage;
use rg3d::gui::{BuildContext, UiNode, UserInterface};
use rg3d::{
    core::pool::Handle,
    gui::{
        grid::{Column, GridBuilder, Row},
        message::{CheckBoxMessage, MessageDirection, UiMessageData},
        widget::WidgetBuilder,
    },
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, PartialEq, Clone)]
pub struct MoveInteractionModeSettings {
    pub grid_snapping: bool,
    pub x_snap_step: f32,
    pub y_snap_step: f32,
    pub z_snap_step: f32,
}

impl Default for MoveInteractionModeSettings {
    fn default() -> Self {
        Self {
            grid_snapping: false,
            x_snap_step: 0.05,
            y_snap_step: 0.05,
            z_snap_step: 0.05,
        }
    }
}

pub struct MoveModeSection {
    pub section: Handle<UiNode>,
    snapping: Handle<UiNode>,
    x_snap_step: Handle<UiNode>,
    y_snap_step: Handle<UiNode>,
    z_snap_step: Handle<UiNode>,
}

impl MoveModeSection {
    pub fn new(ctx: &mut BuildContext, settings: &MoveInteractionModeSettings) -> Self {
        let snapping;
        let x_snap_step;
        let y_snap_step;
        let z_snap_step;
        let section = GridBuilder::new(
            WidgetBuilder::new()
                .with_visibility(false)
                .with_child(make_text_mark(ctx, "Snapping", 0))
                .with_child({
                    snapping = make_bool_input_field(ctx, 0, settings.grid_snapping);
                    snapping
                })
                .with_child(make_text_mark(ctx, "X Snap Step", 1))
                .with_child({
                    x_snap_step = make_f32_input_field(ctx, 1, settings.x_snap_step, 0.001);
                    x_snap_step
                })
                .with_child(make_text_mark(ctx, "Y Snap Step", 2))
                .with_child({
                    y_snap_step = make_f32_input_field(ctx, 2, settings.y_snap_step, 0.001);
                    y_snap_step
                })
                .with_child(make_text_mark(ctx, "Z Snap Step", 3))
                .with_child({
                    z_snap_step = make_f32_input_field(ctx, 3, settings.z_snap_step, 0.001);
                    z_snap_step
                }),
        )
        .add_row(Row::strict(25.0))
        .add_row(Row::strict(25.0))
        .add_row(Row::strict(25.0))
        .add_row(Row::strict(25.0))
        .add_row(Row::stretch())
        .add_column(Column::strict(120.0))
        .add_column(Column::stretch())
        .build(ctx);

        Self {
            section,
            snapping,
            x_snap_step,
            y_snap_step,
            z_snap_step,
        }
    }

    pub fn sync_to_model(&self, ui: &UserInterface, settings: &MoveInteractionModeSettings) {
        for &(node, value) in &[
            (self.x_snap_step, settings.x_snap_step),
            (self.y_snap_step, settings.y_snap_step),
            (self.z_snap_step, settings.z_snap_step),
        ] {
            ui.send_message(NumericUpDownMessage::value(
                node,
                MessageDirection::ToWidget,
                value,
            ));
        }

        ui.send_message(CheckBoxMessage::checked(
            self.snapping,
            MessageDirection::ToWidget,
            Some(settings.grid_snapping),
        ));
    }

    pub fn handle_message(
        &mut self,
        message: &UiMessage,
        settings: &mut MoveInteractionModeSettings,
    ) {
        match message.data() {
            UiMessageData::User(msg) if message.direction() == MessageDirection::FromWidget => {
                if let Some(&NumericUpDownMessage::Value(value)) =
                    msg.cast::<NumericUpDownMessage<f32>>()
                {
                    if message.destination() == self.x_snap_step {
                        settings.x_snap_step = value;
                    } else if message.destination() == self.y_snap_step {
                        settings.y_snap_step = value;
                    } else if message.destination() == self.z_snap_step {
                        settings.z_snap_step = value;
                    }
                }
            }
            &UiMessageData::CheckBox(CheckBoxMessage::Check(Some(value))) => {
                if message.destination() == self.snapping {
                    settings.grid_snapping = value;
                }
            }
            _ => {}
        }
    }
}
