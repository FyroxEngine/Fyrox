#![allow(clippy::reversed_empty_ranges)]

use crate::{
    core::{algebra::Vector2, math::Rect, pool::Handle},
    message::UiMessage,
    widget::{Widget, WidgetBuilder},
    BuildContext, Control, Orientation, UiNode, UserInterface,
};
use std::{
    any::{Any, TypeId},
    cell::RefCell,
    ops::{Deref, DerefMut, Range},
};

#[derive(Clone)]
pub struct WrapPanel {
    widget: Widget,
    orientation: Orientation,
    lines: RefCell<Vec<Line>>,
}

crate::define_widget_deref!(WrapPanel);

impl WrapPanel {
    pub fn new(widget: Widget) -> Self {
        Self {
            widget,
            orientation: Orientation::Vertical,
            lines: Default::default(),
        }
    }

    pub fn set_orientation(&mut self, orientation: Orientation) {
        if self.orientation != orientation {
            self.orientation = orientation;
            self.widget.invalidate_layout();
        }
    }

    pub fn orientation(&self) -> Orientation {
        self.orientation
    }
}

#[derive(Clone)]
struct Line {
    children: Range<usize>,
    bounds: Rect<f32>,
}

impl Default for Line {
    fn default() -> Self {
        Self {
            children: 0..0,
            bounds: Default::default(),
        }
    }
}

impl Control for WrapPanel {
    fn query_component(&self, type_id: TypeId) -> Option<&dyn Any> {
        if type_id == TypeId::of::<Self>() {
            Some(self)
        } else {
            None
        }
    }

    fn measure_override(&self, ui: &UserInterface, available_size: Vector2<f32>) -> Vector2<f32> {
        let mut measured_size: Vector2<f32> = Vector2::default();
        let mut line_size = Vector2::default();
        for child_handle in self.widget.children() {
            let child = ui.node(*child_handle);
            ui.measure_node(*child_handle, available_size);
            let desired = child.desired_size();
            match self.orientation {
                Orientation::Vertical => {
                    if line_size.y + desired.y > available_size.y {
                        // Commit column.
                        measured_size.y = measured_size.y.max(line_size.y);
                        measured_size.x += line_size.x;
                        line_size = Vector2::default();
                    }
                    line_size.x = line_size.x.max(desired.x);
                    line_size.y += desired.y;
                }
                Orientation::Horizontal => {
                    if line_size.x + desired.x > available_size.x {
                        // Commit row.
                        measured_size.x = measured_size.x.max(line_size.x);
                        measured_size.y += line_size.y;
                        line_size = Vector2::default();
                    }
                    line_size.x += desired.x;
                    line_size.y = line_size.y.max(desired.y);
                }
            }
        }

        // Commit rest.
        match self.orientation {
            Orientation::Vertical => {
                measured_size.y = measured_size.y.max(line_size.y);
                measured_size.x += line_size.x;
            }
            Orientation::Horizontal => {
                measured_size.x = measured_size.x.max(line_size.x);
                measured_size.y += line_size.y;
            }
        }

        measured_size
    }

    fn arrange_override(&self, ui: &UserInterface, final_size: Vector2<f32>) -> Vector2<f32> {
        // First pass - arrange lines.
        let mut lines = self.lines.borrow_mut();
        lines.clear();
        let mut line = Line::default();
        for child_handle in self.widget.children() {
            let child = ui.node(*child_handle);
            let desired = child.desired_size();
            match self.orientation {
                Orientation::Vertical => {
                    if line.bounds.h() + desired.y > final_size.y {
                        // Commit column.
                        lines.push(line.clone());
                        // Advance column.
                        line.bounds.position.x += line.bounds.w();
                        line.bounds.position.y = 0.0;
                        line.bounds.size.x = desired.x;
                        line.bounds.size.y = desired.y;
                        // Reset children.
                        line.children.start = line.children.end;
                        line.children.end = line.children.start + 1;
                    } else {
                        line.bounds.size.y += desired.y;
                        line.bounds.size.x = line.bounds.w().max(desired.x);
                        line.children.end += 1;
                    }
                }
                Orientation::Horizontal => {
                    if line.bounds.w() + desired.x > final_size.x {
                        // Commit row.
                        lines.push(line.clone());
                        // Advance row.
                        line.bounds.position.x = 0.0;
                        line.bounds.position.y += line.bounds.h();
                        line.bounds.size.x = desired.x;
                        line.bounds.size.y = desired.y;
                        // Reset children.
                        line.children.start = line.children.end;
                        line.children.end = line.children.start + 1;
                    } else {
                        line.bounds.size.x += desired.x;
                        line.bounds.size.y = line.bounds.h().max(desired.y);
                        line.children.end += 1;
                    }
                }
            }
        }

        // Commit rest.
        lines.push(line);

        // Second pass - arrange children of lines.
        let mut full_size = Vector2::default();
        for line in lines.iter() {
            let mut cursor = line.bounds.position;
            for child_index in line.children.clone() {
                let child_handle = self.children()[child_index];
                let child = ui.node(child_handle);
                let desired = child.desired_size();
                match self.orientation {
                    Orientation::Vertical => {
                        let child_bounds =
                            Rect::new(line.bounds.x(), cursor.y, line.bounds.w(), desired.y);
                        ui.arrange_node(child_handle, &child_bounds);
                        cursor.y += desired.y;
                    }
                    Orientation::Horizontal => {
                        let child_bounds =
                            Rect::new(cursor.x, line.bounds.y(), desired.x, line.bounds.h());
                        ui.arrange_node(child_handle, &child_bounds);
                        cursor.x += desired.x;
                    }
                }
            }
            match self.orientation {
                Orientation::Vertical => {
                    full_size.x += line.bounds.w();
                    full_size.y = final_size.y.max(line.bounds.h());
                }
                Orientation::Horizontal => {
                    full_size.x = final_size.x.max(line.bounds.w());
                    full_size.y += line.bounds.h();
                }
            }
        }

        full_size
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);
    }
}

pub struct WrapPanelBuilder {
    widget_builder: WidgetBuilder,
    orientation: Option<Orientation>,
}

impl WrapPanelBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            orientation: None,
        }
    }

    pub fn with_orientation(mut self, orientation: Orientation) -> Self {
        self.orientation = Some(orientation);
        self
    }

    pub fn build_node(self) -> UiNode {
        let stack_panel = WrapPanel {
            widget: self.widget_builder.build(),
            orientation: self.orientation.unwrap_or(Orientation::Vertical),
            lines: Default::default(),
        };

        UiNode::new(stack_panel)
    }

    pub fn build(self, ui: &mut BuildContext) -> Handle<UiNode> {
        ui.add_node(self.build_node())
    }
}
