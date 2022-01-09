use crate::settings::{make_bool_input_field, make_text_mark};
use fyrox::{
    core::pool::Handle,
    gui::{
        check_box::CheckBoxMessage,
        grid::{Column, GridBuilder, Row},
        message::{MessageDirection, UiMessage},
        widget::WidgetBuilder,
        BuildContext, UiNode, UserInterface,
    },
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, PartialEq, Clone)]
pub struct DebuggingSettings {
    pub show_physics: bool,
    pub show_bounds: bool,
    pub show_tbn: bool,
}

impl Default for DebuggingSettings {
    fn default() -> Self {
        Self {
            show_physics: true,
            show_bounds: true,
            show_tbn: false,
        }
    }
}

pub struct DebuggingSection {
    pub section: Handle<UiNode>,
    show_physics: Handle<UiNode>,
    show_bounds: Handle<UiNode>,
    show_tbn: Handle<UiNode>,
}

impl DebuggingSection {
    pub fn new(ctx: &mut BuildContext, settings: &DebuggingSettings) -> Self {
        let show_physics;
        let show_bounds;
        let show_tbn;
        let section = GridBuilder::new(
            WidgetBuilder::new()
                .with_visibility(false)
                .with_child(make_text_mark(ctx, "Show Physics", 0))
                .with_child({
                    show_physics = make_bool_input_field(ctx, 0, settings.show_physics);
                    show_physics
                })
                .with_child(make_text_mark(ctx, "Show Bounds", 1))
                .with_child({
                    show_bounds = make_bool_input_field(ctx, 1, settings.show_bounds);
                    show_bounds
                })
                .with_child(make_text_mark(ctx, "Show TBN", 2))
                .with_child({
                    show_tbn = make_bool_input_field(ctx, 2, settings.show_tbn);
                    show_tbn
                }),
        )
        .add_row(Row::strict(25.0))
        .add_row(Row::strict(25.0))
        .add_row(Row::strict(25.0))
        .add_row(Row::stretch())
        .add_row(Row::stretch())
        .add_column(Column::strict(120.0))
        .add_column(Column::stretch())
        .build(ctx);

        Self {
            section,
            show_bounds,
            show_physics,
            show_tbn,
        }
    }

    pub fn sync_to_model(&self, ui: &UserInterface, settings: &DebuggingSettings) {
        ui.send_message(CheckBoxMessage::checked(
            self.show_tbn,
            MessageDirection::ToWidget,
            Some(settings.show_tbn),
        ));

        ui.send_message(CheckBoxMessage::checked(
            self.show_physics,
            MessageDirection::ToWidget,
            Some(settings.show_physics),
        ));

        ui.send_message(CheckBoxMessage::checked(
            self.show_bounds,
            MessageDirection::ToWidget,
            Some(settings.show_bounds),
        ));
    }

    pub fn handle_message(&mut self, message: &UiMessage, settings: &mut DebuggingSettings) {
        if let Some(&CheckBoxMessage::Check(Some(value))) = message.data::<CheckBoxMessage>() {
            if message.destination() == self.show_bounds {
                settings.show_bounds = value;
            } else if message.destination() == self.show_tbn {
                settings.show_tbn = value;
            } else if message.destination() == self.show_physics {
                settings.show_physics = value;
            }
        }
    }
}
