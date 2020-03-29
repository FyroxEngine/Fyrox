use crate::{
    border::BorderBuilder,
    canvas::CanvasBuilder,
    button::ButtonBuilder,
    UserInterface,
    Thickness,
    grid::{
        GridBuilder,
        Column,
        Row,
    },
    widget::{
        Widget,
        WidgetBuilder,
    },
    Control,
    UINode,
    core::{
        color::Color,
        math::{
            self,
            vec2::Vec2,
        },
        pool::Handle,
    },
    HorizontalAlignment,
    VerticalAlignment,
    text::TextBuilder,
    brush::Brush,
    decorator::DecoratorBuilder,
    message::{
        ButtonMessage,
        UiMessageData,
        UiMessage,
        ScrollBarMessage,
        WidgetMessage,
    },
    NodeHandleMapping,
};

pub struct ScrollBar<M: 'static, C: 'static + Control<M, C>> {
    widget: Widget<M, C>,
    min: f32,
    max: f32,
    value: f32,
    step: f32,
    orientation: Orientation,
    is_dragging: bool,
    offset: Vec2,
    increase: Handle<UINode<M, C>>,
    decrease: Handle<UINode<M, C>>,
    indicator: Handle<UINode<M, C>>,
    field: Handle<UINode<M, C>>,
    value_text: Handle<UINode<M, C>>,
    value_precision: usize,
}

impl<M, C: 'static + Control<M, C>> Control<M, C> for ScrollBar<M, C> {
    fn widget(&self) -> &Widget<M, C> {
        &self.widget
    }

    fn widget_mut(&mut self) -> &mut Widget<M, C> {
        &mut self.widget
    }

    fn raw_copy(&self) -> UINode<M, C> {
        UINode::ScrollBar(Self {
            widget: self.widget.raw_copy(),
            min: self.min,
            max: self.max,
            value: self.value,
            step: self.step,
            orientation: self.orientation,
            is_dragging: self.is_dragging,
            offset: self.offset,
            increase: self.increase,
            decrease: self.decrease,
            indicator: self.indicator,
            field: self.field,
            value_text: self.value_text,
            value_precision: self.value_precision,
        })
    }

    fn resolve(&mut self, node_map: &NodeHandleMapping<M, C>) {
        self.increase = *node_map.get(&self.increase).unwrap();
        self.decrease = *node_map.get(&self.decrease).unwrap();
        self.indicator = *node_map.get(&self.indicator).unwrap();
        self.value_text = *node_map.get(&self.value_text).unwrap();
        self.field = *node_map.get(&self.field).unwrap();
    }

    fn arrange_override(&self, ui: &UserInterface<M, C>, final_size: Vec2) -> Vec2 {
        let size = self.widget.arrange_override(ui, final_size);

        // Adjust indicator position according to current value
        let percent = (self.value - self.min) / (self.max - 2.0 * self.min);

        let field_size = ui.node(self.field).widget().actual_size();

        let indicator = ui.node(self.indicator).widget();
        match self.orientation {
            Orientation::Horizontal => {
                indicator.set_desired_local_position(Vec2::new(
                    percent * (field_size.x - indicator.actual_size().x).max(0.0),
                    0.0));
                indicator.set_height(field_size.y);
            }
            Orientation::Vertical => {
                indicator.set_desired_local_position(Vec2::new(
                    0.0,
                    percent * (field_size.y - indicator.actual_size().y).max(0.0))
                );
                indicator.set_width(field_size.x);
            }
        }

        size
    }

    fn handle_message(&mut self, self_handle: Handle<UINode<M, C>>, ui: &mut UserInterface<M, C>, message: &mut UiMessage<M, C>) {
        self.widget.handle_message(self_handle, ui, message);

        match &message.data {
            UiMessageData::Button(msg) => {
                if let ButtonMessage::Click = msg {
                    if message.source == self.increase {
                        self.set_value(self.value + self.step);
                    } else if message.source == self.decrease {
                        self.set_value(self.value - self.step);
                    }
                }
            }
            UiMessageData::ScrollBar(ref prop) => {
                if message.source == self_handle || message.target == self_handle {
                    if let ScrollBarMessage::Value(value) = prop {
                        if self.value_text.is_some() {
                            if let UINode::Text(text) = ui.node_mut(self.value_text) {
                                text.set_text(format!("{:.1$}", value, self.value_precision));
                            }
                        }
                    }
                }
            }
            UiMessageData::Widget(msg) => {
                if message.source == self.indicator {
                    match msg {
                        WidgetMessage::MouseDown { pos, .. } => {
                            if self.indicator.is_some() {
                                let indicator_pos = ui.nodes
                                    .borrow(self.indicator)
                                    .widget()
                                    .screen_position;
                                self.is_dragging = true;
                                self.offset = indicator_pos - *pos;
                                ui.capture_mouse(self.indicator);
                                message.handled = true;
                            }
                        }
                        WidgetMessage::MouseUp { .. } => {
                            self.is_dragging = false;
                            ui.release_mouse_capture();
                            message.handled = true;
                        }
                        WidgetMessage::MouseMove(mouse_pos) => {
                            if self.indicator.is_some() {
                                let canvas = ui.borrow_by_name_up(self.indicator, ScrollBar::<M, C>::PART_CANVAS).widget();
                                let indicator_size = ui.nodes
                                    .borrow(self.indicator)
                                    .widget()
                                    .actual_size();
                                if self.is_dragging {
                                    let percent = match self.orientation {
                                        Orientation::Horizontal => {
                                            let span = canvas.actual_size().x - indicator_size.x;
                                            let offset = mouse_pos.x - canvas.screen_position.x + self.offset.x;
                                            if span > 0.0 {
                                                math::clampf(offset / span, 0.0, 1.0)
                                            } else {
                                                0.0
                                            }
                                        }
                                        Orientation::Vertical => {
                                            let span = canvas.actual_size().y - indicator_size.y;
                                            let offset = mouse_pos.y - canvas.screen_position.y + self.offset.y;
                                            if span > 0.0 {
                                                math::clampf(offset / span, 0.0, 1.0)
                                            } else {
                                                0.0
                                            }
                                        }
                                    };
                                    self.set_value(percent * (self.max - self.min));
                                    message.handled = true;
                                }
                            }
                        }
                        _ => ()
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

impl<M, C: 'static + Control<M, C>> ScrollBar<M, C> {
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

    pub fn set_value(&mut self, value: f32) -> &mut Self {
        let old_value = self.value;
        let new_value = math::clampf(value, self.min, self.max);
        if (new_value - old_value).abs() > std::f32::EPSILON {
            self.value = new_value;
            self.widget.post_message(UiMessage::new(UiMessageData::ScrollBar(ScrollBarMessage::Value(new_value))));
            self.widget.invalidate_layout();
        }
        self
    }

    pub fn value(&self) -> f32 {
        self.value
    }

    pub fn set_max_value(&mut self, max: f32) -> &mut Self {
        self.max = max;
        if self.max < self.min {
            std::mem::swap(&mut self.min, &mut self.max);
        }
        let old_value = self.value;
        let clamped_new_value = math::clampf(self.value, self.min, self.max);
        if (clamped_new_value - old_value).abs() > std::f32::EPSILON {
            self.set_value(clamped_new_value);
            self.widget.invalidate_layout();
        }
        self
    }

    pub fn max_value(&self) -> f32 {
        self.max
    }

    pub fn set_min_value(&mut self, min: f32) -> &mut Self {
        self.min = min;
        if self.min > self.max {
            std::mem::swap(&mut self.min, &mut self.max);
        }
        let old_value = self.value;
        let clamped_new_value = math::clampf(self.value, self.min, self.max);
        if (clamped_new_value - old_value).abs() > std::f32::EPSILON {
            self.set_value(clamped_new_value);
            self.widget.invalidate_layout();
        }
        self
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

    pub fn scroll(&mut self, amount: f32) {
        self.set_value(self.value + amount);
    }
}

pub struct ScrollBarBuilder<M: 'static, C: 'static + Control<M, C>> {
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

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Orientation {
    Vertical,
    Horizontal,
}

impl<M, C: 'static + Control<M, C>> ScrollBarBuilder<M, C> {
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

    pub fn build(self, ui: &mut UserInterface<M, C>) -> Handle<UINode<M, C>> {
        let orientation = self.orientation.unwrap_or(Orientation::Horizontal);

        let increase = self.increase.unwrap_or_else(|| {
            ButtonBuilder::new(WidgetBuilder::new())
                .with_text(match orientation {
                    Orientation::Horizontal => ">",
                    Orientation::Vertical => "v"
                })
                .build(ui)
        });

        ui.node_mut(increase)
            .widget_mut()
            .set_width_mut(match orientation {
                Orientation::Horizontal => 30.0,
                Orientation::Vertical => std::f32::NAN
            })
            .set_height_mut(match orientation {
                Orientation::Horizontal => std::f32::NAN,
                Orientation::Vertical => 30.0
            })
            .set_column(match orientation {
                Orientation::Horizontal => 2,
                Orientation::Vertical => 0
            })
            .set_row(match orientation {
                Orientation::Horizontal => 0,
                Orientation::Vertical => 2
            });

        let decrease = self.decrease.unwrap_or_else(|| {
            ButtonBuilder::new(WidgetBuilder::new())
                .with_text(match orientation {
                    Orientation::Horizontal => "<",
                    Orientation::Vertical => "^"
                })
                .build(ui)
        });

        ui.node_mut(decrease)
            .widget_mut()
            .set_column(0)
            .set_row(0)
            .set_width_mut(match orientation {
                Orientation::Horizontal => 30.0,
                Orientation::Vertical => std::f32::NAN
            })
            .set_height_mut(match orientation {
                Orientation::Horizontal => std::f32::NAN,
                Orientation::Vertical => 30.0
            });

        let indicator = self.indicator.unwrap_or_else(|| {
            DecoratorBuilder::new(BorderBuilder::new(WidgetBuilder::new()))
                .build(ui)
        });

        ui.node_mut(indicator)
            .widget_mut()
            .set_min_size(match orientation {
                Orientation::Vertical => Vec2::new(0.0, 30.0),
                Orientation::Horizontal => Vec2::new(30.0, 0.0),
            })
            .set_width_mut(match orientation {
                Orientation::Vertical => 30.0,
                Orientation::Horizontal => std::f32::NAN,
            })
            .set_height_mut(match orientation {
                Orientation::Vertical => std::f32::NAN,
                Orientation::Horizontal => 30.0,
            });

        let field = CanvasBuilder::new(WidgetBuilder::new()
            .with_name(ScrollBar::<M, C>::PART_CANVAS)
            .on_column(match orientation {
                Orientation::Horizontal => 1,
                Orientation::Vertical => 0
            })
            .on_row(match orientation {
                Orientation::Horizontal => 0,
                Orientation::Vertical => 1
            })
            .with_child(indicator)
        ).build(ui);

        let min = self.min.unwrap_or(0.0);
        let max = self.max.unwrap_or(100.0);
        let value = math::clampf(self.value.unwrap_or(0.0), min, max);

        let value_text = TextBuilder::new(WidgetBuilder::new()
            .with_visibility(self.show_value)
            .with_horizontal_alignment(HorizontalAlignment::Center)
            .with_vertical_alignment(VerticalAlignment::Center)
            .with_hit_test_visibility(false)
            .with_margin(Thickness::uniform(3.0))
            .on_column(match orientation {
                Orientation::Horizontal => 1,
                Orientation::Vertical => 0
            })
            .on_row(match orientation {
                Orientation::Horizontal => 0,
                Orientation::Vertical => 1
            }))
            .with_text(format!("{:.1$}", value, self.value_precision))
            .build(ui);

        ui.link_nodes(value_text, indicator);

        let grid = GridBuilder::new(WidgetBuilder::new()
            .with_child(decrease)
            .with_child(field)
            .with_child(increase))
            .add_rows(match orientation {
                Orientation::Horizontal => vec![Row::stretch()],
                Orientation::Vertical => vec![Row::auto(),
                                              Row::stretch(),
                                              Row::auto()]
            })
            .add_columns(match orientation {
                Orientation::Horizontal => vec![Column::auto(),
                                                Column::stretch(),
                                                Column::auto()],
                Orientation::Vertical => vec![Column::stretch()]
            })
            .build(ui);

        let body = self.body.unwrap_or_else(|| {
            BorderBuilder::new(WidgetBuilder::new()
                .with_background(Brush::Solid(Color::opaque(120, 120, 120))))
                .with_stroke_thickness(Thickness::uniform(1.0))
                .build(ui)
        });

        ui.link_nodes(grid, body);

        let scroll_bar = ScrollBar {
            widget: self.widget_builder
                .with_child(body)
                .build(),
            min,
            max,
            value,
            step: self.step.unwrap_or(1.0),
            orientation,
            is_dragging: false,
            offset: Vec2::ZERO,
            increase,
            decrease,
            indicator,
            field,
            value_text,
            value_precision: self.value_precision,
        };

        let handle = ui.add_node(UINode::ScrollBar(scroll_bar));

        ui.flush_messages();

        handle
    }
}