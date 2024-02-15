//! Wrap panel is used to stack children widgets either in vertical or horizontal direction with overflow. See [`WrapPanel`]
//! docs for more info and usage examples.

#![warn(missing_docs)]
#![allow(clippy::reversed_empty_ranges)]

use crate::{
    core::{
        algebra::Vector2, math::Rect, pool::Handle, reflect::prelude::*, type_traits::prelude::*,
        visitor::prelude::*,
    },
    define_constructor,
    message::{MessageDirection, UiMessage},
    widget::{Widget, WidgetBuilder},
    BuildContext, Control, Orientation, UiNode, UserInterface,
};
use fyrox_core::uuid_provider;
use fyrox_core::variable::InheritableVariable;
use fyrox_graph::BaseSceneGraph;
use std::{
    cell::RefCell,
    ops::{Deref, DerefMut, Range},
};

/// A set of possible [`WrapPanel`] widget messages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WrapPanelMessage {
    /// The message is used to change orientation of the wrap panel.
    Orientation(Orientation),
}

impl WrapPanelMessage {
    define_constructor!(
        /// Creates [`WrapPanelMessage::Orientation`] message.
        WrapPanelMessage:Orientation => fn orientation(Orientation), layout: false
    );
}

/// Wrap panel is used to stack children widgets either in vertical or horizontal direction with overflow - every widget
/// that does not have enough space on current line, will automatically be placed on the next line (either vertical or
/// horizontal, depending on the orientation).
///
/// ## How to create
///
/// Use `WrapPanelBuilder` to create new wrap panel instance:
///
/// ```rust,no_run
/// # use fyrox_ui::{
/// #     core::pool::Handle,
/// #     widget::WidgetBuilder, wrap_panel::WrapPanelBuilder, BuildContext, Orientation, UiNode,
/// # };
/// #
/// fn create_wrap_panel(ctx: &mut BuildContext) -> Handle<UiNode> {
///     WrapPanelBuilder::new(WidgetBuilder::new())
///         .with_orientation(Orientation::Horizontal)
///         .build(ctx)
/// }
/// ```
///
/// All widgets, that needs to be arranged, should be direct children of the wrap panel. Use [`WidgetBuilder::with_children`]
/// or [`WidgetBuilder::with_child`] to add children nodes.
///
/// ## Orientation
///
/// Wrap panel can stack your widgets either in vertical or horizontal direction. Use `.with_orientation` while building
/// the panel to switch orientation to desired.
#[derive(Default, Clone, Debug, Visit, Reflect, ComponentProvider)]
pub struct WrapPanel {
    /// Base widget of the wrap panel.
    pub widget: Widget,
    /// Current orientation of the wrap panel.
    pub orientation: InheritableVariable<Orientation>,
    /// Internal lines storage.
    #[visit(skip)]
    #[reflect(hidden)]
    pub lines: RefCell<Vec<Line>>,
}

crate::define_widget_deref!(WrapPanel);

/// Represents a single line (either vertical or horizontal) with arranged widgets.
#[derive(Clone, Debug)]
pub struct Line {
    /// Indices of the children widgets that belongs to this line.
    pub children: Range<usize>,
    /// Bounds of this line.
    pub bounds: Rect<f32>,
}

impl Default for Line {
    fn default() -> Self {
        Self {
            children: 0..0,
            bounds: Default::default(),
        }
    }
}

uuid_provider!(WrapPanel = "f488ab8e-8f8b-473c-a450-5ac33f1afb39");

impl Control for WrapPanel {
    fn measure_override(&self, ui: &UserInterface, available_size: Vector2<f32>) -> Vector2<f32> {
        let mut measured_size: Vector2<f32> = Vector2::default();
        let mut line_size = Vector2::default();
        for child_handle in self.widget.children() {
            let child = ui.node(*child_handle);
            ui.measure_node(*child_handle, available_size);
            let desired = child.desired_size();
            match *self.orientation {
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
        match *self.orientation {
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
            match *self.orientation {
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
                match *self.orientation {
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
            match *self.orientation {
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

        if message.destination() == self.handle && message.direction() == MessageDirection::ToWidget
        {
            if let Some(WrapPanelMessage::Orientation(orientation)) = message.data() {
                if *orientation != *self.orientation {
                    self.orientation.set_value_and_mark_modified(*orientation);
                    self.invalidate_layout();
                }
            }
        }
    }
}

/// Wrap panel builder creates [`WrapPanel`] widget and adds it to the user interface.
pub struct WrapPanelBuilder {
    widget_builder: WidgetBuilder,
    orientation: Option<Orientation>,
}

impl WrapPanelBuilder {
    /// Creates a new wrap panel builder.
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            orientation: None,
        }
    }

    /// Sets the desired orientation of the wrap panel.
    pub fn with_orientation(mut self, orientation: Orientation) -> Self {
        self.orientation = Some(orientation);
        self
    }

    /// Finishes wrap panel building and returns its instance.
    pub fn build_node(self) -> UiNode {
        let stack_panel = WrapPanel {
            widget: self.widget_builder.build(),
            orientation: self.orientation.unwrap_or(Orientation::Vertical).into(),
            lines: Default::default(),
        };

        UiNode::new(stack_panel)
    }

    /// Finishes wrap panel building, adds it to the user interface and returns its handle.
    pub fn build(self, ui: &mut BuildContext) -> Handle<UiNode> {
        ui.add_node(self.build_node())
    }
}
