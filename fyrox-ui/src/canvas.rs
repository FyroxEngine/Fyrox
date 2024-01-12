//! Canvas widget allows its children to have an arbitrary position on an imaginable infinite plane, it also
//! gives the children constraints of infinite size, which forces them to take all the desired size. See
//! [`Canvas`] docs for more info and usage examples.

#![warn(missing_docs)]

use crate::{
    core::{
        algebra::Vector2, math::Rect, pool::Handle, reflect::prelude::*, scope_profile,
        type_traits::prelude::*, visitor::prelude::*,
    },
    message::UiMessage,
    widget::{Widget, WidgetBuilder},
    BuildContext, Control, UiNode, UserInterface,
};
use std::ops::{Deref, DerefMut};

/// Canvas widget allows its children to have an arbitrary position on an imaginable infinite plane, it also
/// gives the children constraints of infinite size, which forces them to take all the desired size. This widget
/// could be used when you need to have an ability to put widgets at arbitrary positions. Canvas widget is the
/// root widget of the widget hierarchy used in `fyrox-ui`.
///
/// ## Examples
///
/// A instance of [`Canvas`] widget can be created using [`CanvasBuilder`] with a set of children widgets provided
/// to [`WidgetBuilder`]:
///
/// ```rust
/// # use fyrox_ui::{
/// #     button::ButtonBuilder, canvas::CanvasBuilder, core::pool::Handle, text::TextBuilder,
/// #     widget::WidgetBuilder, BuildContext, UiNode,
/// # };
/// #
/// fn create_canvas(ctx: &mut BuildContext) -> Handle<UiNode> {
///     CanvasBuilder::new(
///         WidgetBuilder::new()
///             .with_child(
///                 ButtonBuilder::new(WidgetBuilder::new())
///                     .with_text("Click me!")
///                     .build(ctx),
///             )
///             .with_child(
///                 TextBuilder::new(WidgetBuilder::new())
///                     .with_text("Some text")
///                     .build(ctx),
///             ),
///     )
///     .build(ctx)
/// }
/// ```
#[derive(Default, Clone, Visit, Reflect, Debug, TypeUuidProvider, ComponentProvider)]
#[type_uuid(id = "6b843a36-53da-467b-b85e-2380fe891ca1")]
pub struct Canvas {
    /// Base widget of the canvas.
    pub widget: Widget,
}

crate::define_widget_deref!(Canvas);

impl Control for Canvas {
    fn measure_override(&self, ui: &UserInterface, _available_size: Vector2<f32>) -> Vector2<f32> {
        scope_profile!();

        let size_for_child = Vector2::new(f32::INFINITY, f32::INFINITY);

        for child_handle in self.widget.children() {
            ui.measure_node(*child_handle, size_for_child);
        }

        Vector2::default()
    }

    fn arrange_override(&self, ui: &UserInterface, final_size: Vector2<f32>) -> Vector2<f32> {
        scope_profile!();

        for &child_handle in self.widget.children() {
            let child = ui.nodes.borrow(child_handle);
            ui.arrange_node(
                child_handle,
                &Rect::new(
                    child.desired_local_position().x,
                    child.desired_local_position().y,
                    child.desired_size().x,
                    child.desired_size().y,
                ),
            );
        }

        final_size
    }

    fn handle_routed_message(&mut self, ui: &mut UserInterface, message: &mut UiMessage) {
        self.widget.handle_routed_message(ui, message);
    }
}

/// Canvas builder creates new [`Canvas`] widget instances and adds them to the user interface.
pub struct CanvasBuilder {
    widget_builder: WidgetBuilder,
}

impl CanvasBuilder {
    /// Creates new builder instance.
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self { widget_builder }
    }

    /// Finishes canvas widget building and adds the instance to the user interface and returns its handle.
    pub fn build(self, ui: &mut BuildContext) -> Handle<UiNode> {
        let canvas = Canvas {
            widget: self.widget_builder.build(),
        };
        ui.add_node(UiNode::new(canvas))
    }
}
