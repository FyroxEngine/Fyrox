use crate::{
    border::BorderBuilder,
    brush::Brush,
    button::{ButtonBuilder, ButtonMessage},
    core::{
        color::Color,
        num_traits::{clamp, Bounded, NumAssign, NumCast, NumOps},
        pool::Handle,
        reflect::Reflect,
    },
    decorator::DecoratorBuilder,
    define_constructor,
    grid::{Column, GridBuilder, Row},
    message::{KeyCode, MessageDirection, UiMessage},
    text::TextMessage,
    text_box::{TextBox, TextBoxBuilder},
    utils::{make_arrow, ArrowDirection},
    widget::{Widget, WidgetBuilder, WidgetMessage},
    BuildContext, Control, HorizontalAlignment, NodeHandleMapping, Thickness, UiNode,
    UserInterface, VerticalAlignment, BRUSH_DARK, BRUSH_LIGHT,
};
use std::{
    any::{Any, TypeId},
    fmt::{Debug, Display},
    ops::{Deref, DerefMut},
    rc::Rc,
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
    + Reflect
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
        + Reflect
        + 'static
{
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NumericUpDownMessage<T: NumericType> {
    Value(T),
    MinValue(T),
    MaxValue(T),
    Step(T),
    Precision(usize),
}

impl<T: NumericType> NumericUpDownMessage<T> {
    define_constructor!(NumericUpDownMessage:Value => fn value(T), layout: false);
    define_constructor!(NumericUpDownMessage:MinValue => fn min_value(T), layout: false);
    define_constructor!(NumericUpDownMessage:MaxValue => fn max_value(T), layout: false);
    define_constructor!(NumericUpDownMessage:Step => fn step(T), layout: false);

    pub fn precision(
        destination: Handle<UiNode>,
        direction: MessageDirection,
        precision: usize,
    ) -> UiMessage {
        UiMessage {
            handled: Default::default(),
            data: Rc::new(precision),
            destination,
            direction,
            perform_layout: Default::default(),
            flags: 0,
        }
    }
}

#[derive(Clone)]
pub struct NumericUpDown<T: NumericType> {
    pub widget: Widget,
    pub field: Handle<UiNode>,
    pub increase: Handle<UiNode>,
    pub decrease: Handle<UiNode>,
    pub value: T,
    pub step: T,
    pub min_value: T,
    pub max_value: T,
    pub precision: usize,
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

    fn sync_text_field(&self, ui: &UserInterface) {
        ui.send_message(TextMessage::text(
            self.field,
            MessageDirection::ToWidget,
            format!("{:.1$}", self.value, self.precision),
        ));
    }

    fn sync_value_to_bounds_if_needed(&self, ui: &UserInterface) {
        let clamped = self.clamp_value(self.value);
        if self.value != clamped {
            ui.send_message(NumericUpDownMessage::value(
                self.handle,
                MessageDirection::ToWidget,
                clamped,
            ));
        }
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

fn saturating_sub<T>(a: T, b: T) -> T
where
    T: NumericType,
{
    assert!(b >= T::zero());

    if a >= b + T::min_value() {
        a - b
    } else {
        T::min_value()
    }
}

fn saturating_add<T>(a: T, b: T) -> T
where
    T: NumericType,
{
    assert!(b >= T::zero());

    if a < T::max_value() - b {
        a + b
    } else {
        T::max_value()
    }
}

impl<T: NumericType> Control for NumericUpDown<T> {
    fn query_component(&self, type_id: TypeId) -> Option<&dyn Any> {
        if type_id == TypeId::of::<Self>() {
            Some(self)
        } else {
            None
        }
    }

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
                    WidgetMessage::Unfocus => {
                        self.try_parse_value(ui);
                    }
                    WidgetMessage::KeyDown(KeyCode::Return) => {
                        self.try_parse_value(ui);

                        message.set_handled(true);
                    }
                    _ => {}
                }
            }
        } else if let Some(msg) = message.data::<NumericUpDownMessage<T>>() {
            if message.direction() == MessageDirection::ToWidget
                && message.destination() == self.handle()
            {
                match msg {
                    NumericUpDownMessage::Value(value) => {
                        let clamped = self.clamp_value(*value);
                        if self.value != clamped {
                            self.value = clamped;

                            self.sync_text_field(ui);

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
                    NumericUpDownMessage::MinValue(min_value) => {
                        if self.min_value.ne(min_value) {
                            self.min_value = *min_value;
                            ui.send_message(message.reverse());
                            self.sync_value_to_bounds_if_needed(ui);
                        }
                    }
                    NumericUpDownMessage::MaxValue(max_value) => {
                        if self.max_value.ne(max_value) {
                            self.max_value = *max_value;
                            ui.send_message(message.reverse());
                            self.sync_value_to_bounds_if_needed(ui);
                        }
                    }
                    NumericUpDownMessage::Step(step) => {
                        if self.step.ne(step) {
                            self.step = *step;
                            ui.send_message(message.reverse());
                            self.sync_text_field(ui);
                        }
                    }
                    NumericUpDownMessage::Precision(precision) => {
                        if self.precision.ne(precision) {
                            self.precision = *precision;
                            ui.send_message(message.reverse());
                            self.sync_text_field(ui);
                        }
                    }
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
    editable: bool,
}

pub fn make_button(
    ctx: &mut BuildContext,
    arrow: ArrowDirection,
    row: usize,
    editable: bool,
) -> Handle<UiNode> {
    ButtonBuilder::new(
        WidgetBuilder::new()
            .with_enabled(editable)
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
            editable: true,
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
        assert!(step >= T::zero());

        self.step = step;
        self
    }

    pub fn with_precision(mut self, precision: usize) -> Self {
        self.precision = precision;
        self
    }

    pub fn with_editable(mut self, editable: bool) -> Self {
        self.editable = editable;
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
                        .with_text(format!("{:.1$}", self.value, self.precision))
                        .with_editable(self.editable)
                        .build(ctx);
                    field
                })
                .with_child(
                    GridBuilder::new(
                        WidgetBuilder::new()
                            .on_column(1)
                            .with_child({
                                increase = make_button(ctx, ArrowDirection::Top, 0, self.editable);
                                increase
                            })
                            .with_child({
                                decrease =
                                    make_button(ctx, ArrowDirection::Bottom, 1, self.editable);
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

#[cfg(test)]
mod test {
    use crate::numeric::{saturating_add, saturating_sub};

    #[test]
    fn test_saturating_add() {
        // i32
        assert_eq!(saturating_add(0, 1), 1);
        assert_eq!(saturating_add(1, 0), 1);
        assert_eq!(saturating_add(0, 0), 0);
        assert_eq!(saturating_add(1, 1), 2);
        assert_eq!(saturating_add(i32::MAX, 1), i32::MAX);
        assert_eq!(saturating_add(i32::MIN, 1), i32::MIN + 1);

        // f32
        assert_eq!(saturating_add(0.0, 1.0), 1.0);
        assert_eq!(saturating_add(1.0, 0.0), 1.0);
        assert_eq!(saturating_add(f32::MAX, 1.0), f32::MAX);
        assert_eq!(saturating_add(f32::MIN, 1.0), f32::MIN + 1.0);
    }

    #[test]
    fn test_saturating_sub() {
        // i32
        assert_eq!(saturating_sub(0, 0), 0);
        assert_eq!(saturating_sub(0, 1), -1);
        assert_eq!(saturating_sub(1, 1), 0);
        assert_eq!(saturating_sub(1, 0), 1);
        assert_eq!(saturating_sub(10, 10), 0);
        assert_eq!(saturating_sub(i32::MIN, 1), i32::MIN);
        assert_eq!(saturating_sub(i32::MAX, 1), i32::MAX - 1);

        // u32
        assert_eq!(saturating_sub(0u32, 0u32), 0u32);
        assert_eq!(saturating_sub(0u32, 1u32), 0u32);
        assert_eq!(saturating_sub(1u32, 1u32), 0u32);
        assert_eq!(saturating_sub(1u32, 0u32), 1u32);
        assert_eq!(saturating_sub(10u32, 10u32), 0u32);
        assert_eq!(saturating_sub(u32::MIN, 1u32), u32::MIN);
        assert_eq!(saturating_sub(u32::MAX, 1u32), u32::MAX - 1);

        // f32
        assert_eq!(saturating_sub(0.0, 1.0), -1.0);
        assert_eq!(saturating_sub(1.0, 0.0), 1.0);
        assert_eq!(saturating_sub(1.0, 1.0), 0.0);
        assert_eq!(saturating_sub(f32::MIN, 1.0), f32::MIN);
        assert_eq!(saturating_sub(f32::MAX, 1.0), f32::MAX - 1.0);
    }
}
