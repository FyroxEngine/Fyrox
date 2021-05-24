use crate::{
    gui::{BuildContext, Ui, UiMessage, UiNode},
    send_sync_message,
    sidebar::{make_f32_input_field, make_text_mark, COLUMN_WIDTH, ROW_HEIGHT},
};
use rg3d::{
    core::{pool::Handle, scope_profile},
    gui::{
        grid::{Column, GridBuilder, Row},
        message::{MessageDirection, NumericUpDownMessage},
        widget::WidgetBuilder,
    },
    scene::terrain::{Brush, BrushKind},
};

pub struct BrushSection {
    pub section: Handle<UiNode>,
    width: Handle<UiNode>,
    length: Handle<UiNode>,
    radius: Handle<UiNode>,
}

impl BrushSection {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let width;
        let length;
        let radius;
        let section = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(make_text_mark(ctx, "Brush Width", 0))
                .with_child({
                    width = make_f32_input_field(ctx, 0, 0.0, f32::MAX, 0.1);
                    width
                })
                .with_child(make_text_mark(ctx, "Brush Height", 1))
                .with_child({
                    length = make_f32_input_field(ctx, 1, 0.0, f32::MAX, 0.1);
                    length
                })
                .with_child(make_text_mark(ctx, "Brush Radius", 2))
                .with_child({
                    radius = make_f32_input_field(ctx, 2, 0.0, f32::MAX, 0.1);
                    radius
                }),
        )
        .add_column(Column::strict(COLUMN_WIDTH))
        .add_column(Column::stretch())
        .add_row(Row::strict(ROW_HEIGHT))
        .add_row(Row::strict(ROW_HEIGHT))
        .add_row(Row::strict(ROW_HEIGHT))
        .build(ctx);

        Self {
            section,
            width,
            length,
            radius,
        }
    }

    pub fn sync_to_model(&mut self, brush: &Brush, ui: &mut Ui) {
        match brush.kind {
            BrushKind::Circle { radius } => {
                send_sync_message(
                    ui,
                    NumericUpDownMessage::value(self.radius, MessageDirection::ToWidget, radius),
                );
            }
            BrushKind::Rectangle { width, length } => {
                send_sync_message(
                    ui,
                    NumericUpDownMessage::value(self.width, MessageDirection::ToWidget, width),
                );

                send_sync_message(
                    ui,
                    NumericUpDownMessage::value(self.length, MessageDirection::ToWidget, length),
                );
            }
        }
    }

    pub fn handle_message(&mut self, message: &UiMessage, brush: &mut Brush) {
        scope_profile!();
    }
}
