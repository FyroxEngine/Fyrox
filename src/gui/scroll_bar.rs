use crate::gui::{
    border::BorderBuilder, canvas::CanvasBuilder, event::UIEventKind,
    button::ButtonBuilder, UserInterface, maxf, Thickness,
    grid::{GridBuilder, Column, Row}, event::UIEvent,
    widget::{Widget, WidgetBuilder},
    Control, UINode
};
use crate::core::{
    color::Color, math,
    pool::Handle, math::vec2::Vec2,
};

/// Scroll bar
///
/// # Events
///
/// [`NumericValueChanged`] - spawned when value changes by any method.
pub struct ScrollBar {
    widget: Widget,
    min: f32,
    max: f32,
    value: f32,
    step: f32,
    orientation: Orientation,
    is_dragging: bool,
    offset: Vec2,
    increase: Handle<UINode>,
    decrease: Handle<UINode>,
    indicator: Handle<UINode>,
    field: Handle<UINode>,
}

impl Control for ScrollBar {
    fn widget(&self) -> &Widget {
        &self.widget
    }

    fn widget_mut(&mut self) -> &mut Widget {
        &mut self.widget
    }

    fn handle_event(&mut self, _self_handle: Handle<UINode>, ui: &mut UserInterface, evt: &mut UIEvent) {
        if let UIEventKind::Click = evt.kind {
            if evt.source == self.increase {
                self.set_value(self.value + self.step);
            } else if evt.source == self.decrease {
                self.set_value(self.value - self.step);
            }
        }

        if evt.source == self.indicator {
            match evt.kind {
                UIEventKind::MouseDown { pos, .. } => {
                    let indicator_pos = ui.nodes
                        .borrow(self.indicator)
                        .widget()
                        .screen_position;
                    self.is_dragging = true;
                    self.offset = indicator_pos - pos;
                    ui.capture_mouse(self.indicator);
                    evt.handled = true;
                }
                UIEventKind::MouseUp { .. } => {
                    self.is_dragging = false;
                    ui.release_mouse_capture();
                    evt.handled = true;
                }
                UIEventKind::MouseMove { pos, .. } => {
                    let (field_pos, field_size) = {
                        let canvas = ui.borrow_by_name_up(self.indicator, ScrollBar::PART_CANVAS).widget();
                        (canvas.screen_position, canvas.actual_size.get())
                    };

                    let bar_size = ui.nodes.borrow(self.indicator).widget().actual_size.get();
                    let orientation = self.orientation;
                    if self.is_dragging {
                        let percent = match orientation {
                            Orientation::Horizontal => {
                                let span = field_size.x - bar_size.x;
                                let offset = pos.x - field_pos.x + self.offset.x;
                                if span > 0.0 {
                                    math::clampf(offset / span, 0.0, 1.0)
                                } else {
                                    0.0
                                }
                            }
                            Orientation::Vertical => {
                                let span = field_size.y - bar_size.y;
                                let offset = pos.y - field_pos.y + self.offset.y;
                                if span > 0.0 {
                                    math::clampf(offset / span, 0.0, 1.0)
                                } else {
                                    0.0
                                }
                            }
                        };
                        self.set_value(percent * (self.max - self.min));
                        evt.handled = true;
                    }
                }
                _ => ()
            }
        }
    }

    fn arrange_override(&self, ui: &UserInterface, final_size: Vec2) -> Vec2 {
        let size = self.widget.arrange_override(ui, final_size);

        // Adjust indicator position according to current value
        let percent = (self.value - self.min) / (self.max - self.min);

        let field_size = ui.get_node(self.field).widget().actual_size.get();

        let indicator = ui.get_node(self.indicator).widget();
        match self.orientation {
            Orientation::Horizontal => {
                indicator.set_desired_local_position(Vec2::new(
                    percent * maxf(0.0, field_size.x - indicator.actual_size.get().x),
                    0.0)
                );
                indicator.height.set(field_size.y);
            }
            Orientation::Vertical => {
                indicator.set_desired_local_position(Vec2::new(
                    0.0,
                    percent * maxf(0.0, field_size.y - indicator.actual_size.get().y))
                );
                indicator.width.set(field_size.x);
            }
        }

        size
    }
}

impl ScrollBar {
    pub const PART_CANVAS: &'static str = "PART_Canvas";

    pub fn set_value(&mut self, value: f32) {
        let old_value = self.value;
        let new_value = math::clampf(value, self.min, self.max);
        if (new_value - old_value).abs() > std::f32::EPSILON {
            self.value = new_value;
            self.widget.events.borrow_mut().push_back(
                UIEvent::new(UIEventKind::NumericValueChanged {
                    old_value,
                    new_value,
                }));
        }
    }

    pub fn get_value(&self) -> f32 {
        self.value
    }

    pub fn set_max_value(&mut self, max: f32) {
        self.max = max;
        if self.max < self.min {
            std::mem::swap(&mut self.min, &mut self.max);
        }
        let old_value = self.value;
        let clamped_new_value = math::clampf(self.value, self.min, self.max);
        if (clamped_new_value - old_value).abs() > std::f32::EPSILON {
            self.set_value(clamped_new_value);
        }
    }

    pub fn get_max_value(&self) -> f32 {
        self.max
    }

    pub fn set_min_value(&mut self, min: f32) {
        self.min = min;
        if self.min > self.max {
            std::mem::swap(&mut self.min, &mut self.max);
        }
        let old_value = self.value;
        let clamped_new_value = math::clampf(self.value, self.min, self.max);
        if (clamped_new_value - old_value).abs() > std::f32::EPSILON {
            self.set_value(clamped_new_value);
        }
    }

    pub fn get_min_value(&self) -> f32 {
        self.min
    }

    pub fn scroll(&mut self, amount: f32) {
        let old_value = self.value;

        self.value += amount;

        self.widget.events.borrow_mut().push_back(
            UIEvent::new(UIEventKind::NumericValueChanged {
                old_value,
                new_value: self.value,
            }));
    }
}

pub struct ScrollBarBuilder {
    widget_builder: WidgetBuilder,
    min: Option<f32>,
    max: Option<f32>,
    value: Option<f32>,
    step: Option<f32>,
    orientation: Option<Orientation>,
}

#[derive(Copy, Clone, Debug)]
pub enum Orientation {
    Vertical,
    Horizontal,
}

impl ScrollBarBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            min: None,
            max: None,
            value: None,
            step: None,
            orientation: None,
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

    pub fn build(self, ui: &mut UserInterface) -> Handle<UINode> {
        let orientation = self.orientation.unwrap_or(Orientation::Horizontal);

        let increase = ButtonBuilder::new(WidgetBuilder::new()
            .with_width(match orientation {
                Orientation::Horizontal => 30.0,
                Orientation::Vertical => std::f32::NAN
            })
            .with_height(match orientation {
                Orientation::Horizontal => std::f32::NAN,
                Orientation::Vertical => 30.0
            })
            .on_column(match orientation {
                Orientation::Horizontal => 2,
                Orientation::Vertical => 0
            })
            .on_row(match orientation {
                Orientation::Horizontal => 0,
                Orientation::Vertical => 2
            }))
            .with_text(match orientation {
                Orientation::Horizontal => ">",
                Orientation::Vertical => "v"
            })
            .build(ui);

        let decrease = ButtonBuilder::new(WidgetBuilder::new()
            .on_column(0)
            .on_row(0)
            .with_width(match orientation {
                Orientation::Horizontal => 30.0,
                Orientation::Vertical => std::f32::NAN
            })
            .with_height(match orientation {
                Orientation::Horizontal => std::f32::NAN,
                Orientation::Vertical => 30.0
            }))
            .with_text(match orientation {
                Orientation::Horizontal => "<",
                Orientation::Vertical => "^"
            })
            .build(ui);

        let indicator = BorderBuilder::new(WidgetBuilder::new()
            .with_background(Color::opaque(255, 255, 255))
            .with_foreground(Color::opaque(50, 50, 50))
            .with_width(30.0)
            .with_height(30.0))
            .with_stroke_thickness(match orientation {
                Orientation::Horizontal => Thickness { left: 1.0, top: 0.0, right: 1.0, bottom: 0.0 },
                Orientation::Vertical => Thickness { left: 0.0, top: 1.0, right: 0.0, bottom: 1.0 }
            })
            .build(ui);

        let field = CanvasBuilder::new(WidgetBuilder::new()
            .with_name(ScrollBar::PART_CANVAS)
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

        let scroll_bar = ScrollBar {
            widget: self.widget_builder
                .with_child(BorderBuilder::new(WidgetBuilder::new()
                    .with_background(Color::opaque(120, 120, 120))
                    .with_foreground(Color::opaque(200, 200, 200))
                    .with_child(GridBuilder::new(WidgetBuilder::new()
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
                        .build(ui)
                    ))
                    .with_stroke_thickness(Thickness::uniform(1.0))
                    .build(ui)
                )
                .build(),
            min: self.min.unwrap_or(0.0),
            max: self.max.unwrap_or(100.0),
            value: self.value.unwrap_or(0.0),
            step: self.step.unwrap_or(1.0),
            orientation,
            is_dragging: false,
            offset: Vec2::ZERO,
            increase,
            decrease,
            indicator,
            field,
        };
        ui.add_node(scroll_bar)
    }
}
