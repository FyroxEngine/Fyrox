use crate::{
    border::BorderBuilder,
    brush::Brush,
    button::ButtonBuilder,
    core::{color::Color, pool::Handle},
    decorator::DecoratorBuilder,
    formatted_text::WrapMode,
    grid::{Column, GridBuilder, Row},
    message::{
        ButtonMessage, KeyCode, MessageDirection, NumericUpDownMessage, TextBoxMessage, UiMessage,
        UiMessageData, WidgetMessage,
    },
    text_box::{TextBox, TextBoxBuilder},
    utils::{make_arrow, ArrowDirection},
    widget::{Widget, WidgetBuilder},
    BuildContext, Control, HorizontalAlignment, NodeHandleMapping, Thickness, UiNode,
    UserInterface, VerticalAlignment, BRUSH_DARK, BRUSH_LIGHT,
};
use std::ops::{Deref, DerefMut};

#[derive(Clone)]
pub struct NumericUpDown {
    widget: Widget,
    field: Handle<UiNode>,
    increase: Handle<UiNode>,
    decrease: Handle<UiNode>,
    value: f32,
    step: f32,
    min_value: f32,
    max_value: f32,
    precision: usize,
}

crate::define_widget_deref!(NumericUpDown);

impl NumericUpDown {
    fn try_parse_value(&mut self, ui: &mut UserInterface) {
        // Parse input only when focus is lost from text field.
        if let Some(field) = ui.node(self.field).cast::<TextBox>() {
            if let Ok(value) = field.text().parse::<f32>() {
                let value = value.min(self.max_value).max(self.min_value);
                ui.send_message(NumericUpDownMessage::value(
                    self.handle(),
                    MessageDirection::ToWidget,
                    value,
                ));
            }
        }
    }
}

impl Control for NumericUpDown {
    fn resolve(&mut self, node_map: &NodeHandleMapping) {
        node_map.resolve(&mut self.field);
        node_map.resolve(&mut self.increase);
        node_map.resolve(&mut self.decrease);
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        match &message.data() {
            UiMessageData::Widget(msg) => {
                if message.destination() == self.field {
                    match msg {
                        WidgetMessage::LostFocus => {
                            self.try_parse_value(ui);
                        }
                        WidgetMessage::KeyDown(KeyCode::Return) => {
                            self.try_parse_value(ui);
                        }
                        _ => {}
                    }
                }
            }
            &UiMessageData::NumericUpDown(NumericUpDownMessage::Value(value))
                if message.direction() == MessageDirection::ToWidget
                    && message.destination() == self.handle() =>
            {
                let clamped = value.min(self.max_value).max(self.min_value);
                if self.value != clamped {
                    self.value = clamped;

                    // Sync text field.
                    ui.send_message(TextBoxMessage::text(
                        self.field,
                        MessageDirection::ToWidget,
                        format!("{:.1$}", self.value, self.precision),
                    ));

                    let mut msg = NumericUpDownMessage::value(
                        self.handle,
                        MessageDirection::FromWidget,
                        self.value,
                    );
                    // We must maintain flags
                    msg.set_handled(message.handled());
                    msg.flags = message.flags;
                    ui.send_message(msg);
                }
            }
            UiMessageData::Button(ButtonMessage::Click) => {
                if message.destination() == self.decrease {
                    let value = (self.value - self.step)
                        .min(self.max_value)
                        .max(self.min_value);
                    ui.send_message(NumericUpDownMessage::value(
                        self.handle(),
                        MessageDirection::ToWidget,
                        value,
                    ));
                } else if message.destination() == self.increase {
                    let value = (self.value + self.step)
                        .min(self.max_value)
                        .max(self.min_value);
                    ui.send_message(NumericUpDownMessage::value(
                        self.handle(),
                        MessageDirection::ToWidget,
                        value,
                    ));
                }
            }
            _ => {}
        }
    }
}

pub struct NumericUpDownBuilder {
    widget_builder: WidgetBuilder,
    value: f32,
    step: f32,
    min_value: f32,
    max_value: f32,
    precision: usize,
}

pub fn make_button(ctx: &mut BuildContext, arrow: ArrowDirection, row: usize) -> Handle<UiNode> {
    ButtonBuilder::new(
        WidgetBuilder::new()
            .with_margin(Thickness::right(1.0))
            .on_row(row),
    )
    .with_back(
        DecoratorBuilder::new(BorderBuilder::new(
            WidgetBuilder::new().with_foreground(Brush::Solid(Color::opaque(90, 90, 90))),
        ))
        .with_normal_brush(Brush::Solid(Color::opaque(60, 60, 60)))
        .with_hover_brush(Brush::Solid(Color::opaque(80, 80, 80)))
        .with_pressed_brush(Brush::Solid(Color::opaque(80, 118, 178)))
        .build(ctx),
    )
    .with_content(make_arrow(ctx, arrow, 6.0))
    .build(ctx)
}

impl NumericUpDownBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            value: 0.0,
            step: 0.1,
            min_value: -f32::MAX,
            max_value: f32::MAX,
            precision: 3,
        }
    }

    fn set_value(&mut self, value: f32) {
        self.value = value.max(self.min_value).min(self.max_value);
    }

    pub fn with_min_value(mut self, value: f32) -> Self {
        self.min_value = value;
        self.set_value(self.value);
        self
    }

    pub fn with_max_value(mut self, value: f32) -> Self {
        self.max_value = value;
        self.set_value(self.value);
        self
    }

    pub fn with_value(mut self, value: f32) -> Self {
        self.value = value;
        self.set_value(value);
        self
    }

    pub fn with_step(mut self, step: f32) -> Self {
        self.step = step;
        self
    }

    pub fn with_precision(mut self, precision: usize) -> Self {
        self.precision = precision;
        self
    }

    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let increase;
        let decrease;
        let field;
        let back = BorderBuilder::new(
            WidgetBuilder::new()
                .with_background(BRUSH_DARK)
                .with_foreground(BRUSH_LIGHT),
        )
        .with_stroke_thickness(Thickness::uniform(1.0))
        .build(ctx);

        let grid = GridBuilder::new(
            WidgetBuilder::new()
                .with_child({
                    field = TextBoxBuilder::new(WidgetBuilder::new().on_row(0).on_column(0))
                        .with_vertical_text_alignment(VerticalAlignment::Center)
                        .with_horizontal_text_alignment(HorizontalAlignment::Left)
                        .with_wrap(WrapMode::Letter)
                        .with_text(self.value.to_string())
                        .build(ctx);
                    field
                })
                .with_child(
                    GridBuilder::new(
                        WidgetBuilder::new()
                            .on_column(1)
                            .with_child({
                                increase = make_button(ctx, ArrowDirection::Top, 0);
                                increase
                            })
                            .with_child({
                                decrease = make_button(ctx, ArrowDirection::Bottom, 1);
                                decrease
                            }),
                    )
                    .add_column(Column::auto())
                    .add_row(Row::stretch())
                    .add_row(Row::stretch())
                    .build(ctx),
                ),
        )
        .add_row(Row::stretch())
        .add_column(Column::stretch())
        .add_column(Column::auto())
        .build(ctx);

        ctx.link(grid, back);

        let node = NumericUpDown {
            widget: self.widget_builder.with_child(back).build(),
            increase,
            decrease,
            field,
            value: self.value,
            step: self.step,
            min_value: self.min_value,
            max_value: self.max_value,
            precision: self.precision,
        };

        ctx.add_node(UiNode::new(node))
    }
}
