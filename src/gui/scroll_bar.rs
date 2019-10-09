use crate::gui::{
    border::BorderBuilder, canvas::CanvasBuilder,
    event::UIEventKind,
    button::ButtonBuilder, node::{UINodeKind, UINode},
    UserInterface, maxf, Thickness, Layout,
    builder::{GenericNodeBuilder, CommonBuilderFields},
    grid::{GridBuilder, Column, Row}, event::UIEvent, EventSource,
};
use rg3d_core::{
    color::Color, math,
    pool::Handle, math::vec2::Vec2,
};
use std::collections::VecDeque;

/// Scroll bar
///
/// # Events
///
/// [`NumericValueChanged`] - spawned when value changes by any method.
pub struct ScrollBar {
    pub(in crate::gui) owner_handle: Handle<UINode>,
    min: f32,
    max: f32,
    value: f32,
    step: f32,
    orientation: Orientation,
    is_dragging: bool,
    offset: Vec2,
    increase: Handle<UINode>,
    decrease: Handle<UINode>,
    events: VecDeque<UIEvent>,
}

impl ScrollBar {
    pub const PART_CANVAS: &'static str = "PART_Canvas";
    pub const PART_INDICATOR: &'static str = "PART_Indicator";

    pub fn set_value(&mut self, value: f32) {
        let old_value = self.value;
        let new_value = math::clampf(value, self.min, self.max);
        if new_value != old_value {
            self.value = new_value;
            self.events.push_back(
                UIEvent::new(UIEventKind::NumericValueChanged {
                    old_value,
                    new_value,
                }));
        }
    }

    pub fn set_max_value(&mut self, max: f32) {
        self.max = max;
        if self.max < self.min {
            std::mem::swap(&mut self.min, &mut self.max);
        }
        let old_value = self.value;
        let clamped_new_value = math::clampf(self.value, self.min, self.max);
        if clamped_new_value != old_value {
            self.set_value(clamped_new_value);
        }
    }

    pub fn set_min_value(&mut self, min: f32) {
        self.min = min;
        if self.min > self.max {
            std::mem::swap(&mut self.min, &mut self.max);
        }
        let old_value = self.value;
        let clamped_new_value = math::clampf(self.value, self.min, self.max);
        if clamped_new_value != old_value {
            self.set_value(clamped_new_value);
        }
    }
}

impl Layout for ScrollBar {
    fn measure_override(&self, ui: &UserInterface, available_size: Vec2) -> Vec2 {
        ui.default_measure_override(self.owner_handle, available_size)
    }

    fn arrange_override(&self, ui: &UserInterface, final_size: Vec2) -> Vec2 {
        let size = ui.default_arrange_override(self.owner_handle, final_size);

        // Adjust indicator position according to current value
        let percent = (self.value - self.min) / (self.max - self.min);

        let field_size = match ui.borrow_by_name_down(self.owner_handle, Self::PART_CANVAS) {
            Some(canvas) => canvas.actual_size.get(),
            None => return size
        };

        if let Some(node) = ui.borrow_by_name_down(self.owner_handle, Self::PART_INDICATOR) {
            match self.orientation {
                Orientation::Horizontal => {
                    node.set_desired_local_position(Vec2::make(
                        percent * maxf(0.0, field_size.x - node.actual_size.get().x),
                        0.0)
                    );
                    node.height.set(field_size.y);
                }
                Orientation::Vertical => {
                    node.set_desired_local_position(Vec2::make(
                        0.0,
                        percent * maxf(0.0, field_size.y - node.actual_size.get().y))
                    );
                    node.width.set(field_size.x);
                }
            }
        }

        size
    }
}

pub struct ScrollBarBuilder {
    min: Option<f32>,
    max: Option<f32>,
    value: Option<f32>,
    step: Option<f32>,
    orientation: Option<Orientation>,
    common: CommonBuilderFields,
}

#[derive(Copy, Clone, Debug)]
pub enum Orientation {
    Vertical,
    Horizontal,
}

impl Default for ScrollBarBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ScrollBarBuilder {
    pub fn new() -> Self {
        Self {
            min: None,
            max: None,
            value: None,
            step: None,
            orientation: None,
            common: CommonBuilderFields::new(),
        }
    }

    impl_default_builder_methods!();

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

        let increase = ButtonBuilder::new()
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
            })
            .with_text(match orientation {
                Orientation::Horizontal => ">",
                Orientation::Vertical => "v"
            })
            .build(ui);


        let decrease = ButtonBuilder::new()
            .on_column(0)
            .on_row(0)
            .with_width(match orientation {
                Orientation::Horizontal => 30.0,
                Orientation::Vertical => std::f32::NAN
            })
            .with_height(match orientation {
                Orientation::Horizontal => std::f32::NAN,
                Orientation::Vertical => 30.0
            })
            .with_text(match orientation {
                Orientation::Horizontal => "<",
                Orientation::Vertical => "^"
            })
            .build(ui);

        let scroll_bar = ScrollBar {
            owner_handle: Handle::NONE,
            min: self.min.unwrap_or(0.0),
            max: self.max.unwrap_or(100.0),
            value: self.value.unwrap_or(0.0),
            step: self.step.unwrap_or(1.0),
            orientation,
            is_dragging: false,
            offset: Vec2::zero(),
            increase,
            decrease,
            events: VecDeque::new(),
        };

        let indicator = BorderBuilder::new()
            .with_name(ScrollBar::PART_INDICATOR)
            .with_stroke_color(Color::opaque(50, 50, 50))
            .with_stroke_thickness(match orientation {
                Orientation::Horizontal => Thickness { left: 1.0, top: 0.0, right: 1.0, bottom: 0.0 },
                Orientation::Vertical => Thickness { left: 0.0, top: 1.0, right: 0.0, bottom: 1.0 }
            })
            .with_color(Color::opaque(255, 255, 255))
            .with_width(30.0)
            .with_height(30.0)
            .with_event_handler(Box::new(move |ui, handle, evt| {
                if evt.source == handle {
                    match evt.kind {
                        UIEventKind::MouseDown { pos, .. } => {
                            let indicator_pos = if let Some(node) = ui.nodes.borrow(handle) {
                                node.screen_position
                            } else {
                                return;
                            };

                            if let Some(scroll_bar_node) = ui.borrow_by_criteria_up_mut(handle, |node| match node.kind {
                                UINodeKind::ScrollBar(..) => true,
                                _ => false
                            }) {
                                if let UINodeKind::ScrollBar(scroll_bar) = scroll_bar_node.get_kind_mut() {
                                    scroll_bar.is_dragging = true;
                                    scroll_bar.offset = indicator_pos - pos;
                                }
                            }

                            ui.capture_mouse(handle);
                            evt.handled = true;
                        }
                        UIEventKind::MouseUp { .. } => {
                            if let Some(scroll_bar_node) = ui.borrow_by_criteria_up_mut(handle, |node| match node.kind {
                                UINodeKind::ScrollBar(..) => true,
                                _ => false
                            }) {
                                if let UINodeKind::ScrollBar(scroll_bar) = scroll_bar_node.get_kind_mut() {
                                    scroll_bar.is_dragging = false;
                                }
                            }
                            ui.release_mouse_capture();
                            evt.handled = true;
                        }
                        UIEventKind::MouseMove { pos, .. } => {
                            let (field_pos, field_size) =
                                match ui.borrow_by_name_up(handle, ScrollBar::PART_CANVAS) {
                                    Some(canvas) => (canvas.screen_position, canvas.actual_size.get()),
                                    None => return
                                };

                            let bar_size = match ui.nodes.borrow(handle) {
                                Some(node) => node.actual_size.get(),
                                None => return
                            };

                            let scroll_bar_handle = ui.find_by_criteria_up(handle, |node| match node.kind {
                                UINodeKind::ScrollBar(..) => true,
                                _ => false
                            });

                            if let Some(scroll_bar_node) = ui.nodes.borrow_mut(scroll_bar_handle) {
                                if let UINodeKind::ScrollBar(scroll_bar) = scroll_bar_node.get_kind_mut() {
                                    let orientation = scroll_bar.orientation;

                                    if scroll_bar.is_dragging {
                                        let percent = match orientation {
                                            Orientation::Horizontal => {
                                                let span = field_size.x - bar_size.x;
                                                let offset = pos.x - field_pos.x + scroll_bar.offset.x;
                                                if span > 0.0 {
                                                    math::clampf(offset / span, 0.0, 1.0)
                                                } else {
                                                    0.0
                                                }
                                            }
                                            Orientation::Vertical => {
                                                let span = field_size.y - bar_size.y;
                                                let offset = pos.y - field_pos.y + scroll_bar.offset.y;
                                                if span > 0.0 {
                                                    math::clampf(offset / span, 0.0, 1.0)
                                                } else {
                                                    0.0
                                                }
                                            }
                                        };

                                        scroll_bar.set_value(percent * (scroll_bar.max - scroll_bar.min));

                                        evt.handled = true;
                                    }
                                }
                            }
                        }
                        _ => ()
                    }
                }
            }))
            .build(ui);

        GenericNodeBuilder::new(UINodeKind::ScrollBar(scroll_bar), self.common)
            .with_child(BorderBuilder::new()
                .with_color(Color::opaque(120, 120, 120))
                .with_stroke_thickness(Thickness::uniform(1.0))
                .with_stroke_color(Color::opaque(200, 200, 200))
                .with_child(GridBuilder::new()
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
                    .with_child(decrease)
                    .with_child(CanvasBuilder::new()
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
                        .build(ui)
                    )
                    .with_child(increase)
                    .build(ui)
                )
                .build(ui)
            )
            .with_event_handler(Box::new(move |ui, handle, event| {
                match event.kind {
                    UIEventKind::Click => {
                        if let Some(node) = ui.nodes.borrow_mut(handle) {
                            if let UINodeKind::ScrollBar(scroll_bar) = node.get_kind_mut() {
                                if event.source == scroll_bar.increase {
                                    scroll_bar.set_value(scroll_bar.value + scroll_bar.step);
                                } else if event.source == scroll_bar.decrease {
                                    scroll_bar.set_value(scroll_bar.value - scroll_bar.step);
                                }
                            }
                        }
                    }
                    _ => ()
                }
            }))
            .build(ui)
    }
}

impl EventSource for ScrollBar {
    fn emit_event(&mut self) -> Option<UIEvent> {
        self.events.pop_front()
    }
}