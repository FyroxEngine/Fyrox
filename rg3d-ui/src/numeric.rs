use crate::core::num_traits::NumCast;
use crate::{
    border::BorderBuilder,
    brush::Brush,
    button::ButtonBuilder,
    core::{
        color::Color,
        num_traits::NumOps,
        num_traits::{clamp, Bounded, NumAssign},
        pool::Handle,
    },
    decorator::DecoratorBuilder,
    define_constructor,
    formatted_text::WrapMode,
    grid::{Column, GridBuilder, Row},
    message::{ButtonMessage, KeyCode, MessageDirection, TextBoxMessage, UiMessage, WidgetMessage},
    text_box::{TextBox, TextBoxBuilder},
    utils::{make_arrow, ArrowDirection},
    widget::{Widget, WidgetBuilder},
    BuildContext, Control, HorizontalAlignment, NodeHandleMapping, Thickness, UiNode,
    UserInterface, VerticalAlignment, BRUSH_DARK, BRUSH_LIGHT,
};
use std::{
    fmt::{Debug, Display},
    ops::{Deref, DerefMut},
    str::FromStr,
};

pub trait NumericType:
    NumAssign
    + FromStr
    + Clone
    + Copy
    + NumOps
    + PartialOrd
    + Display
    + Bounded
    + Debug
    + Send
    + Sync
    + NumCast
    + Default
    + 'static
{
}

impl<T> NumericType for T where
    T: NumAssign
        + FromStr
        + Clone
        + Copy
        + NumOps
        + PartialOrd
        + Bounded
        + Display
        + Debug
        + Send
        + Sync
        + NumCast
        + Default
        + 'static
{
}

#[derive(Debug, Clone, PartialEq)]
pub enum NumericUpDownMessage<T: NumericType> {
    Value(T),
}

impl<T: NumericType> NumericUpDownMessage<T> {
    define_constructor!(NumericUpDownMessage:Value => fn value(T), layout: false);
}

#[derive(Clone)]
pub struct NumericUpDown<T: NumericType> {
    widget: Widget,
    field: Handle<UiNode>,
    increase: Handle<UiNode>,
    decrease: Handle<UiNode>,
    value: T,
    step: T,
    min_value: T,
    max_value: T,
    precision: usize,
}

impl<T: NumericType> Deref for NumericUpDown<T> {
    type Target = Widget;

    fn deref(&self) -> &Self::Target {
        &self.widget
    }
}

impl<T: NumericType> DerefMut for NumericUpDown<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.widget
    }
}

impl<T: NumericType> NumericUpDown<T> {
    fn clamp_value(&self, value: T) -> T {
        clamp(value, self.min_value, self.max_value)
    }

    fn try_parse_value(&mut self, ui: &mut UserInterface) {
        // Parse input only when focus is lost from text field.
        if let Some(field) = ui.node(self.field).cast::<TextBox>() {
            if let Ok(value) = field.text().parse::<T>() {
                let value = self.clamp_value(value);
                ui.send_message(NumericUpDownMessage::value(
                    self.handle(),
                    MessageDirection::ToWidget,
                    value,
                ));
            }
        }
    }
}

fn saturating_sub<T: NumericType>(a: T, b: T) -> T {
    if b <= a - T::min_value() {
        a - b
    } else {
        T::min_value()
    }
}

fn saturating_add<T: NumericType>(a: T, b: T) -> T {
    if b <= T::max_value() - a {
        a + b
    } else {
        T::max_value()
    }
}

impl<T: NumericType> Control for NumericUpDown<T> {
    fn resolve(&mut self, node_map: &NodeHandleMapping) {
        node_map.resolve(&mut self.field);
        node_map.resolve(&mut self.increase);
        node_map.resolve(&mut self.decrease);
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(msg) = message.data::<WidgetMessage>() {
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
        } else if let Some(NumericUpDownMessage::Value(value)) =
            message.data::<NumericUpDownMessage<T>>()
        {
            if message.direction() == MessageDirection::ToWidget
                && message.destination() == self.handle()
            {
                let clamped = self.clamp_value(*value);
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
        } else if let Some(ButtonMessage::Click) = message.data::<ButtonMessage>() {
            if message.destination() == self.decrease {
                let value = self.clamp_value(saturating_sub(self.value, self.step));
                ui.send_message(NumericUpDownMessage::value(
                    self.handle(),
                    MessageDirection::ToWidget,
                    value,
                ));
            } else if message.destination() == self.increase {
                let value = self.clamp_value(saturating_add(self.value, self.step));

                ui.send_message(NumericUpDownMessage::value(
                    self.handle(),
                    MessageDirection::ToWidget,
                    value,
                ));
            }
        }
    }
}

pub struct NumericUpDownBuilder<T: NumericType> {
    widget_builder: WidgetBuilder,
    value: T,
    step: T,
    min_value: T,
    max_value: T,
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

impl<T: NumericType> NumericUpDownBuilder<T> {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            value: T::zero(),
            step: T::one(),
            min_value: T::min_value(),
            max_value: T::max_value(),
            precision: 3,
        }
    }

    fn set_value(&mut self, value: T) {
        self.value = clamp(value, self.min_value, self.max_value);
    }

    pub fn with_min_value(mut self, value: T) -> Self {
        self.min_value = value;
        self.set_value(self.value);
        self
    }

    pub fn with_max_value(mut self, value: T) -> Self {
        self.max_value = value;
        self.set_value(self.value);
        self
    }

    pub fn with_value(mut self, value: T) -> Self {
        self.value = value;
        self.set_value(value);
        self
    }

    pub fn with_step(mut self, step: T) -> Self {
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
                        .with_text(format!("{:.1$}", self.value, self.precision))
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
