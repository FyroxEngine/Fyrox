use crate::{
    core::{
        math::{vec2::Vec2, Rect},
        pool::Handle,
        scope_profile,
    },
    message::UiMessage,
    widget::{Widget, WidgetBuilder},
    BuildContext, Control, UINode, UserInterface,
};
use std::ops::{Deref, DerefMut};

/// Allows user to directly set position and size of a node
#[derive(Clone)]
pub struct Canvas<M: 'static + std::fmt::Debug + Clone, C: 'static + Control<M, C>> {
    widget: Widget<M, C>,
}

impl<M: 'static + std::fmt::Debug + Clone, C: 'static + Control<M, C>> Deref for Canvas<M, C> {
    type Target = Widget<M, C>;

    fn deref(&self) -> &Self::Target {
        &self.widget
    }
}

impl<M: 'static + std::fmt::Debug + Clone, C: 'static + Control<M, C>> DerefMut for Canvas<M, C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.widget
    }
}

impl<M: 'static + std::fmt::Debug + Clone, C: 'static + Control<M, C>> Control<M, C>
    for Canvas<M, C>
{
    fn measure_override(&self, ui: &UserInterface<M, C>, _available_size: Vec2) -> Vec2 {
        scope_profile!();

        let size_for_child = Vec2::new(std::f32::INFINITY, std::f32::INFINITY);

        for child_handle in self.widget.children() {
            ui.node(*child_handle).measure(ui, size_for_child);
        }

        Vec2::ZERO
    }

    fn arrange_override(&self, ui: &UserInterface<M, C>, final_size: Vec2) -> Vec2 {
        scope_profile!();

        for child_handle in self.widget.children() {
            let child = ui.nodes.borrow(*child_handle);
            child.arrange(
                ui,
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

    fn handle_routed_message(
        &mut self,
        ui: &mut UserInterface<M, C>,
        message: &mut UiMessage<M, C>,
    ) {
        self.widget.handle_routed_message(ui, message);
    }
}

impl<M: 'static + std::fmt::Debug + Clone, C: 'static + Control<M, C>> Canvas<M, C> {
    pub fn new(widget: Widget<M, C>) -> Self {
        Self { widget }
    }
}

pub struct CanvasBuilder<M: 'static + std::fmt::Debug + Clone, C: 'static + Control<M, C>> {
    widget_builder: WidgetBuilder<M, C>,
}

impl<M: 'static + std::fmt::Debug + Clone, C: 'static + Control<M, C>> CanvasBuilder<M, C> {
    pub fn new(widget_builder: WidgetBuilder<M, C>) -> Self {
        Self { widget_builder }
    }

    pub fn build(self, ui: &mut BuildContext<M, C>) -> Handle<UINode<M, C>> {
        let canvas = Canvas {
            widget: self.widget_builder.build(),
        };
        ui.add_node(UINode::Canvas(canvas))
    }
}
