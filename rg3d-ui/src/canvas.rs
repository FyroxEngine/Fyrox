use crate::{
    core::{algebra::Vector2, math::Rect, pool::Handle, scope_profile},
    message::UiMessage,
    widget::{Widget, WidgetBuilder},
    BuildContext, Control, UiNode, UserInterface,
};
use std::{
    any::{Any, TypeId},
    ops::{Deref, DerefMut},
};

/// Allows user to directly set position and size of a node
#[derive(Clone)]
pub struct Canvas {
    widget: Widget,
}

crate::define_widget_deref!(Canvas);

impl Control for Canvas {
    fn query_component(&self, type_id: TypeId) -> Option<&dyn Any> {
        if type_id == TypeId::of::<Self>() {
            Some(self)
        } else {
            None
        }
    }

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

impl Canvas {
    pub fn new(widget: Widget) -> Self {
        Self { widget }
    }
}

pub struct CanvasBuilder {
    widget_builder: WidgetBuilder,
}

impl CanvasBuilder {
    pub fn new(widget_builder: WidgetBuilder) -> Self {
        Self { widget_builder }
    }

    pub fn build(self, ui: &mut BuildContext) -> Handle<UiNode> {
        let canvas = Canvas {
            widget: self.widget_builder.build(),
        };
        ui.add_node(UiNode::new(canvas))
    }
}
