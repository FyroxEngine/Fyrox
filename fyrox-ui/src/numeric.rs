//! A widget that handles numbers of any machine type. See [`NumericUpDown`] docs for more info and usage examples.

#![warn(missing_docs)]

use crate::{
    border::BorderBuilder,
    brush::Brush,
    button::{ButtonBuilder, ButtonMessage},
    core::{
        color::Color,
        num_traits::{clamp, Bounded, NumAssign, NumCast, NumOps},
        pool::Handle,
        reflect::{prelude::*, Reflect},
        type_traits::prelude::*,
        visitor::prelude::*,
    },
    decorator::DecoratorBuilder,
    define_constructor,
    grid::{Column, GridBuilder, Row},
    message::{KeyCode, MessageDirection, MouseButton, UiMessage},
    text::TextMessage,
    text_box::{TextBox, TextBoxBuilder, TextCommitMode},
    utils::{make_arrow, ArrowDirection},
    widget::{Widget, WidgetBuilder, WidgetMessage},
    BuildContext, Control, HorizontalAlignment, Thickness, UiNode, UserInterface,
    VerticalAlignment, BRUSH_DARK, BRUSH_LIGHT,
};
use fyrox_core::variable::InheritableVariable;
use fyrox_graph::BaseSceneGraph;
use std::{
    cmp::Ordering,
    fmt::{Debug, Display},
    ops::{Deref, DerefMut},
    str::FromStr,
};

/// Numeric type is a trait, that has all required traits of a number type. It is used as a useful abstraction over
/// all machine numeric types.
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
    + Visit
    + TypeUuidProvider
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
        + Visit
        + TypeUuidProvider
        + 'static
{
}

/// A set of messages that can be used to modify [`NumericUpDown`] widget state (with [`MessageDirection::ToWidget`], or to
/// fetch changes from it (with [`MessageDirection::FromWidget`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NumericUpDownMessage<T: NumericType> {
    /// Used to set new value of the [`NumericUpDown`] widget (with [`MessageDirection::ToWidget`] direction). Also emitted by the widget
    /// automatically when the new value is set (with [`MessageDirection::FromWidget`]).
    Value(T),
    /// Used to set min value of the [`NumericUpDown`] widget (with [`MessageDirection::ToWidget`] direction). Also emitted by the widget
    /// automatically when the new min value is set (with [`MessageDirection::FromWidget`]).
    MinValue(T),
    /// Used to set max value of the [`NumericUpDown`] widget (with [`MessageDirection::ToWidget`] direction). Also emitted by the widget
    /// automatically when the new max value is set (with [`MessageDirection::FromWidget`]).
    MaxValue(T),
    /// Used to set new step of the [`NumericUpDown`] widget (with [`MessageDirection::ToWidget`] direction). Also emitted by the widget
    /// automatically when the new step is set (with [`MessageDirection::FromWidget`]).
    Step(T),
    /// Used to set new precision of the [`NumericUpDown`] widget (with [`MessageDirection::ToWidget`] direction). Also emitted by the widget
    /// automatically when the new precision is set (with [`MessageDirection::FromWidget`]).
    Precision(usize),
}

impl<T: NumericType> NumericUpDownMessage<T> {
    define_constructor!(
        /// Creates [`NumericUpDownMessage::Value`] message.
        NumericUpDownMessage:Value => fn value(T), layout: false
    );
    define_constructor!(
        /// Creates [`NumericUpDownMessage::MinValue`] message.
        NumericUpDownMessage:MinValue => fn min_value(T), layout: false
    );
    define_constructor!(
        /// Creates [`NumericUpDownMessage::MaxValue`] message.
        NumericUpDownMessage:MaxValue => fn max_value(T), layout: false
    );
    define_constructor!(
        /// Creates [`NumericUpDownMessage::Step`] message.
        NumericUpDownMessage:Step => fn step(T), layout: false
    );

    /// Creates [`NumericUpDownMessage::Precision`] message.
    pub fn precision(
        destination: Handle<UiNode>,
        direction: MessageDirection,
        precision: usize,
    ) -> UiMessage {
        UiMessage {
            handled: Default::default(),
            data: Box::new(precision),
            destination,
            direction,
            perform_layout: Default::default(),
            flags: 0,
        }
    }
}

/// Used to store drag info when dragging the cursor on the up/down buttons.
#[derive(Clone, Debug)]
pub enum DragContext<T: NumericType> {
    /// Dragging is just started.
    PreDrag {
        /// Initial mouse position in Y axis.
        start_mouse_pos: f32,
    },
    /// Dragging is active.
    Dragging {
        /// Start value of the [`NumericUpDown`] widget.
        start_value: T,
        /// Initial mouse position in Y axis.
        start_mouse_pos: f32,
    },
}

/// A widget that handles numbers of any machine type. Use this widget if you need to provide input field for a numeric
/// type.
///
/// ## How to create
///
/// Use [`NumericUpDownBuilder`] to create a new instance of the [`NumericUpDown`] widget:
///
/// ```rust
/// # use fyrox_ui::{
/// #     core::pool::Handle, numeric::NumericUpDownBuilder, widget::WidgetBuilder, BuildContext,
/// #     UiNode,
/// # };
/// fn create_numeric_widget(ctx: &mut BuildContext) -> Handle<UiNode> {
///     NumericUpDownBuilder::new(WidgetBuilder::new())
///         .with_value(123.0f32)
///         .build(ctx)
/// }
/// ```
///
/// Keep in mind, that this widget is generic and can work with any numeric types. Sometimes you might get an "unknown type"
/// error message from the compiler (especially if your use `123.0` ambiguous numeric literals), in this case you need to
/// specify the type explicitly (`NumericUpDownBuilder::<f32>::new...`).
///
/// ## Limits
///
/// This widget supports lower and upper limits for the values. It can be specified by [`NumericUpDownBuilder::with_min_value`]
/// and [`NumericUpDownBuilder::with_max_value`] (or changed at runtime using [`NumericUpDownMessage::MinValue`] and [`NumericUpDownMessage::MaxValue`]
/// messages):
///
/// ```rust
/// use fyrox_ui::{
///     core::pool::Handle, numeric::NumericUpDownBuilder, widget::WidgetBuilder, BuildContext,
///     UiNode,
/// };
/// fn create_numeric_widget(ctx: &mut BuildContext) -> Handle<UiNode> {
///     NumericUpDownBuilder::new(WidgetBuilder::new())
///         .with_value(123.0f32)
///         .with_min_value(42.0)
///         .with_max_value(666.0)
///         .build(ctx)
/// }
/// ```
///
/// The default limits for min and max are [NumericType::min_value] and [NumericType::max_value] respectively.
///
/// [NumericType::min_value]: crate::core::num_traits::Bounded::min_value
/// [NumericType::max_value]: crate::core::num_traits::Bounded::max_value
///
/// ## Step
///
/// Since the value of the widget can be changed via up/down arrow buttons (also by dragging the cursor up or down on them), the widget
/// provides a way to set the step of the value (for increment and decrement at the same time):
///
/// ```rust
/// # use fyrox_ui::{
/// #     core::pool::Handle, numeric::NumericUpDownBuilder, widget::WidgetBuilder, BuildContext,
/// #     UiNode,
/// # };
/// fn create_numeric_widget(ctx: &mut BuildContext) -> Handle<UiNode> {
///     NumericUpDownBuilder::new(WidgetBuilder::new())
///         .with_value(125.0f32)
///         .with_step(5.0)
///         .build(ctx)
/// }
/// ```
///
/// The default value of the step is [NumericType::one].
///
/// [NumericType::one]: crate::core::num_traits::One::one
///
/// ## Precision
///
/// It is possible to specify **visual** rounding of the value up to desired decimal place (it does not change the way how
/// the actual value is rounded). For example, in some cases you might get irrational values such as `1/3 ~= 0.33333333`,
/// but you interested in only first two decimal places. In this case you can set the precision to `2`:
///
/// ```rust
/// # use fyrox_ui::{
/// #     core::pool::Handle, numeric::NumericUpDownBuilder, widget::WidgetBuilder, BuildContext,
/// #     UiNode,
/// # };
/// fn create_numeric_widget(ctx: &mut BuildContext) -> Handle<UiNode> {
///     NumericUpDownBuilder::new(WidgetBuilder::new())
///         .with_value(0.3333333f32)
///         .with_precision(2)
///         .build(ctx)
/// }
/// ```
#[derive(Default, Clone, Visit, Reflect, Debug, ComponentProvider)]
pub struct NumericUpDown<T: NumericType> {
    /// Base widget of the [`NumericUpDown`] widget.
    pub widget: Widget,
    /// A handle of the input field (usually a [`TextBox`] instance).
    pub field: InheritableVariable<Handle<UiNode>>,
    /// A handle of the increase button.
    pub increase: InheritableVariable<Handle<UiNode>>,
    /// A handle of the decrease button.
    pub decrease: InheritableVariable<Handle<UiNode>>,
    /// Current value of the widget.
    pub value: InheritableVariable<T>,
    /// Value of the widget with formatting applied.
    /// This value comes from parsing the result of format! so it has limited precision
    /// and is used to determine if the value has been changed by text editing.
    #[visit(skip)]
    #[reflect(hidden)]
    formatted_value: T,
    /// Step value of the widget.
    pub step: InheritableVariable<T>,
    /// Min value of the widget.
    pub min_value: InheritableVariable<T>,
    /// Max value of the widget.
    pub max_value: InheritableVariable<T>,
    /// Current precision of the widget in decimal places.
    pub precision: InheritableVariable<usize>,
    /// Internal dragging context.
    #[visit(skip)]
    #[reflect(hidden)]
    pub drag_context: Option<DragContext<T>>,
    /// Defines how movement in Y axis will be translated in the actual value change. It is some sort of a scaling modifier.
    pub drag_value_scaling: InheritableVariable<f32>,
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
        clamp(value, *self.min_value, *self.max_value)
    }

    fn sync_text_field(&mut self, ui: &UserInterface) {
        let text = format!("{:.1$}", *self.value, *self.precision);
        self.formatted_value = text.parse::<T>().unwrap_or(*self.value);
        let msg = TextMessage::text(
            *self.field,
            MessageDirection::ToWidget,
            format!("{:.1$}", *self.value, *self.precision),
        );
        msg.set_handled(true);
        ui.send_message(msg);
    }

    fn sync_value_to_bounds_if_needed(&self, ui: &UserInterface) {
        let clamped = self.clamp_value(*self.value);
        if *self.value != clamped {
            ui.send_message(NumericUpDownMessage::value(
                self.handle,
                MessageDirection::ToWidget,
                clamped,
            ));
        }
    }

    fn try_parse_value(&mut self, ui: &UserInterface) {
        // Parse input only when focus is lost from text field.
        if let Some(field) = ui.node(*self.field).cast::<TextBox>() {
            if let Ok(value) = field.text().parse::<T>() {
                // If the value we got from the text box has changed since the last time
                // we parsed it, then the value has been edited through the text box,
                // and the change was meaningful enough to change the result of parsing.
                if value != self.formatted_value {
                    self.formatted_value = value;
                    let value = self.clamp_value(value);
                    ui.send_message(NumericUpDownMessage::value(
                        self.handle(),
                        MessageDirection::ToWidget,
                        value,
                    ));
                }
            } else {
                // Inform the user that parsing failed by re-establishing a valid value.
                self.sync_text_field(ui);
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

fn calculate_value_by_offset<T: NumericType>(
    start_value: T,
    offset: i32,
    step: T,
    min: T,
    max: T,
) -> T {
    let mut new_value = start_value;
    match offset.cmp(&0) {
        Ordering::Less => {
            for _ in 0..(-offset) {
                new_value = saturating_sub(new_value, step);
            }
        }
        Ordering::Equal => {}
        Ordering::Greater => {
            for _ in 0..offset {
                new_value = saturating_add(new_value, step);
            }
        }
    }
    new_value = clamp(new_value, min, max);
    new_value
}

impl<T> TypeUuidProvider for NumericUpDown<T>
where
    T: NumericType,
{
    fn type_uuid() -> Uuid {
        combine_uuids(
            uuid!("f852eda4-18e5-4480-83ae-a607ce1c26f7"),
            T::type_uuid(),
        )
    }
}

impl<T: NumericType> Control for NumericUpDown<T> {
    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(TextMessage::Text(_)) = message.data() {
            if message.destination() == *self.field
                && message.direction == MessageDirection::FromWidget
                && !message.handled()
            {
                self.try_parse_value(ui);
            }
        } else if let Some(msg) = message.data::<WidgetMessage>() {
            match msg {
                WidgetMessage::MouseDown { button, pos, .. } => {
                    // We can activate dragging either by clicking on increase or decrease buttons.
                    if *button == MouseButton::Left
                        && (ui
                            .node(*self.increase)
                            .has_descendant(message.destination(), ui)
                            || ui
                                .node(*self.decrease)
                                .has_descendant(message.destination(), ui))
                    {
                        self.drag_context = Some(DragContext::PreDrag {
                            start_mouse_pos: pos.y,
                        });
                    }
                }
                WidgetMessage::MouseMove { pos, .. } => {
                    if let Some(drag_context) = self.drag_context.as_ref() {
                        match drag_context {
                            DragContext::PreDrag { start_mouse_pos } => {
                                if (pos.y - start_mouse_pos).abs() >= 5.0 {
                                    self.drag_context = Some(DragContext::Dragging {
                                        start_value: *self.value,
                                        start_mouse_pos: *start_mouse_pos,
                                    });
                                }
                            }
                            DragContext::Dragging {
                                start_value,
                                start_mouse_pos,
                            } => {
                                // Just change visual value while dragging; do not touch actual value.
                                ui.send_message(TextMessage::text(
                                    *self.field,
                                    MessageDirection::ToWidget,
                                    format!(
                                        "{:.1$}",
                                        calculate_value_by_offset(
                                            *start_value,
                                            ((*start_mouse_pos - pos.y) * *self.drag_value_scaling)
                                                as i32,
                                            *self.step,
                                            *self.min_value,
                                            *self.max_value
                                        ),
                                        *self.precision
                                    ),
                                ));
                            }
                        }
                    }
                }
                WidgetMessage::KeyDown(key_code) => match *key_code {
                    KeyCode::ArrowUp => {
                        ui.send_message(ButtonMessage::click(
                            *self.increase,
                            MessageDirection::FromWidget,
                        ));
                    }
                    KeyCode::ArrowDown => {
                        ui.send_message(ButtonMessage::click(
                            *self.decrease,
                            MessageDirection::FromWidget,
                        ));
                    }
                    _ => (),
                },
                _ => {}
            }
        } else if let Some(msg) = message.data::<NumericUpDownMessage<T>>() {
            if message.direction() == MessageDirection::ToWidget
                && message.destination() == self.handle()
            {
                match msg {
                    NumericUpDownMessage::Value(value) => {
                        let clamped = self.clamp_value(*value);
                        if *self.value != clamped {
                            self.value.set_value_and_mark_modified(clamped);

                            self.sync_text_field(ui);

                            let mut msg = NumericUpDownMessage::value(
                                self.handle,
                                MessageDirection::FromWidget,
                                *self.value,
                            );
                            // We must maintain flags
                            msg.set_handled(message.handled());
                            msg.flags = message.flags;
                            ui.send_message(msg);
                        }
                    }
                    NumericUpDownMessage::MinValue(min_value) => {
                        if (*self.min_value).ne(min_value) {
                            self.min_value.set_value_and_mark_modified(*min_value);
                            ui.send_message(message.reverse());
                            self.sync_value_to_bounds_if_needed(ui);
                        }
                    }
                    NumericUpDownMessage::MaxValue(max_value) => {
                        if (*self.max_value).ne(max_value) {
                            self.max_value.set_value_and_mark_modified(*max_value);
                            ui.send_message(message.reverse());
                            self.sync_value_to_bounds_if_needed(ui);
                        }
                    }
                    NumericUpDownMessage::Step(step) => {
                        if (*self.step).ne(step) {
                            self.step.set_value_and_mark_modified(*step);
                            ui.send_message(message.reverse());
                            self.sync_text_field(ui);
                        }
                    }
                    NumericUpDownMessage::Precision(precision) => {
                        if (*self.precision).ne(precision) {
                            self.precision.set_value_and_mark_modified(*precision);
                            ui.send_message(message.reverse());
                            self.sync_text_field(ui);
                        }
                    }
                }
            }
        } else if let Some(ButtonMessage::Click) = message.data::<ButtonMessage>() {
            if message.destination() == *self.decrease || message.destination() == *self.increase {
                if let Some(DragContext::Dragging {
                    start_value,
                    start_mouse_pos,
                }) = self.drag_context.take()
                {
                    ui.send_message(NumericUpDownMessage::value(
                        self.handle,
                        MessageDirection::ToWidget,
                        calculate_value_by_offset(
                            start_value,
                            ((start_mouse_pos - ui.cursor_position().y) * *self.drag_value_scaling)
                                as i32,
                            *self.step,
                            *self.min_value,
                            *self.max_value,
                        ),
                    ));
                } else if message.destination() == *self.decrease {
                    let value = self.clamp_value(saturating_sub(*self.value, *self.step));
                    ui.send_message(NumericUpDownMessage::value(
                        self.handle(),
                        MessageDirection::ToWidget,
                        value,
                    ));
                } else if message.destination() == *self.increase {
                    let value = self.clamp_value(saturating_add(*self.value, *self.step));

                    ui.send_message(NumericUpDownMessage::value(
                        self.handle(),
                        MessageDirection::ToWidget,
                        value,
                    ));
                }
            }
        }
    }
}

/// This builder creates new instances of [`NumericUpDown`] widget and adds them to the user interface.
pub struct NumericUpDownBuilder<T: NumericType> {
    widget_builder: WidgetBuilder,
    value: T,
    step: T,
    min_value: T,
    max_value: T,
    precision: usize,
    editable: bool,
    drag_value_scaling: f32,
}

fn make_button(
    ctx: &mut BuildContext,
    arrow: ArrowDirection,
    row: usize,
    editable: bool,
) -> Handle<UiNode> {
    let handle = ButtonBuilder::new(
        WidgetBuilder::new()
            .with_enabled(editable)
            .with_margin(Thickness::right(1.0))
            .on_row(row),
    )
    .with_back(
        DecoratorBuilder::new(
            BorderBuilder::new(
                WidgetBuilder::new().with_foreground(Brush::Solid(Color::opaque(90, 90, 90))),
            )
            .with_corner_radius(2.0)
            .with_pad_by_corner_radius(false),
        )
        .with_normal_brush(Brush::Solid(Color::opaque(60, 60, 60)))
        .with_hover_brush(Brush::Solid(Color::opaque(80, 80, 80)))
        .with_pressed_brush(Brush::Solid(Color::opaque(80, 118, 178)))
        .build(ctx),
    )
    .with_content(make_arrow(ctx, arrow, 6.0))
    .build(ctx);

    // Disable unwanted potential tab navigation for the buttons.
    ctx[handle].accepts_input = false;

    handle
}

impl<T: NumericType> NumericUpDownBuilder<T> {
    /// Creates new builder instance with the base widget builder specified.
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            value: T::zero(),
            step: T::one(),
            min_value: T::min_value(),
            max_value: T::max_value(),
            precision: 3,
            editable: true,
            drag_value_scaling: 0.1,
        }
    }

    fn set_value(&mut self, value: T) {
        self.value = clamp(value, self.min_value, self.max_value);
    }

    /// Sets the desired min value.
    pub fn with_min_value(mut self, value: T) -> Self {
        self.min_value = value;
        self.set_value(self.value);
        self
    }

    /// Sets the desired max value.
    pub fn with_max_value(mut self, value: T) -> Self {
        self.max_value = value;
        self.set_value(self.value);
        self
    }

    /// Sets the desired value.
    pub fn with_value(mut self, value: T) -> Self {
        self.value = value;
        self.set_value(value);
        self
    }

    /// Sets the desired step.
    pub fn with_step(mut self, step: T) -> Self {
        assert!(step >= T::zero());

        self.step = step;
        self
    }

    /// Sets the desired precision.
    pub fn with_precision(mut self, precision: usize) -> Self {
        self.precision = precision;
        self
    }

    /// Enables or disables editing of the widget.
    pub fn with_editable(mut self, editable: bool) -> Self {
        self.editable = editable;
        self
    }

    /// Sets the desired value scaling when dragging. It scales cursor movement value (along Y axis) and multiplies it to get
    /// the new value.
    pub fn with_drag_value_scaling(mut self, drag_value_scaling: f32) -> Self {
        self.drag_value_scaling = drag_value_scaling;
        self
    }

    /// Finishes [`NumericUpDown`] widget creation and adds the new instance to the user interface and returns a handle to it.
    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let increase;
        let decrease;
        let field;
        let back = BorderBuilder::new(
            WidgetBuilder::new()
                .with_background(BRUSH_DARK)
                .with_foreground(BRUSH_LIGHT),
        )
        .with_corner_radius(4.0)
        .with_pad_by_corner_radius(false)
        .with_stroke_thickness(Thickness::uniform(1.0))
        .build(ctx);

        let text = format!("{:.1$}", self.value, self.precision);
        let formatted_value = text.parse::<T>().unwrap_or(self.value);
        let grid = GridBuilder::new(
            WidgetBuilder::new()
                .with_child({
                    field = TextBoxBuilder::new(
                        WidgetBuilder::new()
                            .on_row(0)
                            .on_column(0)
                            .with_margin(Thickness::left(2.0)),
                    )
                    .with_vertical_text_alignment(VerticalAlignment::Center)
                    .with_horizontal_text_alignment(HorizontalAlignment::Left)
                    .with_text_commit_mode(TextCommitMode::Changed)
                    .with_text(text)
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
            increase: increase.into(),
            decrease: decrease.into(),
            field: field.into(),
            value: self.value.into(),
            formatted_value,
            step: self.step.into(),
            min_value: self.min_value.into(),
            max_value: self.max_value.into(),
            precision: self.precision.into(),
            drag_context: None,
            drag_value_scaling: self.drag_value_scaling.into(),
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
