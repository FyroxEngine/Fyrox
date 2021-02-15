use crate::{
    border::BorderBuilder,
    brush::{Brush, GradientPoint},
    button::ButtonBuilder,
    canvas::CanvasBuilder,
    core::{
        algebra::Vector2,
        color::Color,
        math::{self},
        pool::Handle,
    },
    decorator::DecoratorBuilder,
    grid::{Column, GridBuilder, Row},
    message::{
        ButtonMessage, MessageData, MessageDirection, ScrollBarMessage, TextMessage, UiMessage,
        UiMessageData, WidgetMessage,
    },
    text::TextBuilder,
    utils::{make_arrow, ArrowDirection},
    widget::{Widget, WidgetBuilder},
    BuildContext, Control, HorizontalAlignment, NodeHandleMapping, Orientation, Thickness, UINode,
    UserInterface, VerticalAlignment, BRUSH_LIGHT, BRUSH_LIGHTER, BRUSH_LIGHTEST, COLOR_DARKEST,
    COLOR_LIGHTEST,
};
use std::ops::{Deref, DerefMut};

#[derive(Clone)]
pub struct ScrollBar<M: MessageData, C: Control<M, C>> {
    pub widget: Widget<M, C>,
    pub min: f32,
    pub max: f32,
    pub value: f32,
    pub step: f32,
    pub orientation: Orientation,
    pub is_dragging: bool,
    pub offset: Vector2<f32>,
    pub increase: Handle<UINode<M, C>>,
    pub decrease: Handle<UINode<M, C>>,
    pub indicator: Handle<UINode<M, C>>,
    pub field: Handle<UINode<M, C>>,
    pub value_text: Handle<UINode<M, C>>,
    pub value_precision: usize,
}

crate::define_widget_deref!(ScrollBar<M, C>);

impl<M: MessageData, C: Control<M, C>> Control<M, C> for ScrollBar<M, C> {
    fn resolve(&mut self, node_map: &NodeHandleMapping<M, C>) {
        node_map.resolve(&mut self.increase);
        node_map.resolve(&mut self.decrease);
        node_map.resolve(&mut self.indicator);
        node_map.resolve(&mut self.value_text);
        node_map.resolve(&mut self.field);
    }

    fn arrange_override(&self, ui: &UserInterface<M, C>, final_size: Vector2<f32>) -> Vector2<f32> {
        let size = self.widget.arrange_override(ui, final_size);

        // Adjust indicator position according to current value
        let percent = (self.value - self.min) / (self.max - self.min);

        let field_size = ui.node(self.field).actual_size();

        let indicator = ui.node(self.indicator);
        match self.orientation {
            Orientation::Horizontal => {
                ui.send_message(WidgetMessage::height(
                    self.indicator,
                    MessageDirection::ToWidget,
                    field_size.y,
                ));
                ui.send_message(WidgetMessage::width(
                    self.decrease,
                    MessageDirection::ToWidget,
                    field_size.y,
                ));
                ui.send_message(WidgetMessage::width(
                    self.increase,
                    MessageDirection::ToWidget,
                    field_size.y,
                ));

                let position = Vector2::new(
                    percent * (field_size.x - indicator.actual_size().x).max(0.0),
                    0.0,
                );
                ui.send_message(WidgetMessage::desired_position(
                    self.indicator,
                    MessageDirection::ToWidget,
                    position,
                ));
            }
            Orientation::Vertical => {
                ui.send_message(WidgetMessage::width(
                    self.indicator,
                    MessageDirection::ToWidget,
                    field_size.x,
                ));
                ui.send_message(WidgetMessage::height(
                    self.decrease,
                    MessageDirection::ToWidget,
                    field_size.x,
                ));
                ui.send_message(WidgetMessage::height(
                    self.increase,
                    MessageDirection::ToWidget,
                    field_size.x,
                ));

                let position = Vector2::new(
                    0.0,
                    percent * (field_size.y - indicator.actual_size().y).max(0.0),
                );
                ui.send_message(WidgetMessage::desired_position(
                    self.indicator,
                    MessageDirection::ToWidget,
                    position,
                ));
            }
        }

        size
    }

    fn handle_routed_message(
        &mut self,
        ui: &mut UserInterface<M, C>,
        message: &mut UiMessage<M, C>,
    ) {
        self.widget.handle_routed_message(ui, message);

        match &message.data() {
            UiMessageData::Button(ButtonMessage::Click) => {
                if message.destination() == self.increase {
                    ui.send_message(ScrollBarMessage::value(
                        self.handle(),
                        MessageDirection::ToWidget,
                        self.value + self.step,
                    ));
                } else if message.destination() == self.decrease {
                    ui.send_message(ScrollBarMessage::value(
                        self.handle(),
                        MessageDirection::ToWidget,
                        self.value - self.step,
                    ));
                }
            }
            UiMessageData::ScrollBar(msg)
                if message.destination() == self.handle()
                    && message.direction() == MessageDirection::ToWidget =>
            {
                match *msg {
                    ScrollBarMessage::Value(value) => {
                        let old_value = self.value;
                        let new_value = math::clampf(value, self.min, self.max);
                        if (new_value - old_value).abs() > std::f32::EPSILON {
                            self.value = new_value;
                            self.invalidate_layout();

                            if self.value_text.is_some() {
                                ui.send_message(TextMessage::text(
                                    self.value_text,
                                    MessageDirection::ToWidget,
                                    format!("{:.1$}", value, self.value_precision),
                                ));
                            }

                            let response = ScrollBarMessage::value(
                                self.handle,
                                MessageDirection::FromWidget,
                                self.value,
                            );
                            response.set_handled(message.handled());
                            ui.send_message(response);
                        }
                    }
                    ScrollBarMessage::MinValue(min) => {
                        if self.min != min {
                            self.min = min;
                            if self.min > self.max {
                                std::mem::swap(&mut self.min, &mut self.max);
                            }
                            let old_value = self.value;
                            let new_value = math::clampf(self.value, self.min, self.max);
                            if (new_value - old_value).abs() > std::f32::EPSILON {
                                ui.send_message(ScrollBarMessage::value(
                                    self.handle(),
                                    MessageDirection::ToWidget,
                                    new_value,
                                ));
                            }

                            let response = ScrollBarMessage::min_value(
                                self.handle,
                                MessageDirection::FromWidget,
                                self.min,
                            );
                            response.set_handled(message.handled());
                            ui.send_message(response);
                        }
                    }
                    ScrollBarMessage::MaxValue(max) => {
                        if self.max != max {
                            self.max = max;
                            if self.max < self.min {
                                std::mem::swap(&mut self.min, &mut self.max);
                            }
                            let old_value = self.value;
                            let value = math::clampf(self.value, self.min, self.max);
                            if (value - old_value).abs() > std::f32::EPSILON {
                                ui.send_message(ScrollBarMessage::value(
                                    self.handle(),
                                    MessageDirection::ToWidget,
                                    value,
                                ));
                            }

                            let response = ScrollBarMessage::max_value(
                                self.handle,
                                MessageDirection::FromWidget,
                                self.max,
                            );
                            response.set_handled(message.handled());
                            ui.send_message(response);
                        }
                    }
                }
            }
            UiMessageData::Widget(msg) => {
                if message.destination() == self.indicator {
                    match msg {
                        WidgetMessage::MouseDown { pos, .. } => {
                            if self.indicator.is_some() {
                                let indicator_pos = ui.nodes.borrow(self.indicator).screen_position;
                                self.is_dragging = true;
                                self.offset = indicator_pos - *pos;
                                ui.capture_mouse(self.indicator);
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
                                let canvas = ui.borrow_by_name_up(
                                    self.indicator,
                                    ScrollBar::<M, C>::PART_CANVAS,
                                );
                                let indicator_size = ui.nodes.borrow(self.indicator).actual_size();
                                if self.is_dragging {
                                    let percent = match self.orientation {
                                        Orientation::Horizontal => {
                                            let span = canvas.actual_size().x - indicator_size.x;
                                            let offset = mouse_pos.x - canvas.screen_position.x
                                                + self.offset.x;
                                            if span > 0.0 {
                                                math::clampf(offset / span, 0.0, 1.0)
                                            } else {
                                                0.0
                                            }
                                        }
                                        Orientation::Vertical => {
                                            let span = canvas.actual_size().y - indicator_size.y;
                                            let offset = mouse_pos.y - canvas.screen_position.y
                                                + self.offset.y;
                                            if span > 0.0 {
                                                math::clampf(offset / span, 0.0, 1.0)
                                            } else {
                                                0.0
                                            }
                                        }
                                    };
                                    ui.send_message(ScrollBarMessage::value(
                                        self.handle(),
                                        MessageDirection::ToWidget,
                                        self.min + percent * (self.max - self.min),
                                    ));
                                    message.set_handled(true);
                                }
                            }
                        }
                        _ => (),
                    }
                }
            }
            _ => {}
        }
    }

    fn remove_ref(&mut self, handle: Handle<UINode<M, C>>) {
        if self.indicator == handle {
            self.indicator = Handle::NONE;
        }
        if self.decrease == handle {
            self.decrease = Handle::NONE;
        }
        if self.increase == handle {
            self.increase = Handle::NONE;
        }
        if self.value_text == handle {
            self.value_text = Handle::NONE;
        }
        if self.field == handle {
            self.field = Handle::NONE;
        }
    }
}

impl<M: MessageData, C: Control<M, C>> ScrollBar<M, C> {
    pub const PART_CANVAS: &'static str = "PART_Canvas";

    pub fn new(
        widget: Widget<M, C>,
        increase: Handle<UINode<M, C>>,
        decrease: Handle<UINode<M, C>>,
        indicator: Handle<UINode<M, C>>,
        field: Handle<UINode<M, C>>,
        value_text: Handle<UINode<M, C>>,
    ) -> Self {
        Self {
            widget,
            min: 0.0,
            max: 100.0,
            value: 0.0,
            step: 1.0,
            orientation: Orientation::Vertical,
            is_dragging: false,
            offset: Default::default(),
            increase,
            decrease,
            indicator,
            field,
            value_text,
            value_precision: 3,
        }
    }

    pub fn value(&self) -> f32 {
        self.value
    }

    pub fn max_value(&self) -> f32 {
        self.max
    }

    pub fn min_value(&self) -> f32 {
        self.min
    }

    pub fn set_step(&mut self, step: f32) -> &mut Self {
        self.step = step;
        self
    }

    pub fn step(&self) -> f32 {
        self.step
    }
}

pub struct ScrollBarBuilder<M: MessageData, C: Control<M, C>> {
    widget_builder: WidgetBuilder<M, C>,
    min: Option<f32>,
    max: Option<f32>,
    value: Option<f32>,
    step: Option<f32>,
    orientation: Option<Orientation>,
    increase: Option<Handle<UINode<M, C>>>,
    decrease: Option<Handle<UINode<M, C>>>,
    indicator: Option<Handle<UINode<M, C>>>,
    body: Option<Handle<UINode<M, C>>>,
    show_value: bool,
    value_precision: usize,
}

impl<M: MessageData, C: Control<M, C>> ScrollBarBuilder<M, C> {
    pub fn new(widget_builder: WidgetBuilder<M, C>) -> Self {
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
        }
    }

    pub fn with_min(mut self, min: f32) -> Self {
        self.min = Some(min);
        self
    }

    pub fn with_max(mut self, max: f32) -> Self {
        self.max = Some(max);
        self
    }

    pub fn with_value(mut self, value: f32) -> Self {
        self.value = Some(value);
        self
    }

    pub fn with_orientation(mut self, orientation: Orientation) -> Self {
        self.orientation = Some(orientation);
        self
    }

    pub fn with_step(mut self, step: f32) -> Self {
        self.step = Some(step);
        self
    }

    pub fn with_increase(mut self, increase: Handle<UINode<M, C>>) -> Self {
        self.increase = Some(increase);
        self
    }

    pub fn with_decrease(mut self, decrease: Handle<UINode<M, C>>) -> Self {
        self.decrease = Some(decrease);
        self
    }

    pub fn with_indicator(mut self, indicator: Handle<UINode<M, C>>) -> Self {
        self.indicator = Some(indicator);
        self
    }

    pub fn with_body(mut self, body: Handle<UINode<M, C>>) -> Self {
        self.body = Some(body);
        self
    }

    pub fn show_value(mut self, state: bool) -> Self {
        self.show_value = state;
        self
    }

    pub fn with_value_precision(mut self, precision: usize) -> Self {
        self.value_precision = precision;
        self
    }

    pub fn build(self, ctx: &mut BuildContext<M, C>) -> Handle<UINode<M, C>> {
        let orientation = self.orientation.unwrap_or(Orientation::Horizontal);

        let increase = self.increase.unwrap_or_else(|| {
            ButtonBuilder::new(WidgetBuilder::new())
                .with_content(match orientation {
                    Orientation::Horizontal => make_arrow(ctx, ArrowDirection::Right, 8.0),
                    Orientation::Vertical => make_arrow(ctx, ArrowDirection::Bottom, 8.0),
                })
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
                .build(ctx)
        });

        ctx[decrease].set_row(0).set_column(0);

        match orientation {
            Orientation::Vertical => ctx[decrease].set_height(30.0),
            Orientation::Horizontal => ctx[decrease].set_width(30.0),
        };

        let indicator = self.indicator.unwrap_or_else(|| {
            DecoratorBuilder::new(
                BorderBuilder::new(WidgetBuilder::new().with_foreground(Brush::LinearGradient {
                    from: Vector2::new(0.5, 0.0),
                    to: Vector2::new(0.5, 1.0),
                    stops: vec![
                        GradientPoint {
                            stop: 0.0,
                            color: COLOR_DARKEST,
                        },
                        GradientPoint {
                            stop: 0.25,
                            color: COLOR_LIGHTEST,
                        },
                        GradientPoint {
                            stop: 0.75,
                            color: COLOR_LIGHTEST,
                        },
                        GradientPoint {
                            stop: 1.0,
                            color: COLOR_DARKEST,
                        },
                    ],
                }))
                .with_stroke_thickness(Thickness::uniform(1.0)),
            )
            .with_normal_brush(BRUSH_LIGHT)
            .with_hover_brush(BRUSH_LIGHTER)
            .with_pressed_brush(BRUSH_LIGHTEST)
            .build(ctx)
        });

        match orientation {
            Orientation::Vertical => {
                ctx[indicator]
                    .set_min_size(Vector2::new(0.0, 30.0))
                    .set_width(30.0);
            }
            Orientation::Horizontal => {
                ctx[indicator]
                    .set_min_size(Vector2::new(30.0, 0.0))
                    .set_height(30.0);
            }
        }

        let min = self.min.unwrap_or(0.0);
        let max = self.max.unwrap_or(100.0);
        let value = math::clampf(self.value.unwrap_or(0.0), min, max);

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
        .with_text(format!("{:.1$}", value, self.value_precision))
        .build(ctx);

        ctx.link(value_text, indicator);

        let field = CanvasBuilder::new(
            WidgetBuilder::new()
                .with_name(ScrollBar::<M, C>::PART_CANVAS)
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
                .with_child(field)
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
            BorderBuilder::new(
                WidgetBuilder::new().with_background(Brush::Solid(Color::opaque(60, 60, 60))),
            )
            .with_stroke_thickness(Thickness::uniform(1.0))
            .build(ctx)
        });
        ctx.link(grid, body);

        let node = UINode::ScrollBar(ScrollBar {
            widget: self.widget_builder.with_child(body).build(),
            min,
            max,
            value,
            step: self.step.unwrap_or(1.0),
            orientation,
            is_dragging: false,
            offset: Vector2::default(),
            increase,
            decrease,
            indicator,
            field,
            value_text,
            value_precision: self.value_precision,
        });
        ctx.add_node(node)
    }
}
