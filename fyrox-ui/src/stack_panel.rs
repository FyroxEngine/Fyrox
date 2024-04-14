//! Stack panel orders its children widgets linearly; either top-to-bottom or left-to-right. See [`StackPanel`] docs for
//! more info and usage examples.

#![warn(missing_docs)]

use crate::{
    core::{
        algebra::Vector2, math::Rect, pool::Handle, reflect::prelude::*, scope_profile,
        type_traits::prelude::*, visitor::prelude::*,
    },
    define_constructor,
    message::{MessageDirection, UiMessage},
    widget::{Widget, WidgetBuilder},
    BuildContext, Control, Orientation, UiNode, UserInterface,
};
use fyrox_core::uuid_provider;
use fyrox_core::variable::InheritableVariable;
use fyrox_graph::BaseSceneGraph;
use std::ops::{Deref, DerefMut};

/// A set of possible [`StackPanel`] widget messages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StackPanelMessage {
    /// The message is used to change orientation of the stack panel.
    Orientation(Orientation),
}

impl StackPanelMessage {
    define_constructor!(
        /// Creates [`StackPanelMessage::Orientation`] message.
        StackPanelMessage:Orientation => fn orientation(Orientation), layout: false
    );
}

/// Stack Panels are one of several methods to position multiple widgets in relation to each other. A Stack Panel Widget
/// orders its children widgets linearly, aka in a stack of widgets, based on the order the widgets were added as children.
/// So the first widget added will be at the top or left most position, while each additional widget will descend from top to
/// bottom or continue from left most to right most. The below example code places 3 text widgets into a vertical stack:
///
/// ```rust,no_run
/// # use fyrox_ui::{
/// #     UiNode, core::pool::Handle,
/// #     BuildContext,
/// #     widget::WidgetBuilder,
/// #     text::TextBuilder,
/// #     stack_panel::StackPanelBuilder,
/// # };
/// fn create_stack_panel(ctx: &mut BuildContext) -> Handle<UiNode> {
///     StackPanelBuilder::new(
///         WidgetBuilder::new()
///             .with_child(
///                 TextBuilder::new(WidgetBuilder::new())
///                     .with_text("Top")
///                     .build(ctx)
///             )
///             .with_child(
///                 TextBuilder::new(WidgetBuilder::new())
///                     .with_text("Middle")
///                     .build(ctx)
///             )
///             .with_child(
///                 TextBuilder::new(WidgetBuilder::new())
///                     .with_text("Bottom")
///                     .build(ctx)
///             )
///     )
///         .build(ctx)
///     
/// }
/// ```
///
/// As you can see from the example, creating a Stack Panel uses the standard method for creating widgets. Create a new
/// [`StackPanelBuilder`] and provide it with a new [`WidgetBuilder`]. Adding widgets to the stack is done by adding children to
/// the StackBuilder's [`WidgetBuilder`].
///
/// ## Stack Panel Orientation
///
/// As has been indicated, stack panels can be oriented to order its children either Vertical (from top to bottom), or
/// Horizontal (left to most). This is done using the [`StackPanelBuilder::with_orientation`] function providing it
/// with a [`Orientation`] enum value. **By default** all stack panels has [`Orientation::Vertical`].
///
/// ```rust,no_run
/// # use fyrox_ui::{
/// #     Orientation,
/// #     BuildContext,
/// #     widget::WidgetBuilder,
/// #     stack_panel::StackPanelBuilder,
/// # };
///
/// # fn build(ctx: &mut BuildContext) {
/// StackPanelBuilder::new(
///     WidgetBuilder::new()
/// )
///     .with_orientation(Orientation::Horizontal)
///     .build(ctx);
/// # }
/// ```
#[derive(Default, Clone, Visit, Reflect, Debug, ComponentProvider)]
pub struct StackPanel {
    /// Base widget of the stack panel.
    pub widget: Widget,
    /// Current orientation of the stack panel.
    pub orientation: InheritableVariable<Orientation>,
}

crate::define_widget_deref!(StackPanel);

uuid_provider!(StackPanel = "d868f554-a2c5-4280-abfc-396d10a0e1ed");

impl Control for StackPanel {
    fn measure_override(&self, ui: &UserInterface, available_size: Vector2<f32>) -> Vector2<f32> {
        scope_profile!();

        let mut child_constraint = Vector2::new(f32::INFINITY, f32::INFINITY);

        match *self.orientation {
            Orientation::Vertical => {
                child_constraint.x = available_size.x;

                if !self.widget.width().is_nan() {
                    child_constraint.x = self.widget.width();
                }

                child_constraint.x = child_constraint.x.clamp(self.min_width(), self.max_width());
            }
            Orientation::Horizontal => {
                child_constraint.y = available_size.y;

                if !self.widget.height().is_nan() {
                    child_constraint.y = self.widget.height();
                }

                child_constraint.y = child_constraint
                    .y
                    .clamp(self.min_height(), self.max_height());
            }
        }

        let mut measured_size = Vector2::default();

        for child_handle in self.widget.children() {
            ui.measure_node(*child_handle, child_constraint);

            let child = ui.node(*child_handle);
            let desired = child.desired_size();
            match *self.orientation {
                Orientation::Vertical => {
                    if desired.x > measured_size.x {
                        measured_size.x = desired.x;
                    }
                    measured_size.y += desired.y;
                }
                Orientation::Horizontal => {
                    measured_size.x += desired.x;
                    if desired.y > measured_size.y {
                        measured_size.y = desired.y;
                    }
                }
            }
        }

        measured_size
    }

    fn arrange_override(&self, ui: &UserInterface, final_size: Vector2<f32>) -> Vector2<f32> {
        scope_profile!();

        let mut width = final_size.x;
        let mut height = final_size.y;

        match *self.orientation {
            Orientation::Vertical => height = 0.0,
            Orientation::Horizontal => width = 0.0,
        }

        for child_handle in self.widget.children() {
            let child = ui.node(*child_handle);
            match *self.orientation {
                Orientation::Vertical => {
                    let child_bounds = Rect::new(
                        0.0,
                        height,
                        width.max(child.desired_size().x),
                        child.desired_size().y,
                    );
                    ui.arrange_node(*child_handle, &child_bounds);
                    width = width.max(child.desired_size().x);
                    height += child.desired_size().y;
                }
                Orientation::Horizontal => {
                    let child_bounds = Rect::new(
                        width,
                        0.0,
                        child.desired_size().x,
                        height.max(child.desired_size().y),
                    );
                    ui.arrange_node(*child_handle, &child_bounds);
                    width += child.desired_size().x;
                    height = height.max(child.desired_size().y);
                }
            }
        }

        match *self.orientation {
            Orientation::Vertical => {
                height = height.max(final_size.y);
            }
            Orientation::Horizontal => {
                width = width.max(final_size.x);
            }
        }

        Vector2::new(width, height)
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);

        if message.destination() == self.handle && message.direction() == MessageDirection::ToWidget
        {
            if let Some(StackPanelMessage::Orientation(orientation)) = message.data() {
                if *orientation != *self.orientation {
                    self.orientation.set_value_and_mark_modified(*orientation);
                    self.invalidate_layout();
                }
            }
        }
    }
}

/// Stack panel builders creates [`StackPanel`] widgets and registers them in the user interface.
pub struct StackPanelBuilder {
    widget_builder: WidgetBuilder,
    orientation: Option<Orientation>,
}

impl StackPanelBuilder {
    /// Creates new stack panel builder with the base widget builder.
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self {
            widget_builder,
            orientation: None,
        }
    }

    /// Sets the desired orientation of the stack panel.
    pub fn with_orientation(mut self, orientation: Orientation) -> Self {
        self.orientation = Some(orientation);
        self
    }

    /// Finishes stack panel building.
    pub fn build_stack_panel(self) -> StackPanel {
        StackPanel {
            widget: self.widget_builder.build(),
            orientation: self.orientation.unwrap_or(Orientation::Vertical).into(),
        }
    }

    /// Finishes stack panel building and wraps the result in a UI node.
    pub fn build_node(self) -> UiNode {
        UiNode::new(self.build_stack_panel())
    }

    /// Finishes stack panel building and adds the new stack panel widget instance to the user interface and
    /// returns its handle.
    pub fn build(self, ctx: &mut BuildContext) -> Handle<UiNode> {
        ctx.add_node(self.build_node())
    }
}
