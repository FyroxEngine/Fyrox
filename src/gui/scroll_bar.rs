use crate::{
    gui::{
        border::BorderBuilder,
        canvas::CanvasBuilder,
        VerticalAlignment,
        HorizontalAlignment,
        draw::Color,
        event::{RoutedEventHandlerType, RoutedEventHandler},
        button::ButtonBuilder,
        node::{UINodeKind, UINode},
        UserInterface,
        maxf,
        Thickness,
        Layout,
        builder::{GenericNodeBuilder, CommonBuilderFields},
        grid::{GridBuilder, Column, Row}
    },
    math,
    utils::pool::Handle,
    math::vec2::Vec2,
};
use crate::gui::event::RoutedEventKind;

pub struct ValueChangedArgs {
    pub source: Handle<UINode>,
    pub old_value: f32,
    pub new_value: f32,
}

pub type ValueChanged = dyn FnMut(&mut UserInterface, ValueChangedArgs);

pub struct ScrollBar {
    pub(in crate::gui) owner_handle: Handle<UINode>,
    min: f32,
    max: f32,
    value: f32,
    step: f32,
    orientation: Orientation,
    is_dragging: bool,
    offset: Vec2,
    value_changed: Option<Box<ValueChanged>>,
}

impl ScrollBar {
    pub const PART_CANVAS: &'static str = "PART_Canvas";
    pub const PART_INDICATOR: &'static str = "PART_Indicator";

    fn new() -> Self {
        Self {
            owner_handle: Handle::none(),
            min: 0.0,
            max: 100.0,
            value: 0.0,
            step: 1.0,
            orientation: Orientation::Horizontal,
            is_dragging: false,
            offset: Vec2::new(),
            value_changed: None,
        }
    }

    pub fn set_value(handle: &Handle<UINode>, ui: &mut UserInterface, value: f32) {
        let mut value_changed;
        let args;

        if let Some(node) = ui.nodes.borrow_mut(handle) {
            if let UINodeKind::ScrollBar(scroll_bar) = node.get_kind_mut() {
                let old_value = scroll_bar.value;
                let new_value = math::clampf(value, scroll_bar.min, scroll_bar.max);
                if new_value != old_value {
                    scroll_bar.value = new_value;
                    value_changed = scroll_bar.value_changed.take();
                    args = Some(ValueChangedArgs {
                        old_value,
                        new_value,
                        source: handle.clone(),
                    });
                } else {
                    return;
                }
            } else {
                return;
            }
        } else {
            return;
        }

        if let Some(ref mut handler) = value_changed {
            if let Some(args) = args {
                handler(ui, args)
            }
        }

        if let Some(node) = ui.nodes.borrow_mut(handle) {
            if let UINodeKind::ScrollBar(scroll_bar) = node.get_kind_mut() {
                scroll_bar.value_changed = value_changed;
            }
        }
    }

    pub fn set_max_value(handle: &Handle<UINode>, ui: &mut UserInterface, max: f32) {
        let mut new_value = None;
        if let Some(node) = ui.nodes.borrow_mut(handle) {
            if let UINodeKind::ScrollBar(scroll_bar) = node.get_kind_mut() {
                scroll_bar.max = max;
                if scroll_bar.max < scroll_bar.min {
                    std::mem::swap(&mut scroll_bar.min, &mut scroll_bar.max);
                }
                let old_value = scroll_bar.value;
                let clamped_new_value = math::clampf(scroll_bar.value, scroll_bar.min, scroll_bar.max);
                if clamped_new_value != old_value {
                    new_value = Some(clamped_new_value);
                }
            }
        }

        if let Some(new_value) = new_value {
            ScrollBar::set_value(handle, ui, new_value);
        }
    }

    pub fn set_min_value(handle: &Handle<UINode>, ui: &mut UserInterface, min: f32) {
        let mut new_value = None;
        if let Some(node) = ui.nodes.borrow_mut(handle) {
            if let UINodeKind::ScrollBar(scroll_bar) = node.get_kind_mut() {
                scroll_bar.min = min;
                if scroll_bar.min > scroll_bar.max {
                    std::mem::swap(&mut scroll_bar.min, &mut scroll_bar.max);
                }
                let old_value = scroll_bar.value;
                let clamped_new_value = math::clampf(scroll_bar.value, scroll_bar.min, scroll_bar.max);
                if clamped_new_value != old_value {
                    new_value = Some(clamped_new_value);
                }
            }
        }

        if let Some(new_value) = new_value {
            ScrollBar::set_value(handle, ui, new_value);
        }
    }
}

impl Layout for ScrollBar {
    fn measure_override(&self, ui: &UserInterface, available_size: Vec2) -> Vec2 {
        ui.default_measure_override(&self.owner_handle, available_size)
    }

    fn arrange_override(&self, ui: &UserInterface, final_size: Vec2) -> Vec2 {
        let size = ui.default_arrange_override(&self.owner_handle, final_size);


        // Adjust indicator position according to current value
        let percent = (self.value - self.min) / (self.max - self.min);

        let field_size = match ui.borrow_by_name_down(&self.owner_handle, Self::PART_CANVAS) {
            Some(canvas) => canvas.actual_size.get(),
            None => return size
        };

        if let Some(node) = ui.borrow_by_name_down(&self.owner_handle, Self::PART_INDICATOR) {
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
    value_changed: Option<Box<ValueChanged>>,
    step: Option<f32>,
    orientation: Option<Orientation>,
    common: CommonBuilderFields,
}

#[derive(Copy, Clone, Debug)]
pub enum Orientation {
    Vertical,
    Horizontal,
}

impl ScrollBarBuilder {
    pub fn new() -> Self {
        Self {
            min: None,
            max: None,
            value: None,
            step: None,
            value_changed: None,
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

    pub fn with_value_changed(mut self, value_changed: Box<ValueChanged>) -> Self {
        self.value_changed = Some(value_changed);
        self
    }

    pub fn build(self, ui: &mut UserInterface) -> Handle<UINode> {
        let mut scroll_bar = ScrollBar::new();
        if let Some(orientation) = self.orientation {
            scroll_bar.orientation = orientation;
        }
        scroll_bar.value_changed = self.value_changed;
        let orientation = scroll_bar.orientation;
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
                    .with_child(ButtonBuilder::new()
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
                        .with_click(Box::new(move |ui, handle| {
                            let scroll_bar_handle = ui.find_by_criteria_up(&handle, |node| match node.kind {
                                UINodeKind::ScrollBar(..) => true,
                                _ => false
                            });

                            let new_value = if let Some(scroll_bar_node) = ui.nodes.borrow_mut(&scroll_bar_handle) {
                                if let UINodeKind::ScrollBar(scroll_bar) = scroll_bar_node.get_kind_mut() {
                                    scroll_bar.value - scroll_bar.step
                                } else {
                                    return;
                                }
                            } else {
                                return;
                            };

                            ScrollBar::set_value(&scroll_bar_handle, ui, new_value);
                        }))
                        .build(ui)
                    )
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
                        .with_child(BorderBuilder::new()
                            .with_name(ScrollBar::PART_INDICATOR)
                            .with_stroke_color(Color::opaque(50, 50, 50))
                            .with_stroke_thickness(match orientation {
                                Orientation::Horizontal => Thickness { left: 1.0, top: 0.0, right: 1.0, bottom: 0.0 },
                                Orientation::Vertical => Thickness { left: 0.0, top: 1.0, right: 0.0, bottom: 1.0 }
                            })
                            .with_color(Color::opaque(255, 255, 255))
                            .with_width(30.0)
                            .with_height(30.0)
                            .with_handler(RoutedEventHandlerType::MouseDown, Box::new(move |ui, handle, evt| {
                                let indicator_pos = if let Some(node) = ui.nodes.borrow(&handle) {
                                    node.screen_position
                                } else {
                                    return;
                                };

                                if let RoutedEventKind::MouseDown { pos, .. } = evt.kind {
                                    if let Some(scroll_bar_node) = ui.borrow_by_criteria_up_mut(&handle, |node| match node.kind {
                                        UINodeKind::ScrollBar(..) => true,
                                        _ => false
                                    }) {
                                        if let UINodeKind::ScrollBar(scroll_bar) = scroll_bar_node.get_kind_mut() {
                                            scroll_bar.is_dragging = true;
                                            scroll_bar.offset = indicator_pos - pos;
                                        }
                                    }

                                    ui.capture_mouse(&handle);
                                    evt.handled = true;
                                }
                            }))
                            .with_handler(RoutedEventHandlerType::MouseUp, Box::new(move |ui, handle, evt| {
                                if let Some(scroll_bar_node) = ui.borrow_by_criteria_up_mut(&handle, |node| match node.kind {
                                    UINodeKind::ScrollBar(..) => true,
                                    _ => false
                                }) {
                                    if let UINodeKind::ScrollBar(scroll_bar) = scroll_bar_node.get_kind_mut() {
                                        scroll_bar.is_dragging = false;
                                    }
                                }
                                ui.release_mouse_capture();
                                evt.handled = true;
                            }))
                            .with_handler(RoutedEventHandlerType::MouseMove, Box::new(move |ui, handle, evt| {
                                let mouse_pos = match evt.kind {
                                    RoutedEventKind::MouseMove { pos } => pos,
                                    _ => return
                                };

                                let (field_pos, field_size) =
                                    match ui.borrow_by_name_up(&handle, ScrollBar::PART_CANVAS) {
                                        Some(canvas) => (canvas.screen_position, canvas.actual_size.get()),
                                        None => return
                                    };

                                let bar_size = match ui.nodes.borrow(&handle) {
                                    Some(node) => node.actual_size.get(),
                                    None => return
                                };

                                let new_value;

                                let scroll_bar_handle = ui.find_by_criteria_up(&handle, |node| match node.kind {
                                    UINodeKind::ScrollBar(..) => true,
                                    _ => false
                                });

                                if let Some(scroll_bar_node) = ui.nodes.borrow_mut(&scroll_bar_handle) {
                                    if let UINodeKind::ScrollBar(scroll_bar) = scroll_bar_node.get_kind_mut() {
                                        let orientation = scroll_bar.orientation;

                                        if scroll_bar.is_dragging {
                                            let percent = match orientation {
                                                Orientation::Horizontal => {
                                                    let span = field_size.x - bar_size.x;
                                                    let offset = mouse_pos.x - field_pos.x + scroll_bar.offset.x;
                                                    if span > 0.0 {
                                                        math::clampf(offset / span, 0.0, 1.0)
                                                    } else {
                                                        0.0
                                                    }
                                                }
                                                Orientation::Vertical => {
                                                    let span = field_size.y - bar_size.y;
                                                    let offset = mouse_pos.y - field_pos.y + scroll_bar.offset.y;
                                                    if span > 0.0 {
                                                        math::clampf(offset / span, 0.0, 1.0)
                                                    } else {
                                                        0.0
                                                    }
                                                }
                                            };

                                            new_value = percent * (scroll_bar.max - scroll_bar.min);

                                            evt.handled = true;
                                        } else {
                                            return;
                                        }
                                    } else {
                                        return;
                                    }
                                } else {
                                    return;
                                }

                                ScrollBar::set_value(&scroll_bar_handle, ui, new_value);
                            }))
                            .build(ui)
                        )
                        .build(ui)
                    )
                    .with_child(ButtonBuilder::new()
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
                        .with_click(Box::new(move |ui, handle| {
                            let scroll_bar_handle = ui.find_by_criteria_up(&handle, |node| match node.kind {
                                UINodeKind::ScrollBar(..) => true,
                                _ => false
                            });

                            let new_value = if let Some(scroll_bar_node) = ui.nodes.borrow_mut(&scroll_bar_handle) {
                                if let UINodeKind::ScrollBar(scroll_bar) = scroll_bar_node.get_kind_mut() {
                                    scroll_bar.value + scroll_bar.step
                                } else {
                                    return;
                                }
                            } else {
                                return;
                            };

                            ScrollBar::set_value(&scroll_bar_handle, ui, new_value);
                        }))
                        .with_text(match orientation {
                            Orientation::Horizontal => ">",
                            Orientation::Vertical => "v"
                        })
                        .build(ui)
                    )
                    .build(ui)
                )
                .build(ui)
            )
            .build(ui)
    }
}