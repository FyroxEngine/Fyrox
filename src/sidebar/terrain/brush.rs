use crate::gui::make_dropdown_list_option;
use crate::sidebar::make_section;
use crate::{
    send_sync_message,
    sidebar::{make_f32_input_field, make_text_mark, COLUMN_WIDTH, ROW_HEIGHT},
};
use rg3d::gui::message::UiMessage;
use rg3d::gui::numeric::NumericUpDownMessage;
use rg3d::gui::{BuildContext, UiNode, UserInterface};
use rg3d::{
    core::{pool::Handle, scope_profile},
    gui::{
        dropdown_list::DropdownListBuilder,
        grid::{Column, GridBuilder, Row},
        message::{DropdownListMessage, MessageDirection, UiMessageData},
        widget::WidgetBuilder,
    },
    scene::terrain::{Brush, BrushMode, BrushShape},
};
use std::sync::{Arc, Mutex};

pub struct BrushSection {
    pub section: Handle<UiNode>,
    kind: Handle<UiNode>,
    mode: Handle<UiNode>,
    width: Handle<UiNode>,
    length: Handle<UiNode>,
    radius: Handle<UiNode>,
    pub brush: Arc<Mutex<Brush>>,
}

impl BrushSection {
    pub fn new(ctx: &mut BuildContext) -> Self {
        let kind;
        let mode;
        let width;
        let length;
        let radius;
        let section = make_section(
            "Brush Properties",
            GridBuilder::new(
                WidgetBuilder::new()
                    .with_child(make_text_mark(ctx, "Brush Kind", 0))
                    .with_child({
                        kind =
                            DropdownListBuilder::new(WidgetBuilder::new().on_row(0).on_column(1))
                                .with_items(vec![
                                    make_dropdown_list_option(ctx, "Circle"),
                                    make_dropdown_list_option(ctx, "Rectangle"),
                                ])
                                .with_selected(0)
                                .build(ctx);
                        kind
                    })
                    .with_child(make_text_mark(ctx, "Brush Mode", 1))
                    .with_child({
                        mode =
                            DropdownListBuilder::new(WidgetBuilder::new().on_row(1).on_column(1))
                                .with_items(vec![
                                    make_dropdown_list_option(ctx, "Modify Height Map"),
                                    make_dropdown_list_option(ctx, "Draw On Mask"),
                                ])
                                .with_selected(0)
                                .build(ctx);
                        mode
                    })
                    .with_child(make_text_mark(ctx, "Brush Width", 2))
                    .with_child({
                        width = make_f32_input_field(ctx, 2, 0.0, f32::MAX, 0.1);
                        width
                    })
                    .with_child(make_text_mark(ctx, "Brush Height", 3))
                    .with_child({
                        length = make_f32_input_field(ctx, 3, 0.0, f32::MAX, 0.1);
                        length
                    })
                    .with_child(make_text_mark(ctx, "Brush Radius", 4))
                    .with_child({
                        radius = make_f32_input_field(ctx, 4, 0.0, f32::MAX, 0.1);
                        radius
                    }),
            )
            .add_column(Column::strict(COLUMN_WIDTH))
            .add_column(Column::stretch())
            .add_row(Row::strict(ROW_HEIGHT))
            .add_row(Row::strict(ROW_HEIGHT))
            .add_row(Row::strict(ROW_HEIGHT))
            .add_row(Row::strict(ROW_HEIGHT))
            .add_row(Row::strict(ROW_HEIGHT))
            .build(ctx),
            ctx,
        );

        Self {
            section,
            kind,
            mode,
            width,
            length,
            radius,
            brush: Arc::new(Mutex::new(Brush {
                center: Default::default(),
                shape: BrushShape::Circle { radius: 1.0 },
                mode: BrushMode::ModifyHeightMap { amount: 0.25 },
            })),
        }
    }

    pub fn sync_to_model(&mut self, ui: &mut UserInterface) {
        let brush = self.brush.lock().unwrap();

        match brush.shape {
            BrushShape::Circle { radius } => {
                send_sync_message(
                    ui,
                    DropdownListMessage::selection(self.kind, MessageDirection::ToWidget, Some(0)),
                );

                send_sync_message(
                    ui,
                    NumericUpDownMessage::value(self.radius, MessageDirection::ToWidget, radius),
                );
            }
            BrushShape::Rectangle { width, length } => {
                send_sync_message(
                    ui,
                    DropdownListMessage::selection(self.kind, MessageDirection::ToWidget, Some(1)),
                );

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

        match brush.mode {
            BrushMode::ModifyHeightMap { .. } => {
                send_sync_message(
                    ui,
                    DropdownListMessage::selection(self.mode, MessageDirection::ToWidget, Some(0)),
                );
            }
            BrushMode::DrawOnMask { .. } => {
                send_sync_message(
                    ui,
                    DropdownListMessage::selection(self.mode, MessageDirection::ToWidget, Some(1)),
                );
            }
        }
    }

    pub fn handle_message(&mut self, message: &UiMessage) {
        scope_profile!();

        let mut brush = self.brush.lock().unwrap();

        match message.data() {
            UiMessageData::DropdownList(DropdownListMessage::SelectionChanged(Some(selection))) => {
                if message.destination() == self.kind {
                    match selection {
                        0 => {
                            brush.shape = BrushShape::Circle { radius: 1.0 };
                        }
                        1 => {
                            brush.shape = BrushShape::Rectangle {
                                width: 0.5,
                                length: 0.5,
                            }
                        }
                        _ => unreachable!(),
                    }
                } else if message.destination() == self.mode {
                    match selection {
                        0 => brush.mode = BrushMode::ModifyHeightMap { amount: 0.25 },
                        1 => {
                            brush.mode = BrushMode::DrawOnMask {
                                layer: 0,
                                alpha: 1.0,
                            }
                        }
                        _ => unreachable!(),
                    }
                }
            }
            UiMessageData::User(msg) => {
                if let Some(&NumericUpDownMessage::Value(value)) =
                    msg.cast::<NumericUpDownMessage<f32>>()
                {
                    match &mut brush.shape {
                        BrushShape::Circle { radius } => {
                            if message.destination() == self.radius {
                                *radius = value;
                            }
                        }
                        BrushShape::Rectangle { width, length } => {
                            if message.destination() == self.length {
                                *length = value;
                            } else if message.destination() == self.width {
                                *width = value;
                            }
                        }
                    }
                }
            }
            _ => (),
        }
    }
}
