use crate::{
    core::{
        pool::Handle,
        math::{
            vec2::Vec2,
            Rect,
        },
    },
    UINode,
    widget::{
        Widget,
        WidgetBuilder,
    },
    UserInterface,
    Control,
    message::UiMessage
};

/// Allows user to directly set position and size of a node
pub struct Canvas<M: 'static, C: 'static + Control<M, C>> {
    widget: Widget<M, C>,
}

impl<M, C: 'static + Control<M, C>> Control<M, C> for Canvas<M, C> {
    fn widget(&self) -> &Widget<M, C> {
        &self.widget
    }

    fn widget_mut(&mut self) -> &mut Widget<M, C> {
        &mut self.widget
    }

    fn raw_copy(&self) -> UINode<M, C> {
        UINode::Canvas(Self {
            widget: self.widget.raw_copy()
        })
    }

    fn measure_override(&self, ui: &UserInterface<M, C>, _available_size: Vec2) -> Vec2 {
        let size_for_child = Vec2::new(
            std::f32::INFINITY,
            std::f32::INFINITY,
        );

        for child_handle in self.widget.children() {
            ui.node(*child_handle).measure(ui, size_for_child);
        }

        Vec2::ZERO
    }

    fn arrange_override(&self, ui: &UserInterface<M, C>, final_size: Vec2) -> Vec2 {
        for child_handle in self.widget.children() {
            let child = ui.nodes.borrow(*child_handle);
            child.arrange(ui, &Rect::new(
                child.widget().desired_local_position().x,
                child.widget().desired_local_position().y,
                child.widget().desired_size().x,
                child.widget().desired_size().y));
        }

        final_size
    }

    fn handle_message(&mut self, self_handle: Handle<UINode<M, C>>, ui: &mut UserInterface<M, C>, message: &mut UiMessage<M, C>) {
        self.widget.handle_message(self_handle, ui, message);
    }
}

impl<M, C: 'static + Control<M, C>> Canvas<M, C> {
    pub fn new(widget: Widget<M, C>) -> Self {
        Self {
            widget
        }
    }
}

pub struct CanvasBuilder<M: 'static, C: 'static + Control<M, C>> {
    widget_builder: WidgetBuilder<M, C>,
}

impl<M, C: 'static + Control<M, C>> CanvasBuilder<M, C> {
    pub fn new(widget_builder: WidgetBuilder<M, C>) -> Self {
        Self {
            widget_builder,
        }
    }

    pub fn build(self, ui: &mut UserInterface<M, C>) -> Handle<UINode<M, C>> {
        let handle = ui.add_node(UINode::Canvas(Canvas {
            widget: self.widget_builder.build()
        }));

        ui.flush_messages();

        handle
    }
}