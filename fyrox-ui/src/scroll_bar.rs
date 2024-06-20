//! Scroll bar is used to represent a value on a finite range. It has a thumb that shows the current value on
//! on the bar. See [`ScrollBar`] docs for more info and usage examples.

#![warn(missing_docs)]

use crate::font::FontResource;
use crate::{
    border::BorderBuilder,
    brush::Brush,
    button::{ButtonBuilder, ButtonMessage},
    canvas::CanvasBuilder,
    core::{
        algebra::Vector2, color::Color, pool::Handle, reflect::prelude::*, type_traits::prelude::*,
        visitor::prelude::*,
    },
    decorator::DecoratorBuilder,
    define_constructor,
    grid::{Column, GridBuilder, Row},
    message::{MessageDirection, UiMessage},
    text::{TextBuilder, TextMessage},
    utils::{make_arrow, ArrowDirection},
    widget::{Widget, WidgetBuilder, WidgetMessage},
    BuildContext, Control, HorizontalAlignment, Orientation, Thickness, UiNode, UserInterface,
    VerticalAlignment, BRUSH_DARK, BRUSH_LIGHT, BRUSH_LIGHTER, BRUSH_LIGHTEST,
};
use fyrox_core::uuid_provider;
use fyrox_core::variable::InheritableVariable;
use fyrox_graph::BaseSceneGraph;
use std::ops::{Deref, DerefMut};

/// A set of messages that can be accepted by [`ScrollBar`] widget.
#[derive(Debug, Clone, PartialEq)]
pub enum ScrollBarMessage {
    /// Used to indicate that the value of the scroll bar has changed ([`MessageDirection::FromWidget`]) or to set a
    /// new value (with [`MessageDirection::ToWidget`].
    Value(f32),
    /// Used to indicate that the min value of the scroll bar has changed ([`MessageDirection::FromWidget`]) or to set a
    /// new min value (with [`MessageDirection::ToWidget`].
    MinValue(f32),
    /// Used to indicate that the max value of the scroll bar has changed ([`MessageDirection::FromWidget`]) or to set a
    /// new max value (with [`MessageDirection::ToWidget`].
    MaxValue(f32),
    /// Used to set the size of the indicator(thumb) adaptively, according to the relative sizes of the container and the
    /// content in it (with [`MessageDirection::ToWidget`].
    SizeRatio(f32),
}

impl ScrollBarMessage {
    define_constructor!(
        /// Creates [`ScrollBarMessage::Value`] message.
        ScrollBarMessage:Value => fn value(f32), layout: false
    );
    define_constructor!(
        /// Creates [`ScrollBarMessage::MaxValue`] message.
        ScrollBarMessage:MaxValue => fn max_value(f32), layout: false
    );
    define_constructor!(
        /// Creates [`ScrollBarMessage::MinValue`] message.
        ScrollBarMessage:MinValue => fn min_value(f32), layout: false
    );
    define_constructor!(
        /// Creates [`ScrollBarMessage::SizeRatio`] message.
        ScrollBarMessage:SizeRatio => fn size_ratio(f32), layout: false
    );
}

/// Scroll bar is used to represent a value on a finite range. It has a thumb that shows the current value on
/// on the bar. Usually it is used in pair with [`crate::scroll_panel::ScrollPanel`] to create something like
/// [`crate::scroll_viewer::ScrollViewer`] widget. However, it could also be used to create sliders to show some
/// value that lies within some range.
///
/// ## Example
///
/// A simple example of how to create a new [`ScrollBar`] could be something like this:
///
/// ```rust
/// # use fyrox_ui::{
/// #     core::pool::Handle, scroll_bar::ScrollBarBuilder, widget::WidgetBuilder, BuildContext,
/// #     UiNode,
/// # };
/// fn create_scroll_bar(ctx: &mut BuildContext) -> Handle<UiNode> {
///     ScrollBarBuilder::new(WidgetBuilder::new())
///         .with_min(0.0)
///         .with_max(200.0)
///         .with_value(123.0)
///         .build(ctx)
/// }
/// ```
///
/// It creates a horizontal scroll bar with `123.0` value and a range of `[0.0..200.0]`. To fetch the new value
/// of the scroll bar, use [`ScrollBarMessage::Value`] message:
///
/// ```rust
/// # use fyrox_ui::{
/// #     core::pool::Handle,
/// #     message::{MessageDirection, UiMessage},
/// #     scroll_bar::ScrollBarMessage,
/// #     UiNode,
/// # };
/// # fn foo(scroll_bar: Handle<UiNode>, message: &mut UiMessage) {
/// if message.destination() == scroll_bar
///     && message.direction() == MessageDirection::FromWidget
/// {
///     if let Some(ScrollBarMessage::Value(value)) = message.data() {
///         println!("{}", value);
///     }
/// }
/// # }
/// ```
///
/// Please note, that you need to explicitly filter messages by [`MessageDirection::FromWidget`], because it's the only
/// direction that is used as an "indicator" that the value was accepted by the scroll bar.
///
/// ## Orientation
///
/// Scroll bar could be either horizontal (default) or vertical. You can select the orientation when building
/// a scroll bar using [`ScrollBarBuilder::with_orientation`] method and provide a desired value from [`Orientation`]
/// enum there.
///
/// ## Show values
///
/// By default, scroll bar does not show its actual value, you can turn it on using [`ScrollBarBuilder::show_value`]
/// method with `true` as the first argument. To change rounding of the value, use [`ScrollBarBuilder::with_value_precision`]
/// and provide the desired amount of decimal places there.
///
/// ## Step
///
/// Scroll bar provides arrows to change the current value using a fixed step value. You can change it using
/// [`ScrollBarBuilder::with_step`] method.
#[derive(Default, Clone, Debug, Visit, Reflect, ComponentProvider)]
pub struct ScrollBar {
    /// Base widget of the scroll bar.
    pub widget: Widget,
    /// Min value of the scroll bar.
    pub min: InheritableVariable<f32>,
    /// Max value of the scroll bar.
    pub max: InheritableVariable<f32>,
    /// Current value of the scroll bar.
    pub value: InheritableVariable<f32>,
    /// Step of the scroll bar.
    pub step: InheritableVariable<f32>,
    /// Current orientation of the scroll bar.
    pub orientation: InheritableVariable<Orientation>,
    /// Internal flag, that could be used to check whether the scroll bar's thumb is being dragged or not.
    pub is_dragging: bool,
    /// Internal mouse offset that is used for dragging purposes.
    pub offset: Vector2<f32>,
    /// A handle of the increase button.
    pub increase: InheritableVariable<Handle<UiNode>>,
    /// A handle of the decrease button.
    pub decrease: InheritableVariable<Handle<UiNode>>,
    /// A handle of the indicator (thumb).
    pub indicator: InheritableVariable<Handle<UiNode>>,
    /// A handle of the canvas that is used for the thumb.
    pub indicator_canvas: InheritableVariable<Handle<UiNode>>,
    /// A handle of the [`crate::text::Text`] widget that is used to show the current value of the scroll bar.
    pub value_text: InheritableVariable<Handle<UiNode>>,
    /// Current value precison in decimal places.
    pub value_precision: InheritableVariable<usize>,
}

crate::define_widget_deref!(ScrollBar);

uuid_provider!(ScrollBar = "92accc96-b334-424d-97ea-332c4787acf6");

impl Control for ScrollBar {
    fn arrange_override(&self, ui: &UserInterface, final_size: Vector2<f32>) -> Vector2<f32> {
        let size = self.widget.arrange_override(ui, final_size);

        // Adjust indicator position according to current value
        let percent = (*self.value - *self.min) / (*self.max - *self.min);

        let field_size = ui.node(*self.indicator_canvas).actual_local_size();

        let indicator = ui.node(*self.indicator);
        match *self.orientation {
            Orientation::Horizontal => {
                ui.send_message(WidgetMessage::height(
                    *self.indicator,
                    MessageDirection::ToWidget,
                    field_size.y,
                ));
                ui.send_message(WidgetMessage::width(
                    *self.decrease,
                    MessageDirection::ToWidget,
                    field_size.y,
                ));
                ui.send_message(WidgetMessage::width(
                    *self.increase,
                    MessageDirection::ToWidget,
                    field_size.y,
                ));

                let position = Vector2::new(
                    percent * (field_size.x - indicator.actual_local_size().x).max(0.0),
                    0.0,
                );
                ui.send_message(WidgetMessage::desired_position(
                    *self.indicator,
                    MessageDirection::ToWidget,
                    position,
                ));
            }
            Orientation::Vertical => {
                ui.send_message(WidgetMessage::width(
                    *self.indicator,
                    MessageDirection::ToWidget,
                    field_size.x,
                ));
                ui.send_message(WidgetMessage::height(
                    *self.decrease,
                    MessageDirection::ToWidget,
                    field_size.x,
                ));
                ui.send_message(WidgetMessage::height(
                    *self.increase,
                    MessageDirection::ToWidget,
                    field_size.x,
                ));

                let position = Vector2::new(
                    0.0,
                    percent * (field_size.y - indicator.actual_local_size().y).max(0.0),
                );
                ui.send_message(WidgetMessage::desired_position(
                    *self.indicator,
                    MessageDirection::ToWidget,
                    position,
                ));
            }
        }

        size
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if let Some(ButtonMessage::Click) = message.data::<ButtonMessage>() {
            if message.destination() == *self.increase {
                ui.send_message(ScrollBarMessage::value(
                    self.handle(),
                    MessageDirection::ToWidget,
                    *self.value + *self.step,
                ));
            } else if message.destination() == *self.decrease {
                ui.send_message(ScrollBarMessage::value(
                    self.handle(),
                    MessageDirection::ToWidget,
                    *self.value - *self.step,
                ));
            }
        } else if let Some(msg) = message.data::<ScrollBarMessage>() {
            if message.destination() == self.handle()
                && message.direction() == MessageDirection::ToWidget
            {
                match *msg {
                    ScrollBarMessage::Value(value) => {
                        let old_value = *self.value;
                        let new_value = value.clamp(*self.min, *self.max);
                        if (new_value - old_value).abs() > f32::EPSILON {
                            self.value.set_value_and_mark_modified(new_value);
                            self.invalidate_arrange();

                            if self.value_text.is_some() {
                                ui.send_message(TextMessage::text(
                                    *self.value_text,
                                    MessageDirection::ToWidget,
                                    format!("{:.1$}", value, *self.value_precision),
                                ));
                            }

                            let mut response = ScrollBarMessage::value(
                                self.handle,
                                MessageDirection::FromWidget,
                                *self.value,
                            );
                            response.flags = message.flags;
                            response.set_handled(message.handled());
                            ui.send_message(response);
                        }
                    }
                    ScrollBarMessage::MinValue(min) => {
                        if *self.min != min {
                            self.min.set_value_and_mark_modified(min);
                            if *self.min > *self.max {
                                std::mem::swap(&mut self.min, &mut self.max);
                            }
                            let old_value = *self.value;
                            let new_value = self.value.clamp(*self.min, *self.max);
                            if (new_value - old_value).abs() > f32::EPSILON {
                                ui.send_message(ScrollBarMessage::value(
                                    self.handle(),
                                    MessageDirection::ToWidget,
                                    new_value,
                                ));
                            }

                            let response = ScrollBarMessage::min_value(
                                self.handle,
                                MessageDirection::FromWidget,
                                *self.min,
                            );
                            response.set_handled(message.handled());
                            ui.send_message(response);
                        }
                    }
                    ScrollBarMessage::MaxValue(max) => {
                        if *self.max != max {
                            self.max.set_value_and_mark_modified(max);
                            if *self.max < *self.min {
                                std::mem::swap(&mut self.min, &mut self.max);
                            }
                            let old_value = *self.value;
                            let value = self.value.clamp(*self.min, *self.max);
                            if (value - old_value).abs() > f32::EPSILON {
                                ui.send_message(ScrollBarMessage::value(
                                    self.handle(),
                                    MessageDirection::ToWidget,
                                    value,
                                ));
                            }

                            let response = ScrollBarMessage::max_value(
                                self.handle,
                                MessageDirection::FromWidget,
                                *self.max,
                            );
                            response.set_handled(message.handled());
                            ui.send_message(response);
                        }
                    }
                    ScrollBarMessage::SizeRatio(size_ratio) => {
                        let field_size = ui.node(*self.indicator_canvas).actual_global_size();
                        let indicator_size = ui.node(*self.indicator).actual_global_size();

                        match *self.orientation {
                            Orientation::Horizontal => {
                                // minimum size of the indicator will be 15 irrespective of size ratio
                                let new_size = (size_ratio * field_size.x).max(15.0);
                                let old_size = indicator_size.x;

                                if new_size != old_size {
                                    ui.send_message(WidgetMessage::width(
                                        *self.indicator,
                                        MessageDirection::ToWidget,
                                        new_size,
                                    ));
                                }
                            }
                            Orientation::Vertical => {
                                // minimum size of the indicator will be 15 irrespective of size ratio
                                let new_size = (size_ratio * field_size.y).max(15.0);
                                let old_size = indicator_size.y;

                                if new_size != old_size {
                                    ui.send_message(WidgetMessage::height(
                                        *self.indicator,
                                        MessageDirection::ToWidget,
                                        new_size,
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        } else if let Some(msg) = message.data::<WidgetMessage>() {
            if message.destination() == *self.indicator {
                match msg {
                    WidgetMessage::MouseDown { pos, .. } => {
                        if self.indicator.is_some() {
                            let indicator_pos = ui.nodes.borrow(*self.indicator).screen_position();
                            self.is_dragging = true;
                            self.offset = indicator_pos - *pos;
                            ui.capture_mouse(*self.indicator);
                            message.set_handled(true);
                        }
                    }
                    WidgetMessage::MouseUp { .. } => {
                        self.is_dragging = false;
                        ui.release_mouse_capture();
                        message.set_handled(true);
                    }
                    WidgetMessage::MouseMove { pos: mouse_pos, .. } => {
                        if self.indicator.is_some() {
                            let indicator_canvas = ui.node(*self.indicator_canvas);
                            let indicator_size =
                                ui.nodes.borrow(*self.indicator).actual_global_size();
                            if self.is_dragging {
                                let percent = match *self.orientation {
                                    Orientation::Horizontal => {
                                        let span = indicator_canvas.actual_global_size().x
                                            - indicator_size.x;
                                        let offset = mouse_pos.x
                                            - indicator_canvas.screen_position().x
                                            + self.offset.x;
                                        if span > 0.0 {
                                            (offset / span).clamp(0.0, 1.0)
                                        } else {
                                            0.0
                                        }
                                    }
                                    Orientation::Vertical => {
                                        let span = indicator_canvas.actual_global_size().y
                                            - indicator_size.y;
                                        let offset = mouse_pos.y
                                            - indicator_canvas.screen_position().y
                                            + self.offset.y;
                                        if span > 0.0 {
                                            (offset / span).clamp(0.0, 1.0)
                                        } else {
                                            0.0
                                        }
                                    }
                                };
                                ui.send_message(ScrollBarMessage::value(
                                    self.handle(),
                                    MessageDirection::ToWidget,
                                    *self.min + percent * (*self.max - *self.min),
                                ));
                                message.set_handled(true);
                            }
                        }
                    }
                    _ => (),
                }
            }
        }
    }
}

/// Scroll bar widget is used to create [`ScrollBar`] widget instances and add them to the user interface.
pub struct ScrollBarBuilder {
    widget_builder: WidgetBuilder,
    min: Option<f32>,
    max: Option<f32>,
    value: Option<f32>,
    step: Option<f32>,
    orientation: Option<Orientation>,
    increase: Option<Handle<UiNode>>,
    decrease: Option<Handle<UiNode>>,
    indicator: Option<Handle<UiNode>>,
    body: Option<Handle<UiNode>>,
    show_value: bool,
    value_precision: usize,
    font: Option<FontResource>,
    font_size: f32,
}

impl ScrollBarBuilder {
    /// Creates new scroll bar builder instance.
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            min: None,
            max: None,
            value: None,
            step: None,
            orientation: None,
            increase: None,
            decrease: None,
            indicator: None,
            body: None,
            show_value: false,
            value_precision: 3,
            font: None,
            font_size: 14.0,
        }
    }

    /// Sets the desired min value.
    pub fn with_min(mut self, min: f32) -> Self {
        self.min = Some(min);
        self
    }

    /// Sets the desired max value.
    pub fn with_max(mut self, max: f32) -> Self {
        self.max = Some(max);
        self
    }

    /// Sets the desired value.
    pub fn with_value(mut self, value: f32) -> Self {
        self.value = Some(value);
        self
    }

    /// Sets the desired orientation.
    pub fn with_orientation(mut self, orientation: Orientation) -> Self {
        self.orientation = Some(orientation);
        self
    }

    /// Sets the desired step.
    pub fn with_step(mut self, step: f32) -> Self {
        self.step = Some(step);
        self
    }

    /// Sets the new handle to a button, that is used to increase values of the scroll bar.
    pub fn with_increase(mut self, increase: Handle<UiNode>) -> Self {
        self.increase = Some(increase);
        self
    }

    /// Sets the new handle to a button, that is used to decrease values of the scroll bar.
    pub fn with_decrease(mut self, decrease: Handle<UiNode>) -> Self {
        self.decrease = Some(decrease);
        self
    }

    /// Sets the new handle to a widget, that is used as a thumb of the scroll bar.
    pub fn with_indicator(mut self, indicator: Handle<UiNode>) -> Self {
        self.indicator = Some(indicator);
        self
    }

    /// Sets the new handle to a widget, that is used as a background of the scroll bar.
    pub fn with_body(mut self, body: Handle<UiNode>) -> Self {
        self.body = Some(body);
        self
    }

    /// Show or hide the value of the scroll bar.
    pub fn show_value(mut self, state: bool) -> Self {
        self.show_value = state;
        self
    }

    /// Sets the desired value precision of the scroll bar.
    pub fn with_value_precision(mut self, precision: usize) -> Self {
        self.value_precision = precision;
        self
    }

    /// Sets the desired font.
    pub fn with_font(mut self, font: FontResource) -> Self {
        self.font = Some(font);
        self
    }

    /// Sets the desired font size.
    pub fn with_font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }

    /// Creates new scroll bar instance and adds it to the user interface.
    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        let orientation = self.orientation.unwrap_or(Orientation::Horizontal);

        let increase = self.increase.unwrap_or_else(|| {
            ButtonBuilder::new(WidgetBuilder::new())
                .with_content(match orientation {
                    Orientation::Horizontal => make_arrow(ctx, ArrowDirection::Right, 8.0),
                    Orientation::Vertical => make_arrow(ctx, ArrowDirection::Bottom, 8.0),
                })
                .with_repeat_clicks_on_hold(true)
                .build(ctx)
        });

        match orientation {
            Orientation::Vertical => {
                ctx[increase].set_height(30.0).set_row(2).set_column(0);
            }
            Orientation::Horizontal => {
                ctx[increase].set_width(30.0).set_row(0).set_column(2);
            }
        }

        let decrease = self.decrease.unwrap_or_else(|| {
            ButtonBuilder::new(WidgetBuilder::new())
                .with_content(match orientation {
                    Orientation::Horizontal => make_arrow(ctx, ArrowDirection::Left, 8.0),
                    Orientation::Vertical => make_arrow(ctx, ArrowDirection::Top, 8.0),
                })
                .with_repeat_clicks_on_hold(true)
                .build(ctx)
        });

        ctx[decrease].set_row(0).set_column(0);

        match orientation {
            Orientation::Vertical => ctx[decrease].set_height(30.0),
            Orientation::Horizontal => ctx[decrease].set_width(30.0),
        };

        let indicator = self.indicator.unwrap_or_else(|| {
            DecoratorBuilder::new(
                BorderBuilder::new(
                    WidgetBuilder::new().with_foreground(Brush::Solid(Color::TRANSPARENT)),
                )
                .with_corner_radius(8.0)
                .with_pad_by_corner_radius(false)
                .with_stroke_thickness(Thickness::uniform(1.0)),
            )
            .with_normal_brush(BRUSH_LIGHT)
            .with_hover_brush(BRUSH_LIGHTER)
            .with_pressed_brush(BRUSH_LIGHTEST)
            .build(ctx)
        });

        match orientation {
            Orientation::Vertical => {
                ctx[indicator].set_min_size(Vector2::new(0.0, 15.0));
            }
            Orientation::Horizontal => {
                ctx[indicator].set_min_size(Vector2::new(15.0, 0.0));
            }
        }

        let min = self.min.unwrap_or(0.0);
        let max = self.max.unwrap_or(100.0);
        let value = self.value.unwrap_or(0.0).clamp(min, max);

        let value_text = if self.show_value {
            let value_text = TextBuilder::new(
                WidgetBuilder::new()
                    .with_visibility(self.show_value)
                    .with_horizontal_alignment(HorizontalAlignment::Center)
                    .with_vertical_alignment(VerticalAlignment::Center)
                    .with_hit_test_visibility(false)
                    .with_margin(Thickness::uniform(3.0))
                    .on_column(match orientation {
                        Orientation::Horizontal => 1,
                        Orientation::Vertical => 0,
                    })
                    .on_row(match orientation {
                        Orientation::Horizontal => 0,
                        Orientation::Vertical => 1,
                    }),
            )
            .with_font(self.font.unwrap_or_else(|| ctx.default_font()))
            .with_font_size(self.font_size)
            .with_text(format!("{:.1$}", value, self.value_precision))
            .build(ctx);

            ctx.link(value_text, indicator);

            value_text
        } else {
            Handle::NONE
        };

        let indicator_canvas = CanvasBuilder::new(
            WidgetBuilder::new()
                .on_column(match orientation {
                    Orientation::Horizontal => 1,
                    Orientation::Vertical => 0,
                })
                .on_row(match orientation {
                    Orientation::Horizontal => 0,
                    Orientation::Vertical => 1,
                })
                .with_child(indicator),
        )
        .build(ctx);

        let grid = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(decrease)
                .with_child(indicator_canvas)
                .with_child(increase),
        )
        .add_rows(match orientation {
            Orientation::Horizontal => vec![Row::stretch()],
            Orientation::Vertical => vec![Row::auto(), Row::stretch(), Row::auto()],
        })
        .add_columns(match orientation {
            Orientation::Horizontal => vec![Column::auto(), Column::stretch(), Column::auto()],
            Orientation::Vertical => vec![Column::stretch()],
        })
        .build(ctx);

        let body = self.body.unwrap_or_else(|| {
            BorderBuilder::new(WidgetBuilder::new().with_background(BRUSH_DARK))
                .with_stroke_thickness(Thickness::uniform(1.0))
                .build(ctx)
        });
        ctx.link(grid, body);

        let node = UiNode::new(ScrollBar {
            widget: self.widget_builder.with_child(body).build(),
            min: min.into(),
            max: max.into(),
            value: value.into(),
            step: self.step.unwrap_or(1.0).into(),
            orientation: orientation.into(),
            is_dragging: false,
            offset: Vector2::default(),
            increase: increase.into(),
            decrease: decrease.into(),
            indicator: indicator.into(),
            indicator_canvas: indicator_canvas.into(),
            value_text: value_text.into(),
            value_precision: self.value_precision.into(),
        });
        ctx.add_node(node)
    }
}
